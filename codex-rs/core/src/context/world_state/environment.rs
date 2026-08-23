use super::PreviousSectionState;
use super::WorldStateSection;
use crate::context::ContextualUserFragment;
use crate::context::environment_context::FileSystemContext;
use crate::context::environment_context::NetworkContext;
use crate::context::environment_context::push_xml_escaped_text;
use crate::environment_selection::TurnEnvironmentSnapshot;
use crate::session::turn_context::TurnContext;
use crate::session::turn_context::TurnEnvironment;
use codex_exec_server::LOCAL_ENVIRONMENT_ID;
use codex_protocol::protocol::TurnContextItem;
use codex_protocol::protocol::TurnContextNetworkItem;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_path_uri::PathUri;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

/// Environment values visible to the model.
#[derive(Clone, Debug, Default)]
pub(crate) struct EnvironmentsState {
    environments: BTreeMap<String, EnvironmentState>,
    project_name: Option<String>,
    current_date: Option<String>,
    timezone: Option<String>,
    network: Option<NetworkContext>,
    filesystem: Option<FileSystemContext>,
    subagents: Option<String>,
}

impl EnvironmentsState {
    pub(crate) fn from_turn_context_with_environments(
        turn_context: &TurnContext,
        environments: &TurnEnvironmentSnapshot,
        current_date: Option<String>,
    ) -> Self {
        let primary_environment = environments.primary();
        let workspace_roots = primary_environment
            .map(TurnEnvironment::workspace_roots)
            .unwrap_or_default();
        Self {
            environments: environment_states(environments),
            project_name: project_name_from_workspace_roots(workspace_roots),
            current_date,
            timezone: turn_context.timezone.clone(),
            network: network_from_turn_context(turn_context),
            filesystem: primary_environment.map(|environment| {
                FileSystemContext::from_permission_profile(
                    environment.permission_profile(),
                    workspace_roots,
                )
            }),
            subagents: None,
        }
    }

    pub(crate) fn from_turn_context_item(turn_context_item: &TurnContextItem) -> Self {
        let workspace_roots = workspace_roots_from_turn_context_item(turn_context_item)
            .iter()
            .map(PathUri::from_abs_path)
            .collect::<Vec<_>>();
        Self {
            environments: [(
                LOCAL_ENVIRONMENT_ID.to_string(),
                EnvironmentState {
                    cwd: PathUri::from_abs_path(&turn_context_item.cwd),
                    status: EnvironmentStatus::Available,
                    shell: None,
                    is_primary: false,
                },
            )]
            .into_iter()
            .collect(),
            project_name: project_name_from_workspace_roots(&workspace_roots),
            current_date: turn_context_item.current_date.clone(),
            timezone: turn_context_item.timezone.clone(),
            network: network_from_turn_context_item(turn_context_item),
            filesystem: Some(FileSystemContext::from_permission_profile(
                &turn_context_item.permission_profile(),
                &workspace_roots,
            )),
            subagents: None,
        }
    }

    pub(crate) fn with_subagents(mut self, subagents: String) -> Self {
        if !subagents.is_empty() {
            self.subagents = Some(subagents);
        }
        self
    }

    fn rendered_full(&self) -> RenderedEnvironments {
        RenderedEnvironments {
            updates: self
                .environments
                .iter()
                .map(|(id, environment)| {
                    (id.clone(), EnvironmentUpdate::Current(environment.clone()))
                })
                .collect(),
            legacy_single: is_legacy_single(&self.environments),
            project_name: self.project_name.clone(),
            include_primary: self.environments.len() > 1,
            current_date: self.current_date.clone(),
            timezone: self.timezone.clone(),
            network: self.network.clone(),
            filesystem: self.filesystem.clone(),
            subagents: self.subagents.clone(),
        }
    }
}

impl WorldStateSection for EnvironmentsState {
    const ID: &'static str = "environments";
    type Snapshot = EnvironmentsSnapshot;

    fn snapshot(&self) -> Self::Snapshot {
        EnvironmentsSnapshot {
            environments: self
                .environments
                .iter()
                .map(|(id, environment)| {
                    (
                        id.clone(),
                        EnvironmentSnapshot {
                            cwd: environment.cwd.inferred_native_path_string(),
                            status: environment.status,
                            shell: environment.shell.clone(),
                            is_primary: self.environments.len() > 1 && environment.is_primary,
                        },
                    )
                })
                .collect(),
            project_name: self.project_name.clone(),
            current_date: self.current_date.clone(),
            timezone: self.timezone.clone(),
            network: self.network.as_ref().map(NetworkContext::render),
            filesystem: self.filesystem.as_ref().map(FileSystemContext::render),
            subagents: self.subagents.clone(),
        }
    }

