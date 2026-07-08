use codex_protocol::ThreadId;
#[cfg(test)]
use codex_protocol::config_types::EnvironmentVariablePattern;
use codex_protocol::config_types::ShellEnvironmentPolicy;
use codex_protocol::models::ActivePermissionProfile;
use codex_protocol::shell_environment;
use std::collections::HashMap;
use std::path::Path;

pub use codex_protocol::shell_environment::CODEX_AGENT_ENV_VAR;
pub use codex_protocol::shell_environment::CODEX_CALL_ID_ENV_VAR;
pub use codex_protocol::shell_environment::CODEX_ROLLOUT_ENV_VAR;
pub use codex_protocol::shell_environment::CODEX_THREAD_ID_ENV_VAR;

/// Informational name of the active permission profile. Child processes can
/// overwrite this value, so it must not be treated as proof of enforcement.
pub const CODEX_PERMISSION_PROFILE_ENV_VAR: &str = "CODEX_PERMISSION_PROFILE";

#[derive(Clone, Copy, Debug, Default)]
pub struct RuntimeEnv<'a> {
    pub thread_id: Option<ThreadId>,
    pub agent_name: Option<&'a str>,
    pub call_id: Option<&'a str>,
    pub rollout_path: Option<&'a Path>,
}

/// Construct an environment map based on the rules in the specified policy. The
/// resulting map can be passed directly to `Command::envs()` after calling
/// `env_clear()` to ensure no unintended variables are leaked to the spawned
/// process.
///
/// The derivation follows the algorithm documented in the struct-level comment
/// for [`ShellEnvironmentPolicy`].
///
/// Runtime identity variables such as `CODEX_THREAD_ID`, `CODEX_AGENT`,
/// `CODEX_CALL_ID`, and `CODEX_ROLLOUT` are injected when their values are
/// provided, even when `include_only` is set.
pub fn create_env(
    policy: &ShellEnvironmentPolicy,
    thread_id: Option<ThreadId>,
    agent_name: Option<&str>,
) -> HashMap<String, String> {
    create_env_with_runtime(
        policy,
        RuntimeEnv {
            thread_id,
            agent_name,
            ..Default::default()
        },
    )
}

pub fn create_env_with_runtime(
    policy: &ShellEnvironmentPolicy,
    runtime: RuntimeEnv<'_>,
) -> HashMap<String, String> {
    let thread_id = runtime.thread_id.map(|thread_id| thread_id.to_string());
    shell_environment::create_env_with_runtime(
        policy,
        shell_environment::RuntimeEnv {
            thread_id: thread_id.as_deref(),
            agent_name: runtime.agent_name,
            call_id: runtime.call_id,
            rollout_path: runtime.rollout_path,
        },
    )
}

/// Injects the selected named permission profile into a shell tool's environment.
///
/// This is applied after the shell environment policy so the runtime-selected
/// profile wins over inherited or configured values.
pub(crate) fn inject_permission_profile_env(
    env: &mut HashMap<String, String>,
    active_permission_profile: Option<&ActivePermissionProfile>,
) {
    if cfg!(windows) {
        env.retain(|key, _| !key.eq_ignore_ascii_case(CODEX_PERMISSION_PROFILE_ENV_VAR));
    } else {
        env.remove(CODEX_PERMISSION_PROFILE_ENV_VAR);
    }
    if let Some(active_permission_profile) = active_permission_profile {
        env.insert(
            CODEX_PERMISSION_PROFILE_ENV_VAR.to_string(),
            active_permission_profile.id.clone(),
        );
    }
}

#[cfg(all(test, target_os = "windows"))]
fn create_env_from_vars<I>(
    vars: I,
    policy: &ShellEnvironmentPolicy,
    thread_id: Option<ThreadId>,
    agent_name: Option<&str>,
) -> HashMap<String, String>
where
    I: IntoIterator<Item = (String, String)>,
{
    let thread_id = thread_id.map(|thread_id| thread_id.to_string());
    shell_environment::create_env_from_vars(vars, policy, thread_id.as_deref(), agent_name)
}

#[cfg(test)]
fn populate_env<I>(
    vars: I,
    policy: &ShellEnvironmentPolicy,
    thread_id: Option<ThreadId>,
    agent_name: Option<&str>,
) -> HashMap<String, String>
where
    I: IntoIterator<Item = (String, String)>,
{
    let thread_id = thread_id.map(|thread_id| thread_id.to_string());
    shell_environment::populate_env(vars, policy, thread_id.as_deref(), agent_name)
}

#[cfg(test)]
fn populate_env_with_runtime<I>(
    vars: I,
    policy: &ShellEnvironmentPolicy,
    runtime: RuntimeEnv<'_>,
) -> HashMap<String, String>
where
    I: IntoIterator<Item = (String, String)>,
{
    let thread_id = runtime.thread_id.map(|thread_id| thread_id.to_string());
    shell_environment::populate_env_with_runtime(
        vars,
        policy,
        shell_environment::RuntimeEnv {
            thread_id: thread_id.as_deref(),
            agent_name: runtime.agent_name,
            call_id: runtime.call_id,
            rollout_path: runtime.rollout_path,
        },
    )
}

#[cfg(test)]
#[path = "exec_env_tests.rs"]
mod tests;
