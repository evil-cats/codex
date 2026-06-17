use crate::config::Config;
use crate::session::turn_context::TurnContext;
use codex_config::ConfigLayerSource;
use codex_protocol::AgentPath;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use toml::Value as TomlValue;

pub(crate) fn current_agent_name(turn: &TurnContext) -> Option<String> {
    if let Some(agent_name) = agent_name_from_session_source(&turn.session_source) {
        return Some(agent_name);
    }

    if turn.session_source.is_non_root_agent() {
        return None;
    }

    root_agent_name_from_config(&turn.config)
}

pub(crate) fn agent_name_from_stored_fields(
    agent_role: Option<&str>,
    agent_path: Option<&str>,
    agent_nickname: Option<&str>,
) -> Option<String> {
    non_empty_trimmed_opt(agent_role)
        .or_else(|| agent_path.and_then(agent_path_name_from_str))
        .or_else(|| non_empty_trimmed_opt(agent_nickname))
}

fn agent_name_from_session_source(session_source: &SessionSource) -> Option<String> {
    let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        agent_path,
        agent_nickname,
        agent_role,
        ..
    }) = session_source
    else {
        return None;
    };

    non_empty_trimmed_opt(agent_role.as_deref())
        .or_else(|| agent_path.as_ref().and_then(agent_path_name))
        .or_else(|| non_empty_trimmed_opt(agent_nickname.as_deref()))
}

fn agent_path_name(agent_path: &AgentPath) -> Option<String> {
    non_empty_trimmed(agent_path.name())
}

fn agent_path_name_from_str(agent_path: &str) -> Option<String> {
    agent_path.rsplit('/').find_map(non_empty_trimmed)
}

fn root_agent_name_from_config(config: &Config) -> Option<String> {
    let effective_config = config.config_layer_stack.effective_config();
    let active_profile = active_user_profile(config);
    root_agent_name_from_values(&effective_config, active_profile.as_deref())
}

fn root_agent_name_from_values(
    effective_config: &TomlValue,
    active_profile: Option<&str>,
) -> Option<String> {
    effective_config
        .get("name")
        .and_then(TomlValue::as_str)
        .and_then(non_empty_trimmed)
        .or_else(|| active_profile.and_then(non_empty_trimmed))
}

fn active_user_profile(config: &Config) -> Option<String> {
    let active_user_layer = config.config_layer_stack.get_active_user_layer()?;
    let ConfigLayerSource::User { profile, .. } = &active_user_layer.name else {
        return None;
    };
    profile.clone()
}

fn non_empty_trimmed_opt(value: Option<&str>) -> Option<String> {
    value.and_then(non_empty_trimmed)
}

fn non_empty_trimmed(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
#[path = "agent_name_tests.rs"]
mod tests;
