use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::read_file_spec::READ_FILE_TOOL_NAME;
use crate::tools::handlers::system_time_spec::DEFAULT_SYSTEM_TIME_OFFSET;
use crate::tools::handlers::system_time_spec::GET_SYSTEM_TIME_TOOL_NAME;
use crate::tools::handlers::thread_info_spec::GET_THREAD_INFO_TOOL_NAME;
use codex_protocol::items::CoreToolActivityItem;
use codex_protocol::items::CoreToolActivityKind;
use codex_protocol::items::CoreToolActivityStatus;
use codex_protocol::items::TurnItem;
use serde_json::Value as JsonValue;
use std::path::Path;
use std::time::Instant;

pub(crate) struct CoreToolActivityHandle {
    item: CoreToolActivityItem,
    started_at: Instant,
}

impl CoreToolActivityHandle {
    pub(crate) async fn maybe_start(invocation: &ToolInvocation) -> Option<Self> {
        let item = core_tool_activity_item(invocation, CoreToolActivityStatus::InProgress)?;
        invocation
            .session
            .emit_turn_item_started(&invocation.turn, &TurnItem::CoreToolActivity(item.clone()))
            .await;
        Some(Self {
            item,
            started_at: Instant::now(),
        })
    }

    pub(crate) async fn complete(
        mut self,
        invocation: &ToolInvocation,
        success: bool,
        error: Option<String>,
    ) {
        self.item.status = if success {
            CoreToolActivityStatus::Completed
        } else {
            CoreToolActivityStatus::Failed
        };
        self.item.duration = Some(self.started_at.elapsed());
        self.item.error = error;
        invocation
            .session
            .emit_turn_item_completed(&invocation.turn, TurnItem::CoreToolActivity(self.item))
            .await;
    }
}

fn core_tool_activity_item(
    invocation: &ToolInvocation,
    status: CoreToolActivityStatus,
) -> Option<CoreToolActivityItem> {
    if invocation.tool_name.namespace.is_some() {
        return None;
    }
    let ToolPayload::Function { arguments } = &invocation.payload else {
        return None;
    };
    let arguments_json = parse_arguments_json(arguments);
    let (kind, detail) = match invocation.tool_name.name.as_str() {
        READ_FILE_TOOL_NAME => {
            let cwd = invocation
                .turn
                .environments
                .primary()
                .map(|environment| environment.cwd().as_path())
                .unwrap_or_else(|| invocation.turn.config.cwd.as_path());
            (
                CoreToolActivityKind::File,
                read_file_detail(&arguments_json, cwd),
            )
        }
        GET_THREAD_INFO_TOOL_NAME => (
            CoreToolActivityKind::ThreadInfo,
            thread_info_detail(&arguments_json),
        ),
        GET_SYSTEM_TIME_TOOL_NAME => (
            CoreToolActivityKind::SystemTime,
            system_time_detail(&arguments_json),
        ),
        _ => return None,
    };

    Some(CoreToolActivityItem {
        id: invocation.call_id.clone(),
        tool_name: invocation.tool_name.name.clone(),
        kind,
        detail,
        arguments: arguments_json,
        status,
        error: None,
        duration: None,
    })
}

fn parse_arguments_json(arguments: &str) -> JsonValue {
    if arguments.trim().is_empty() {
        return JsonValue::Object(serde_json::Map::new());
    }
    serde_json::from_str(arguments).unwrap_or_else(|_| JsonValue::String(arguments.to_string()))
}

fn read_file_detail(arguments: &JsonValue, cwd: &Path) -> String {
    let path = string_arg(arguments, "path").unwrap_or("unknown");
    display_file_arg(path, cwd)
}

fn thread_info_detail(arguments: &JsonValue) -> String {
    let Some(thread_id) = string_arg(arguments, "thread_id") else {
        return "current".to_string();
    };
    let thread_id = thread_id.trim();
    if thread_id.is_empty() {
        "current".to_string()
    } else {
        thread_id.chars().take(8).collect()
    }
}

fn system_time_detail(arguments: &JsonValue) -> String {
    let Some(offset) = string_arg(arguments, "offset") else {
        return DEFAULT_SYSTEM_TIME_OFFSET.to_string();
    };
    let offset = offset.trim();
    if offset.is_empty() {
        DEFAULT_SYSTEM_TIME_OFFSET.to_string()
    } else if offset.eq_ignore_ascii_case("local") {
        "local".to_string()
    } else if offset.eq_ignore_ascii_case("utc") {
        "utc".to_string()
    } else {
        offset.to_string()
    }
}

fn string_arg<'a>(arguments: &'a JsonValue, key: &str) -> Option<&'a str> {
    arguments.get(key)?.as_str()
}

fn display_path_arg(path: &str, cwd: &Path) -> String {
    let candidate = Path::new(path);
    if candidate.is_absolute()
        && let Ok(relative) = candidate.strip_prefix(cwd)
    {
        return relative.display().to_string();
    }
    path.to_string()
}

fn display_file_arg(path: &str, cwd: &Path) -> String {
    short_display_path(&display_path_arg(path, cwd))
}

fn short_display_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/');
    let mut parts = trimmed.split('/').rev().filter(|part| {
        !part.is_empty()
            && *part != "build"
            && *part != "dist"
            && *part != "node_modules"
            && *part != "src"
    });
    parts
        .next()
        .map(str::to_string)
        .unwrap_or_else(|| trimmed.to_string())
}
