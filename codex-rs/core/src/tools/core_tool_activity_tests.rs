//! Регрессионные проверки пользовательских деталей core tool activity.

use super::read_file_detail;
use crate::environment_selection::TurnEnvironmentSnapshot;
use crate::session::turn_context::TurnEnvironment;
use codex_exec_server::Environment;
use codex_utils_path_uri::PathUri;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

#[tokio::test]
async fn read_file_detail_uses_selected_environment_path_convention() {
    let environment = Arc::new(Environment::default_for_tests());
    let environments = TurnEnvironmentSnapshot {
        turn_environments: vec![
            TurnEnvironment::new(
                "primary".to_string(),
                Arc::clone(&environment),
                PathUri::parse("file:///primary").expect("primary cwd URI"),
                /*shell*/ None,
            ),
            TurnEnvironment::new(
                "remote".to_string(),
                Arc::clone(&environment),
                PathUri::parse("file:///C:/workspace").expect("foreign Windows cwd URI"),
                /*shell*/ None,
            ),
        ],
        starting: Vec::new(),
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
