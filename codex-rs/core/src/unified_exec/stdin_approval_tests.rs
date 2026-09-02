//! Проверяет общую границу approval для начального и последующего stdin.

use super::*;
use crate::session::tests::make_session_and_context;
use codex_protocol::models::PermissionProfile;
use codex_utils_path_uri::PathUri;
use pretty_assertions::assert_eq;
use serde_json::json;

fn terminal_permissions(profile: &PermissionProfile) -> TerminalPermissions {
    TerminalPermissions {
        policy: TerminalPolicy {
            sandbox: FileSystemSandboxContext::from_permission_profile(
                effective_permission_profile(profile, /*additional_permissions*/ None),
            ),
            environment_network: None,
            controller_network: None,
            controller_proxy: false,
        },
        sandbox_source: TerminalSandboxSource::Native,
        launch_permissions: SandboxPermissions::UseDefault,
        additional_permissions: None,
        internal_permissions: None,
    }
}

fn exec_approval(input: String) -> ApprovalAction {
    ApprovalAction::ExecCommand {
        id: "exec-1".to_string(),
        environment_id: codex_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
        command: vec!["cat".to_string()],
        stdin: Some(input),
        hook_command: "cat".to_string(),
        cwd: PathUri::parse("file:///tmp").expect("valid cwd"),
        sandbox_permissions: SandboxPermissions::RequireEscalated,
        additional_permissions: None,
        justification: None,
        tty: false,
        proposed_execpolicy_amendment: None,
    }
}

#[test]
fn exec_command_stdin_default_launch_does_not_require_review() -> anyhow::Result<()> {
    let permissions = terminal_permissions(&PermissionProfile::read_only());

    assert!(
        permissions
            .initial_stdin_approval(
                Some("ordinary data"),
                /*command_was_reviewed*/ false,
                SandboxPermissions::UseDefault,
                /*reviewed_additional_permissions*/ None,
            )?
            .is_none()
    );
    Ok(())
}

#[test]
fn exec_command_stdin_matching_command_review_covers_the_same_launch() -> anyhow::Result<()> {
    let mut permissions = terminal_permissions(&PermissionProfile::Disabled);
    permissions.launch_permissions = SandboxPermissions::RequireEscalated;

    assert!(
        permissions
            .initial_stdin_approval(
                Some("possibly executable input"),
                /*command_was_reviewed*/ true,
                SandboxPermissions::RequireEscalated,
                /*reviewed_additional_permissions*/ None,
            )?
            .is_none()
    );
    Ok(())
}

#[test]
fn exec_command_stdin_elevated_launch_requires_fresh_review_after_cached_command()
-> anyhow::Result<()> {
    let mut permissions = terminal_permissions(&PermissionProfile::Disabled);
    permissions.launch_permissions = SandboxPermissions::RequireEscalated;

    let approval = permissions
        .initial_stdin_approval(
            Some("run this"),
            /*command_was_reviewed*/ false,
            SandboxPermissions::UseDefault,
            /*reviewed_additional_permissions*/ None,
        )?
        .expect("elevated stdin must be reviewed");
    assert_eq!(
        approval.sandbox_permissions,
        SandboxPermissions::RequireEscalated
    );
    assert_eq!(approval.additional_permissions, None);
    Ok(())
}

#[test]
fn exec_command_stdin_internal_grant_requires_review_without_exposing_its_path()
-> anyhow::Result<()> {
    let grants = serde_json::from_value(json!({
        "file_system": {"write": ["/private/metrics"]}
    }))?;
    let mut permissions = terminal_permissions(&PermissionProfile::read_only());
    permissions.internal_permissions = Some(grants);

    let approval = permissions
        .initial_stdin_approval(
            Some("input"),
            /*command_was_reviewed*/ true,
            SandboxPermissions::UseDefault,
            /*reviewed_additional_permissions*/ None,
        )?
        .expect("internal grant must require a factual second review");
    assert_eq!(
        approval.sandbox_permissions,
        SandboxPermissions::WithAdditionalPermissions
    );
    assert_eq!(approval.additional_permissions, None);
    assert!(
        approval
            .reason
            .contains("internal plugin metrics write grant")
    );
    assert!(!approval.reason.contains("/private/metrics"));
    Ok(())
}

#[tokio::test]
async fn exec_command_stdin_review_rejects_nul_and_unreviewable_tail() -> anyhow::Result<()> {
    let (_session, turn) = make_session_and_context().await;

    let nul = validate_terminal_input_review(
        &exec_approval("before\0after".to_string()),
        /*approval_reason*/ None,
        /*retry_reason*/ None,
        &turn.session_telemetry,
    );
    assert!(matches!(
        nul,
        Err(ToolError::Rejected(reason)) if reason.contains("NUL byte")
    ));

    let oversized = validate_terminal_input_review(
        &exec_approval("\"".repeat(4_100)),
        /*approval_reason*/ None,
        /*retry_reason*/ None,
        &turn.session_telemetry,
    );
    assert!(matches!(
        oversized,
        Err(ToolError::Rejected(reason)) if reason.contains("too large to review safely")
    ));
    Ok(())
}

#[test]
fn exec_command_stdin_retained_terminal_uses_the_same_permission_boundary() {
    let baseline = PermissionProfile::read_only();
    let mut permissions = terminal_permissions(&baseline);
    permissions.launch_permissions = SandboxPermissions::RequireEscalated;
    let current = terminal_permissions(&baseline);

    assert_eq!(
        permissions.review_requirement(&current.policy, &baseline),
        Ok(SandboxPermissions::RequireEscalated)
    );
}
