//! Регрессионные проверки пользовательских деталей core tool activity.

use super::GET_SYSTEM_TIME_TOOL_NAME;
use super::GET_THREAD_INFO_TOOL_NAME;
use super::READ_FILE_TOOL_NAME;
use super::core_tool_activity_kind;
use super::read_file_detail;
use crate::config::PermissionProfileSnapshot;
use crate::environment_selection::EnvironmentConfigOrigin;
use crate::environment_selection::TurnEnvironmentSnapshot;
use crate::environment_selection::TurnEnvironmentState;
use crate::session::turn_context::TurnEnvironment;
use codex_exec_server::Environment;
use codex_protocol::items::CoreToolActivityKind;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::EnvironmentConfig;
use codex_protocol::protocol::EnvironmentConfigState;
use codex_protocol::protocol::TurnEnvironmentSelection;
use codex_tools::ToolName;
use codex_utils_path_uri::PathUri;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

#[test]
fn normalized_default_namespace_remains_visible() {
    let actual = [
        READ_FILE_TOOL_NAME,
        GET_THREAD_INFO_TOOL_NAME,
        GET_SYSTEM_TIME_TOOL_NAME,
    ]
    .map(|name| core_tool_activity_kind(&ToolName::plain(name).with_default_namespace()));

    assert_eq!(
        [
            Some(CoreToolActivityKind::File),
            Some(CoreToolActivityKind::ThreadInfo),
            Some(CoreToolActivityKind::SystemTime),
        ],
        actual
    );
}

#[test]
fn non_default_namespace_remains_hidden() {
    assert_eq!(
        None,
        core_tool_activity_kind(&ToolName::namespaced("extension", READ_FILE_TOOL_NAME))
    );
}

#[tokio::test]
async fn read_file_detail_uses_selected_environment_path_convention() {
    let environment = Arc::new(Environment::default_for_tests());
    let environment_config = EnvironmentConfig {
        allow_login_shell: true,
        permission_profile: PermissionProfileSnapshot::legacy(PermissionProfile::read_only()),
        shell_environment_policy: Default::default(),
        exec_policy: None,
        mcp_policy: None,
        network_policy: None,
        selected_capability_roots: Vec::new(),
    };
    let environments = TurnEnvironmentSnapshot {
        environments: vec![
            TurnEnvironmentState::Ready(TurnEnvironment::new(
                TurnEnvironmentSelection {
                    environment_id: "primary".to_string(),
                    cwd: PathUri::parse("file:///primary").expect("primary cwd URI"),
                    workspace_roots: Vec::new(),
                    config: EnvironmentConfigState::Ready(environment_config.clone()),
                },
                EnvironmentConfigOrigin::Thread,
                Arc::clone(&environment),
                /*shell*/ None,
            )),
            TurnEnvironmentState::Ready(TurnEnvironment::new(
                TurnEnvironmentSelection {
                    environment_id: "remote".to_string(),
                    cwd: PathUri::parse("file:///C:/workspace").expect("foreign Windows cwd URI"),
                    workspace_roots: Vec::new(),
                    config: EnvironmentConfigState::Ready(environment_config),
                },
                EnvironmentConfigOrigin::Thread,
                Arc::clone(&environment),
                /*shell*/ None,
            )),
        ],
    };
    let arguments = json!({
        "path": r"C:\workspace\src",
        "environment_id": "remote",
    });

    assert_eq!(
        "src",
        read_file_detail(&arguments, &environments, Path::new("/primary"))
    );
}
