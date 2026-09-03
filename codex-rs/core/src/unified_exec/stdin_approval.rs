//! Проверяет ввод терминала по фактическим полномочиям запуска.
//!
//! Снимок полномочий принадлежит хосту: последующее изменение конфигурации не
//! меняет sandbox уже запущенного процесса и не считается новым ограничением для
//! него без отдельной проверки.

use super::ProcessEntry;
use super::UnifiedExecContext;
use super::UnifiedExecError;
use crate::config::NetworkProxySpec;
use crate::session::turn_context::TurnContext;
use crate::session::turn_context::TurnEnvironment;
use crate::tools::sandboxing::ApprovalAction;
use crate::tools::sandboxing::ToolError;
use codex_file_system::ExecPermissionProfile;
use codex_file_system::FileSystemSandboxContext;
use codex_network_proxy::EnvironmentNetworkPolicy;
use codex_otel::SessionTelemetry;
use codex_protocol::models::AdditionalPermissionProfile;
use codex_protocol::models::PermissionProfile;
use codex_protocol::models::SandboxPermissions;
use codex_protocol::permissions::NetworkSandboxPolicy;
use codex_sandboxing::policy_transforms::effective_permission_profile;
use codex_sandboxing::policy_transforms::merge_permission_profiles;

const MAX_TERMINAL_REVIEW_BYTES: usize = 8_000;

#[derive(Clone, Copy)]
pub(crate) enum TerminalSandboxSource {
    Native,
    Executor,
}

pub(crate) struct TerminalPermissions {
    policy: TerminalPolicy,
    sandbox_source: TerminalSandboxSource,
    launch_permissions: SandboxPermissions,
    additional_permissions: Option<AdditionalPermissionProfile>,
    internal_permissions: Option<AdditionalPermissionProfile>,
}

pub(crate) struct TerminalInputApproval {
    pub(crate) sandbox_permissions: SandboxPermissions,
    pub(crate) additional_permissions: Option<AdditionalPermissionProfile>,
    pub(crate) reason: String,
}

/// Сохранённые настройки запуска, которые нельзя сериализовать в approval.
#[derive(PartialEq, Eq)]
struct TerminalPolicy {
    sandbox: FileSystemSandboxContext,
    environment_network: Option<EnvironmentNetworkPolicy>,
    controller_network: Option<NetworkProxySpec>,
    controller_proxy: bool,
}

impl TerminalPolicy {
    fn capture(
        environment: &TurnEnvironment,
        turn: &TurnContext,
        sandbox_source: TerminalSandboxSource,
        additional_permissions: Option<AdditionalPermissionProfile>,
    ) -> Self {
        let mut sandbox = environment.sandbox_context(additional_permissions);
        if matches!(sandbox_source, TerminalSandboxSource::Native) {
            // Встроенный helper применяет значения executor по умолчанию, тогда как
            // локальный запуск обязан сохранить явный уровень Windows sandbox.
            sandbox.windows_sandbox_level = environment.config().windows_sandbox_level;
        }
        Self {
            sandbox,
            environment_network: environment.config().network_policy.clone(),
            controller_network: turn.config.permissions.network.clone(),
            controller_proxy: turn.network.is_some(),
        }
    }

    fn file_system_context(&self) -> FileSystemSandboxContext {
        let mut context = self.sandbox.clone();
        // Изменение сети требует review, но не считается несовместимостью denied reads.
        if let ExecPermissionProfile::Managed { network, .. } = &mut context.permissions {
            *network = NetworkSandboxPolicy::Restricted;
        }
        context
    }
}

impl TerminalPermissions {
    pub(crate) fn for_launch(
        environment: &TurnEnvironment,
        turn: &TurnContext,
        sandbox_source: TerminalSandboxSource,
        launch_permissions: SandboxPermissions,
        additional_permissions: Option<&AdditionalPermissionProfile>,
        internal_permissions: Option<&AdditionalPermissionProfile>,
    ) -> Self {
        Self {
            policy: TerminalPolicy::capture(
                environment,
                turn,
                sandbox_source,
                merge_permission_profiles(additional_permissions, internal_permissions),
            ),
            sandbox_source,
            launch_permissions,
            additional_permissions: additional_permissions.cloned(),
            internal_permissions: internal_permissions.cloned(),
        }
    }