    fn render_diff(
        &self,
        previous: PreviousSectionState<'_, Self::Snapshot>,
    ) -> Option<Box<dyn ContextualUserFragment>> {
        let current = self.snapshot();
        let empty = EnvironmentsSnapshot::default();
        let previous = match previous {
            PreviousSectionState::Known(previous) => previous,
            PreviousSectionState::Absent | PreviousSectionState::Unknown => &empty,
        };
        let turn_context_values_changed = current.project_name != previous.project_name
            || current.current_date != previous.current_date
            || current.timezone != previous.timezone
            || current.network != previous.network
            || current.filesystem != previous.filesystem;
        let multiple_environments = self.environments.len() > 1;
        let previous_multiple_environments = previous.environments.len() > 1;
        let mut updates = self
            .environments
            .iter()
            .filter(|(id, _)| {
                let environment = &current.environments[*id];
                previous.environments.get(*id).is_none_or(|previous| {
                    multiple_environments != previous_multiple_environments
                        || !environment.has_same_diff_value(previous)
                })
            })
            .map(|(id, environment)| (id.clone(), EnvironmentUpdate::Current(environment.clone())))
            .collect::<BTreeMap<_, _>>();
        updates.extend(
            previous
                .environments
                .keys()
                .filter(|id| !self.environments.contains_key(*id))
                .map(|id| (id.clone(), EnvironmentUpdate::Unavailable)),
        );
        let legacy_single = is_legacy_single(&self.environments)
            && updates
                .values()
                .all(|update| matches!(update, EnvironmentUpdate::Current(_)));
        (!updates.is_empty() || turn_context_values_changed).then(|| {
            Box::new(RenderedEnvironments {
                updates,
                legacy_single,
                project_name: self.project_name.clone(),
                include_primary: multiple_environments || previous_multiple_environments,
                current_date: self.current_date.clone(),
                timezone: self.timezone.clone(),
                network: self.network.clone(),
                filesystem: self.filesystem.clone(),
                subagents: self.subagents.clone(),
            }) as Box<dyn ContextualUserFragment>
        })
    }
}

impl ContextualUserFragment for EnvironmentsState {
    fn role(&self) -> &'static str {
        "user"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        environment_context_markers()
    }

    fn body(&self) -> String {
        self.rendered_full().body()
    }
}

struct RenderedEnvironments {
    updates: BTreeMap<String, EnvironmentUpdate>,
    legacy_single: bool,
    project_name: Option<String>,
    include_primary: bool,
    current_date: Option<String>,
    timezone: Option<String>,
    network: Option<NetworkContext>,
    filesystem: Option<FileSystemContext>,
    subagents: Option<String>,
}

enum EnvironmentUpdate {
    Current(EnvironmentState),
    Unavailable,
}

impl ContextualUserFragment for RenderedEnvironments {
    fn role(&self) -> &'static str {
        "user"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        environment_context_markers()
    }

    fn body(&self) -> String {
        let mut rendered = "\n".to_string();
        if self.legacy_single {
            if let Some(EnvironmentUpdate::Current(environment)) = self.updates.values().next() {
                push_environment_values(&mut rendered, environment, "  ");
            }
        } else if !self.updates.is_empty() {
            rendered.push_str("  <environments>\n");
            for (id, update) in &self.updates {
                match update {
                    EnvironmentUpdate::Current(environment) => {
                        rendered.push_str("    <environment id=\"");
                        push_xml_escaped_text(&mut rendered, id);
                        rendered.push('"');
                        if self.include_primary {
                            rendered.push_str(if environment.is_primary {
                                " primary=\"true\""
                            } else {
                                " primary=\"false\""
                            });
                        }
                        rendered.push_str(">\n");
                        push_environment_values(&mut rendered, environment, "      ");
                        rendered.push_str("    </environment>\n");
                    }
                    EnvironmentUpdate::Unavailable => {
                        rendered.push_str("    <environment id=\"");
                        push_xml_escaped_text(&mut rendered, id);
                        rendered.push_str("\" status=\"unavailable\" />\n");
                    }
                }
            }
            rendered.push_str("  </environments>\n");
        }
        push_optional_element(&mut rendered, "project_name", self.project_name.as_deref());
        push_optional_element(&mut rendered, "current_date", self.current_date.as_deref());
        push_optional_element(&mut rendered, "timezone", self.timezone.as_deref());
        if let Some(network) = &self.network {
            rendered.push_str("  ");
            rendered.push_str(&network.render());
            rendered.push('\n');
        }
        if let Some(filesystem) = &self.filesystem {
            rendered.push_str("  ");
            rendered.push_str(&filesystem.render());
            rendered.push('\n');
        }
        if let Some(subagents) = &self.subagents {
            rendered.push_str("  <subagents>\n");
            for line in subagents.lines() {
                rendered.push_str("    ");
                rendered.push_str(line);
                rendered.push('\n');
            }
            rendered.push_str("  </subagents>\n");
        }
        rendered
    }
}

