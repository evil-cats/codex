use super::*;
use codex_protocol::ThreadId;
use pretty_assertions::assert_eq;

fn thread_id(value: &str) -> ThreadId {
    ThreadId::from_string(value).expect("valid thread id")
}

#[test]
fn root_agent_name_prefers_config_name_over_profile() {
    let effective_config: TomlValue = toml::from_str("name = \"Hermione\"\n").expect("valid toml");

    assert_eq!(
        root_agent_name_from_values(&effective_config, Some("hermione")),
        Some("Hermione".to_string())
    );
}

#[test]
fn root_agent_name_falls_back_to_profile() {
    let effective_config: TomlValue = toml::from_str("").expect("valid toml");

    assert_eq!(
        root_agent_name_from_values(&effective_config, Some("hermione")),
        Some("hermione".to_string())
    );
}

#[test]
fn agent_name_from_thread_spawn_prefers_agent_role() {
    let source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: thread_id("00000000-0000-0000-0000-000000000001"),
        depth: 1,
        agent_path: Some(AgentPath::try_from("/root/research").unwrap()),
        agent_nickname: Some("fast reader".to_string()),
        agent_role: Some("Researcher".to_string()),
    });

    assert_eq!(
        agent_name_from_session_source(&source),
        Some("Researcher".to_string())
    );
}

#[test]
fn agent_name_from_thread_spawn_falls_back_to_agent_path_name() {
    let source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: thread_id("00000000-0000-0000-0000-000000000001"),
        depth: 1,
        agent_path: Some(AgentPath::try_from("/root/research").unwrap()),
        agent_nickname: Some("fast reader".to_string()),
        agent_role: None,
    });

    assert_eq!(
        agent_name_from_session_source(&source),
        Some("research".to_string())
    );
}

#[test]
fn stored_agent_name_falls_back_to_path_leaf() {
    assert_eq!(
        agent_name_from_stored_fields(None, Some("/root/research"), Some("nickname")),
        Some("research".to_string())
    );
}