    pub(crate) fn initial_stdin_approval(
        &self,
        input: Option<&str>,
        command_was_reviewed: bool,
        reviewed_sandbox_permissions: SandboxPermissions,
        reviewed_additional_permissions: Option<&AdditionalPermissionProfile>,
    ) -> Result<Option<TerminalInputApproval>, serde_json::Error> {
        let Some(_) = input.filter(|input| !input.is_empty()) else {
            return Ok(None);
        };
        let sandbox_permissions = self.launch_approval_permissions();
        let prior_review_covers_launch = command_was_reviewed
            && self.internal_permissions.is_none()
            && sandbox_permissions == reviewed_sandbox_permissions
            && self.additional_permissions.as_ref() == reviewed_additional_permissions;
        if prior_review_covers_launch
            || (!command_was_reviewed && sandbox_permissions == SandboxPermissions::UseDefault)
        {
            return Ok(None);
        }

        let reason = self.approval_reason_for_operation(
            sandbox_permissions,
            "Send initial input while launching this command.",
        )?;
        Ok(Some(TerminalInputApproval {
            sandbox_permissions,
            additional_permissions: self.additional_permissions.clone(),
            reason,
        }))
    }

    fn launch_approval_permissions(&self) -> SandboxPermissions {
        if self.launch_permissions.requires_escalated_permissions() {
            SandboxPermissions::RequireEscalated
        } else if self.additional_permissions.is_some() || self.internal_permissions.is_some() {
            SandboxPermissions::WithAdditionalPermissions
        } else {
            SandboxPermissions::UseDefault
        }
    }

    /// Сравнивает полномочия запуска с текущей политикой, не обращаясь к процессу.
    fn review_requirement(
        &self,
        current: &TerminalPolicy,
        baseline: &PermissionProfile,
    ) -> Result<SandboxPermissions, &'static str> {
        let bypassed = self.launch_permissions.requires_escalated_permissions();
        if current.environment_network.is_some()
            && (bypassed || self.policy.environment_network != current.environment_network)
        {
            return Err(
                "this terminal cannot enforce the current environment-owned network restrictions; start a new terminal",
            );
        }
        if baseline
            .file_system_sandbox_policy()
            .has_denied_read_restrictions()
            && (bypassed || self.policy.file_system_context() != current.file_system_context())
        {
            return Err(
                "this terminal cannot enforce the current denied-read restrictions; start a new terminal",
            );
        }
        Ok(if bypassed || &self.policy != current {
            SandboxPermissions::RequireEscalated
        } else if self.policy.sandbox.permissions
            == effective_permission_profile(baseline, /*additional_permissions*/ None).into()
        {
            SandboxPermissions::UseDefault
        } else {
            SandboxPermissions::WithAdditionalPermissions
        })
    }

    fn approval_reason(
        &self,
        sandbox_permissions: SandboxPermissions,
    ) -> Result<String, serde_json::Error> {
        self.approval_reason_for_operation(
            sandbox_permissions,
            "Send input to an existing terminal.",
        )
    }

    fn approval_reason_for_operation(
        &self,
        sandbox_permissions: SandboxPermissions,
        operation: &str,
    ) -> Result<String, serde_json::Error> {
        let authority = if self.launch_permissions.requires_escalated_permissions() {
            "This terminal was launched outside the sandbox, bypassing any managed network proxy."
        } else if self.policy.sandbox.permissions == ExecPermissionProfile::Disabled {
            "This terminal runs without a filesystem sandbox."
        } else {
            match sandbox_permissions {
                SandboxPermissions::UseDefault => "This terminal uses the current permissions.",
                SandboxPermissions::WithAdditionalPermissions => {
                    "This terminal retains additional permissions."
                }
                SandboxPermissions::RequireEscalated => {
                    "This terminal retains sandbox or network settings that differ from the current permissions."
                }
            }
        };
        let mut reason = format!("{operation} {authority}");
        if self.internal_permissions.is_some() {
            reason.push_str(" It also has an internal plugin metrics write grant.");
        }
        reason.push_str(" The cwd is its launch directory; the terminal's current directory and state may have changed.");
        if let Some(grants) = &self.additional_permissions {
            // Stable reason text also reaches clients that strip the experimental
            // additionalPermissions field. Internal paths never enter this text.
            reason.push_str(&format!(
                " Retained grants: {}.",
                serde_json::to_string(grants)?
            ));
        }
        Ok(reason)
    }
}

