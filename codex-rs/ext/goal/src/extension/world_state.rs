//! Восстанавливает точный active-goal context через extension-owned `WorldState`.
//! Маркер очистки создаётся только после snapshot с доказанным состоянием `active`
//! и доставляется одному sampling без записи в conversation history.

use codex_extension_api::ContextContributor;
use codex_extension_api::ExtensionFuture;
use codex_extension_api::PreviousWorldStateSection;
use codex_extension_api::RenderedWorldStateFragment;
use codex_extension_api::WorldStateContributionInput;
use codex_extension_api::WorldStateSectionContribution;
use codex_protocol::protocol::ThreadGoal;
use codex_state::ThreadGoalStatus;
use serde_json::json;

use super::GoalExtension;
use super::goal_runtime_handle;
use crate::steering::active_goal_context_prompt;
use crate::tool::protocol_goal_from_state;

const ACTIVE_GOAL_WORLD_STATE_ID: &str = "active_goal";
const THREAD_GOAL_CONTEXT_OPEN_TAG: &str = "<thread_goal_context>";
const THREAD_GOAL_CONTEXT_CLOSE_TAG: &str = "</thread_goal_context>";
const NO_ACTIVE_GOAL_BODY: &str = "\
The previously active thread goal is no longer active. Do not continue it unless the user \
activates or replaces it.";
const UNAVAILABLE_GOAL_BODY: &str = "\
The active thread goal state is temporarily unavailable. Do not treat an earlier \
goal context fragment as current or change the goal based on stale context.";

impl<C> ContextContributor for GoalExtension<C>
where
    C: Send + Sync + 'static,
{
    fn matches_model_context_fragment(&self, role: &str, text: &str) -> bool {
        is_thread_goal_context_fragment(role, text)
    }

    fn contribute_world_state<'a>(
        &'a self,
        input: WorldStateContributionInput<'a>,
    ) -> ExtensionFuture<'a, Vec<WorldStateSectionContribution>> {
        Box::pin(async move {
            let Some(runtime) = goal_runtime_handle(input.thread_store) else {
                return Vec::new();
            };

            let active_goal = if runtime.tools_visible() {
                match self
                    .state_dbs
                    .thread_goals()
                    .get_thread_goal(runtime.thread_id())
                    .await
                {
                    Ok(goal) => goal
                        .filter(|goal| goal.status == ThreadGoalStatus::Active)
                        .map(protocol_goal_from_state),
                    Err(err) => {
                        tracing::warn!(
                            thread_id = %runtime.thread_id(),
                            "failed to read active goal for model context: {err}"
                        );
                        return vec![unavailable_goal_world_state_section()];
                    }
                }
            } else {
                None
            };

            vec![active_goal_world_state_section(active_goal.as_ref())]
        })
    }
}

/// Формирует стабильную секцию и очищает только доказанный предыдущий snapshot `active`.
fn active_goal_world_state_section(
    active_goal: Option<&ThreadGoal>,
) -> WorldStateSectionContribution {
    let body = active_goal.map(active_goal_context_prompt);
    let snapshot = json!({
        "state": if body.is_some() { "active" } else { "inactive" },
        "body": body,
    });
    let expected_snapshot = snapshot.clone();
    let retained_body = body.clone();
    let current_is_active = active_goal.is_some();

    let contribution =
        WorldStateSectionContribution::new(ACTIVE_GOAL_WORLD_STATE_ID, snapshot, move |previous| {
            let previous_was_active = match &previous {
                PreviousWorldStateSection::Known(previous) if *previous == &expected_snapshot => {
                    return None;
                }
                PreviousWorldStateSection::Known(previous) => {
                    previous.get("state").and_then(serde_json::Value::as_str) == Some("active")
                }
                PreviousWorldStateSection::Absent | PreviousWorldStateSection::Unknown => false,
            };

            let body = match body.as_deref() {
                Some(body) => body,
                None if !previous_was_active => return None,
                None => NO_ACTIVE_GOAL_BODY,
            };
            let fragment = RenderedWorldStateFragment::new(
                "user",
                (THREAD_GOAL_CONTEXT_OPEN_TAG, THREAD_GOAL_CONTEXT_CLOSE_TAG),
                body,
            );
            Some(if current_is_active {
                fragment
            } else {
                fragment.for_next_sampling_only()
            })
        });

    match retained_body {
        Some(body) => contribution.with_retained_fragment_matcher(move |role, text| {
            role == "user"
                && text.trim_start().starts_with(THREAD_GOAL_CONTEXT_OPEN_TAG)
                && text.trim_end().ends_with(THREAD_GOAL_CONTEXT_CLOSE_TAG)
                && text.contains(&body)
        }),
        None => contribution,
    }
}

fn is_thread_goal_context_fragment(role: &str, text: &str) -> bool {
    role == "user"
        && text.trim_start().starts_with(THREAD_GOAL_CONTEXT_OPEN_TAG)
        && text.trim_end().ends_with(THREAD_GOAL_CONTEXT_CLOSE_TAG)
}

fn unavailable_goal_world_state_section() -> WorldStateSectionContribution {
    let snapshot = json!({ "state": "unavailable" });
    let expected_snapshot = snapshot.clone();

    WorldStateSectionContribution::new(ACTIVE_GOAL_WORLD_STATE_ID, snapshot, move |previous| {
        if let PreviousWorldStateSection::Known(previous) = previous
            && previous == &expected_snapshot
        {
            return None;
        }
        Some(RenderedWorldStateFragment::new(
            "user",
            (THREAD_GOAL_CONTEXT_OPEN_TAG, THREAD_GOAL_CONTEXT_CLOSE_TAG),
            UNAVAILABLE_GOAL_BODY,
        ))
    })
    .with_retained_fragment_matcher(|role, text| {
        role == "user"
            && text.trim_start().starts_with(THREAD_GOAL_CONTEXT_OPEN_TAG)
            && text.trim_end().ends_with(THREAD_GOAL_CONTEXT_CLOSE_TAG)
            && text.contains(UNAVAILABLE_GOAL_BODY)
    })
}