fn push_environment_values(rendered: &mut String, environment: &EnvironmentState, indent: &str) {
    rendered.push_str(indent);
    rendered.push_str("<cwd>");
    push_xml_escaped_text(rendered, &environment.cwd.inferred_native_path_string());
    rendered.push_str("</cwd>\n");
    if environment.status == EnvironmentStatus::Starting {
        rendered.push_str(indent);
        rendered.push_str("<status>starting</status>\n");
    }
    if let Some(shell) = &environment.shell {
        rendered.push_str(indent);
        rendered.push_str("<shell>");
        push_xml_escaped_text(rendered, shell);
        rendered.push_str("</shell>\n");
    }
}

fn push_optional_element(rendered: &mut String, name: &str, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    rendered.push_str("  <");
    rendered.push_str(name);
    rendered.push('>');
    push_xml_escaped_text(rendered, value);
    rendered.push_str("</");
    rendered.push_str(name);
    rendered.push_str(">\n");
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EnvironmentState {
    cwd: PathUri,
    status: EnvironmentStatus,
    shell: Option<String>,
    is_primary: bool,
}

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct EnvironmentsSnapshot {
    environments: BTreeMap<String, EnvironmentSnapshot>,
    project_name: Option<String>,
    current_date: Option<String>,
    timezone: Option<String>,
    network: Option<String>,
    filesystem: Option<String>,
    subagents: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct EnvironmentSnapshot {
    cwd: String,
    status: EnvironmentStatus,
    shell: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    is_primary: bool,
}

impl EnvironmentSnapshot {
    fn has_same_diff_value(&self, other: &Self) -> bool {
        self.cwd == other.cwd
            && self.status == other.status
            && self.is_primary == other.is_primary
            && self
                .shell
                .as_ref()
                .zip(other.shell.as_ref())
                .is_none_or(|(current, previous)| current == previous)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum EnvironmentStatus {
    Starting,
    Available,
}

fn environment_states(snapshot: &TurnEnvironmentSnapshot) -> BTreeMap<String, EnvironmentState> {
    let mut environments = snapshot
        .turn_environments()
        .enumerate()
        .map(|(index, environment)| {
            (
                environment.selection.environment_id.clone(),
                EnvironmentState {
                    cwd: environment.cwd().clone(),
                    status: EnvironmentStatus::Available,
                    shell: environment
                        .shell
                        .as_ref()
                        .map(|shell| shell.name().to_string()),
                    is_primary: index == 0,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    for environment in snapshot.starting() {
        environments
            .entry(environment.selection.environment_id.clone())
            .or_insert_with(|| EnvironmentState {
                cwd: environment.selection.cwd.clone(),
                status: EnvironmentStatus::Starting,
                shell: None,
                is_primary: false,
            });
    }
    environments
}

fn is_legacy_single(environments: &BTreeMap<String, EnvironmentState>) -> bool {
    environments.len() == 1
        && environments
            .values()
            .all(|environment| environment.status == EnvironmentStatus::Available)
}

fn environment_context_markers() -> (&'static str, &'static str) {
    (
        codex_protocol::protocol::ENVIRONMENT_CONTEXT_OPEN_TAG,
        codex_protocol::protocol::ENVIRONMENT_CONTEXT_CLOSE_TAG,
    )
}

fn network_from_turn_context(turn_context: &TurnContext) -> Option<NetworkContext> {
    let network = turn_context
        .config
        .config_layer_stack
        .requirements()
        .network
        .as_ref()?;

    Some(NetworkContext::new(
        network
            .domains
            .as_ref()
            .and_then(codex_config::NetworkDomainPermissionsToml::allowed_domains)
            .unwrap_or_default(),
        network
            .domains
            .as_ref()
            .and_then(codex_config::NetworkDomainPermissionsToml::denied_domains)
            .unwrap_or_default(),
    ))
}

fn network_from_turn_context_item(turn_context_item: &TurnContextItem) -> Option<NetworkContext> {
    let TurnContextNetworkItem {
        allowed_domains,
        denied_domains,
    } = turn_context_item.network.as_ref()?;
    Some(NetworkContext::new(
        allowed_domains.clone(),
        denied_domains.clone(),
    ))
}

fn workspace_roots_from_turn_context_item(
    turn_context_item: &TurnContextItem,
) -> Vec<AbsolutePathBuf> {
    if let Some(workspace_roots) = turn_context_item.workspace_roots.as_ref() {
        return workspace_roots.clone();
    }

    vec![turn_context_item.cwd.clone()]
}

fn project_name_from_workspace_roots(workspace_roots: &[PathUri]) -> Option<String> {
    workspace_roots.first().map(|root| {
        root.basename()
            .unwrap_or_else(|| root.inferred_native_path_string())
    })
}

#[cfg(test)]
#[path = "environment_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "environment_render_tests.rs"]
mod render_tests;
