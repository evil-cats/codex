use codex_protocol::ThreadId;
#[cfg(test)]
use codex_protocol::config_types::EnvironmentVariablePattern;
use codex_protocol::config_types::ShellEnvironmentPolicy;
use codex_protocol::shell_environment;
use std::collections::HashMap;

pub use codex_protocol::shell_environment::CODEX_AGENT_ENV_VAR;
pub use codex_protocol::shell_environment::CODEX_THREAD_ID_ENV_VAR;

/// Construct an environment map based on the rules in the specified policy. The
/// resulting map can be passed directly to `Command::envs()` after calling
/// `env_clear()` to ensure no unintended variables are leaked to the spawned
/// process.
///
/// The derivation follows the algorithm documented in the struct-level comment
/// for [`ShellEnvironmentPolicy`].
///
/// `CODEX_THREAD_ID` and `CODEX_AGENT` are injected when their values are
/// provided, even when `include_only` is set.
pub fn create_env(
    policy: &ShellEnvironmentPolicy,
    thread_id: Option<ThreadId>,
    agent_name: Option<&str>,
) -> HashMap<String, String> {
    let thread_id = thread_id.map(|thread_id| thread_id.to_string());
    shell_environment::create_env(policy, thread_id.as_deref(), agent_name)
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
#[path = "exec_env_tests.rs"]
mod tests;
