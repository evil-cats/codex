use crate::environment_selection::TurnEnvironmentSnapshot;
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
use codex_tools::ToolName;
use serde_json::Value as JsonValue;
use std::path::Path;
use std::time::Instant;

const CLOCK_NAMESPACE: &str = "clock";
const CLOCK_CURRENT_TIME_TOOL_NAME: &str = "curr_time";

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

/// Строит ограниченный UI-элемент `CoreToolActivityItem` только для поддерживаемого
/// function tool, сохраняя исходные `arguments` и диагностическое имя с namespace.
fn core_tool_activity_item(
    invocation: &ToolInvocation,
    status: CoreToolActivityStatus,
) -> Option<CoreToolActivityItem> {
    let ToolPayload::Function { arguments } = &invocation.payload else {
        return None;
    };
    let arguments_json = parse_arguments_json(arguments);
    let kind = core_tool_activity_kind(&invocation.tool_name)?;
    let detail = match kind {
        CoreToolActivityKind::File => read_file_detail(
            &arguments_json,
            &invocation.step_context.environments,
            invocation.turn.config.cwd.as_path(),
        ),
        CoreToolActivityKind::ThreadInfo => thread_info_detail(&arguments_json),
        CoreToolActivityKind::SystemTime => {
            system_time_detail(&invocation.tool_name, &arguments_json)
        }
    };
    let tool_name = core_tool_activity_tool_name(&invocation.tool_name);

    Some(CoreToolActivityItem {
        id: invocation.call_id.clone(),
        tool_name,
        kind,
        detail,
        arguments: arguments_json,
        status,
        error: None,
        duration: None,
    })
}

/// Сопоставляет инструменты default namespace и точную upstream-пару
/// `clock/curr_time` с видами пользовательской activity, не захватывая остальные
/// namespace.
fn core_tool_activity_kind(tool_name: &ToolName) -> Option<CoreToolActivityKind> {
    if is_clock_current_time(tool_name) {
        return Some(CoreToolActivityKind::SystemTime);
    }
    if !tool_name.is_default_namespace() {
        return None;
    }
    match tool_name.name.as_str() {
        READ_FILE_TOOL_NAME => Some(CoreToolActivityKind::File),
        GET_THREAD_INFO_TOOL_NAME => Some(CoreToolActivityKind::ThreadInfo),
        GET_SYSTEM_TIME_TOOL_NAME => Some(CoreToolActivityKind::SystemTime),
        _ => None,
    }
}

/// Проверяет точную пару namespace/name для upstream-инструмента чтения времени.
fn is_clock_current_time(tool_name: &ToolName) -> bool {
    tool_name.namespace.as_deref() == Some(CLOCK_NAMESPACE)
        && tool_name.name == CLOCK_CURRENT_TIME_TOOL_NAME
}

/// Сохраняет namespace в диагностическом имени, не меняя имена инструментов
/// default namespace.
fn core_tool_activity_tool_name(tool_name: &ToolName) -> String {
    match tool_name.namespace.as_deref() {
        Some(namespace) if !tool_name.is_default_namespace() => {
            format!("{namespace}/{}", tool_name.name)
        }
        _ => tool_name.name.clone(),
    }
}

fn parse_arguments_json(arguments: &str) -> JsonValue {
    if arguments.trim().is_empty() {
        return JsonValue::Object(serde_json::Map::new());
    }
    serde_json::from_str(arguments).unwrap_or_else(|_| JsonValue::String(arguments.to_string()))
}

fn read_file_detail(
    arguments: &JsonValue,
    environments: &TurnEnvironmentSnapshot,
    fallback_cwd: &Path,
) -> String {
    let path = string_arg(arguments, "path").unwrap_or("unknown");
    let environment = string_arg(arguments, "environment_id").map_or_else(
        || environments.primary(),
        |environment_id| {
            environments
                .turn_environments()
                .find(|environment| environment.selection.environment_id == environment_id)
        },
    );
    if let Some(resolved_path) =
        environment.and_then(|environment| environment.cwd().join(path).ok())
        && let Some(basename) = resolved_path.basename()
    {
        return basename;
    }

    display_file_arg(path, fallback_cwd)
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

/// Формирует краткую деталь времени; `clock/curr_time` всегда возвращает UTC,
/// тогда как fork-инструмент сохраняет выбранное пользователем значение `offset`.
fn system_time_detail(tool_name: &ToolName, arguments: &JsonValue) -> String {
    if is_clock_current_time(tool_name) {
        return "utc".to_string();
    }
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

#[cfg(test)]
#[path = "core_tool_activity_tests.rs"]
mod tests;