impl ProcessEntry {
    pub(super) fn stdin_approval(
        &self,
        context: &UnifiedExecContext,
        input: &str,
        strict_auto_review: bool,
    ) -> Result<Option<(ApprovalAction, String)>, UnifiedExecError> {
        if input.is_empty() || (!self.tty && input == "\u{3}") {
            return Ok(None);
        }
        let environment = context
            .step_context
            .environments
            .turn_environments()
            .find(|environment| environment.selection.environment_id == self.environment_id)
            .ok_or_else(|| {
                approval_error(
                    "cannot access the terminal's original environment; select it before retrying",
                )
            })?;
        let permissions = &self.permissions;
        let current = TerminalPolicy::capture(
            environment,
            &context.step_context.turn,
            permissions.sandbox_source,
            merge_permission_profiles(
                permissions.additional_permissions.as_ref(),
                permissions.internal_permissions.as_ref(),
            ),
        );
        let sandbox_permissions = permissions
            .review_requirement(&current, environment.permission_profile())
            .map_err(approval_error)?;
        if sandbox_permissions == SandboxPermissions::UseDefault && !strict_auto_review {
            return Ok(None);
        }
        let reason = permissions
            .approval_reason(sandbox_permissions)
            .map_err(approval_error)?;
        let action = ApprovalAction::WriteStdin {
            id: self.call_id.clone(),
            approval_id: context.call_id.clone(),
            environment_id: self.environment_id.clone(),
            process_id: self.process_id,
            input: input.to_string(),
            cwd: self.cwd.clone(),
            tty: self.tty,
            sandbox_permissions,
            additional_permissions: permissions.additional_permissions.clone(),
        };
        Ok(Some((action, reason)))
    }
}

pub(crate) fn validate_terminal_input_review(
    action: &ApprovalAction,
    approval_reason: Option<&str>,
    retry_reason: Option<&str>,
    telemetry: &SessionTelemetry,
) -> Result<(), ToolError> {
    let input = match action {
        ApprovalAction::ExecCommand {
            stdin: Some(input), ..
        } if !input.is_empty() => input,
        ApprovalAction::WriteStdin { input, .. } if !input.is_empty() => input,
        _ => return Ok(()),
    };
    if input.contains('\0') {
        return Err(ToolError::Rejected(
            "terminal input contains a NUL byte and cannot be reviewed safely".to_string(),
        ));
    }

    let reviewed = crate::guardian::format_guardian_action_pretty(
        &action
            .clone()
            .into_guardian_request()
            .map_err(|error| ToolError::Rejected(error.to_string()))?,
    )
    .map_err(|error| ToolError::Rejected(error.to_string()))?;
    let reasons_len = approval_reason.map_or(0, str::len) + retry_reason.map_or(0, str::len);
    let oversized = reviewed.text.len().saturating_add(reasons_len) > MAX_TERMINAL_REVIEW_BYTES;
    let result = if reviewed.truncated {
        "formatter_truncated"
    } else if oversized {
        "over_limit"
    } else {
        "within_limit"
    };
    let input_kind = if input.chars().all(char::is_control) {
        "control"
    } else {
        "text"
    };
    telemetry.counter(
        "codex.unified_exec.stdin_review.size_check",
        /*inc*/ 1,
        &[("result", result), ("input_kind", input_kind)],
    );
    if reviewed.truncated || oversized {
        return Err(ToolError::Rejected(
            "terminal input and permission details are too large to review safely; use a smaller input or start a new terminal with fewer grants".to_string(),
        ));
    }
    Ok(())
}

fn approval_error(reason: impl std::fmt::Display) -> UnifiedExecError {
    UnifiedExecError::StdinApproval(ToolError::Rejected(reason.to_string()))
}

#[cfg(test)]
#[path = "stdin_approval_tests.rs"]
mod tests;
