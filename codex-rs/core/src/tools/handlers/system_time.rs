use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::context::boxed_tool_output;
use crate::tools::handlers::parse_arguments;
use crate::tools::handlers::system_time_spec::DEFAULT_SYSTEM_TIME_FORMAT;
use crate::tools::handlers::system_time_spec::DEFAULT_SYSTEM_TIME_OFFSET;
use crate::tools::handlers::system_time_spec::GET_SYSTEM_TIME_TOOL_NAME;
use crate::tools::handlers::system_time_spec::create_get_system_time_tool;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use chrono::DateTime;
use chrono::FixedOffset;
use chrono::Local;
use chrono::Offset;
use chrono::SecondsFormat;
use chrono::Utc;
use chrono::format::StrftimeItems;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use serde::Deserialize;
use serde::Serialize;

pub struct SystemTimeHandler;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SystemTimeArgs {
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    offset: Option<String>,
    #[serde(default)]
    full: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RequestedOffset {
    Local,
    Fixed { label: String, offset: FixedOffset },
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct SystemTimeShortResponse {
    formatted: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct SystemTimeFullResponse {
    formatted: String,
    format: String,
    offset: String,
    resolved_offset: String,
    resolved_offset_seconds: i32,
    unix_seconds: i64,
    unix_millis: i64,
    rfc3339: String,
    utc_rfc3339: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(untagged)]
enum SystemTimeResponse {
    Short(SystemTimeShortResponse),
    Full(SystemTimeFullResponse),
}

impl ToolExecutor<ToolInvocation> for SystemTimeHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain(GET_SYSTEM_TIME_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        create_get_system_time_tool()
    }

    fn handle<'a>(&'a self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'a>
    where
        ToolInvocation: 'a,
    {
        Box::pin(async move {
            let ToolInvocation { payload, .. } = invocation;

            let arguments = match payload {
                ToolPayload::Function { arguments } => arguments,
                _ => {
                    return Err(FunctionCallError::RespondToModel(format!(
                        "{GET_SYSTEM_TIME_TOOL_NAME} handler received unsupported payload"
                    )));
                }
            };

            let args: SystemTimeArgs = parse_arguments(&arguments)?;
            let response = system_time_response(args)?;
            let content = serde_json::to_string(&response).map_err(|err| {
                FunctionCallError::Fatal(format!(
                    "failed to serialize {GET_SYSTEM_TIME_TOOL_NAME} response: {err}"
                ))
            })?;

            Ok(boxed_tool_output(FunctionToolOutput::from_text(
                content,
                Some(true),
            )))
        })
    }
}

impl CoreToolRuntime for SystemTimeHandler {}

fn system_time_response(args: SystemTimeArgs) -> Result<SystemTimeResponse, FunctionCallError> {
    let full = args.full;
    let format = args
        .format
        .unwrap_or_else(|| DEFAULT_SYSTEM_TIME_FORMAT.to_string());
    let format_items = StrftimeItems::new(&format)
        .parse_to_owned()
        .map_err(|err| {
            FunctionCallError::RespondToModel(format!("invalid chrono strftime format: {err}"))
        })?;
    let requested_offset = parse_requested_offset(args.offset.as_deref())?;

    let (offset_label, selected, utc) = match requested_offset {
        RequestedOffset::Local => {
            let local = Local::now();
            let offset = local.offset().fix();
            (
                DEFAULT_SYSTEM_TIME_OFFSET.to_string(),
                local.with_timezone(&offset),
                local.with_timezone(&Utc),
            )
        }
        RequestedOffset::Fixed { label, offset } => {
            let utc = Utc::now();
            (label, utc.with_timezone(&offset), utc)
        }
    };

    let formatted = selected
        .format_with_items(format_items.iter().cloned())
        .to_string();

    if !full {
        return Ok(SystemTimeResponse::Short(SystemTimeShortResponse {
            formatted,
        }));
    }

    Ok(SystemTimeResponse::Full(build_full_response(
        formatted,
        format,
        offset_label,
        selected,
        utc,
    )))
}

fn build_full_response(
    formatted: String,
    format: String,
    offset_label: String,
    selected: DateTime<FixedOffset>,
    utc: DateTime<Utc>,
) -> SystemTimeFullResponse {
    let resolved_offset_seconds = selected.offset().local_minus_utc();
    SystemTimeFullResponse {
        formatted,
        format,
        offset: offset_label,
        resolved_offset: format_fixed_offset(resolved_offset_seconds),
        resolved_offset_seconds,
        unix_seconds: selected.timestamp(),
        unix_millis: selected.timestamp_millis(),
        rfc3339: selected.to_rfc3339_opts(SecondsFormat::Millis, false),
        utc_rfc3339: utc.to_rfc3339_opts(SecondsFormat::Millis, true),
    }
}

fn parse_requested_offset(offset: Option<&str>) -> Result<RequestedOffset, FunctionCallError> {
    let Some(offset) = offset else {
        return Ok(RequestedOffset::Local);
    };
    let trimmed = offset.trim();
    let normalized = trimmed.to_ascii_lowercase();
    match normalized.as_str() {
        "" | "local" => Ok(RequestedOffset::Local),
        "utc" => Ok(RequestedOffset::Fixed {
            label: "utc".to_string(),
            offset: Utc.fix(),
        }),
        _ => parse_fixed_offset(trimmed)
            .map(|fixed_offset| RequestedOffset::Fixed {
                label: format_fixed_offset(fixed_offset.local_minus_utc()),
                offset: fixed_offset,
            })
            .map_err(FunctionCallError::RespondToModel),
    }
}

fn parse_fixed_offset(value: &str) -> Result<FixedOffset, String> {
    let bytes = value.as_bytes();
    if bytes.len() != 6
        || !matches!(bytes[0], b'+' | b'-')
        || bytes[3] != b':'
        || !bytes[1].is_ascii_digit()
        || !bytes[2].is_ascii_digit()
        || !bytes[4].is_ascii_digit()
        || !bytes[5].is_ascii_digit()
    {
        return Err(offset_error(value));
    }

    let hours = digit(bytes[1]) * 10 + digit(bytes[2]);
    let minutes = digit(bytes[4]) * 10 + digit(bytes[5]);
    if hours > 23 || minutes > 59 {
        return Err(offset_error(value));
    }

    let seconds = hours * 60 * 60 + minutes * 60;
    let signed_seconds = if bytes[0] == b'-' { -seconds } else { seconds };
    FixedOffset::east_opt(signed_seconds).ok_or_else(|| offset_error(value))
}

fn digit(byte: u8) -> i32 {
    (byte - b'0') as i32
}

fn offset_error(value: &str) -> String {
    format!(
        "invalid offset `{value}`; use `local`, `utc`, or a fixed offset in `+HH:MM`/`-HH:MM` form"
    )
}

fn format_fixed_offset(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let total = seconds.abs();
    let hours = total / 3600;
    let minutes = total % 3600 / 60;
    format!("{sign}{hours:02}:{minutes:02}")
}

#[cfg(test)]
#[path = "system_time_tests.rs"]
mod tests;
