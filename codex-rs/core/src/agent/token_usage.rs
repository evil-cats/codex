//! Session-tree token accounting for live root turns.
//!
//! The ledger is shared through `AgentControl`, attributes every completed response to its direct
//! thread, and emits immutable snapshots at separator-independent lifecycle boundaries. It keeps
//! late agent completions alive after the root turn ends, but never reconstructs old summaries
//! from rollout records that lack a model slug.

use codex_protocol::AgentPath;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ModelTokenUsageSnapshot;
use codex_protocol::protocol::RootTurnTokenUsageSnapshot;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::TokenUsageSnapshot;
use codex_protocol::protocol::compare_model_slugs;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct RootTurnUsageLedger {
    roots: HashMap<String, RootTurnUsage>,
}

struct RootTurnUsage {
    root_thread_id: ThreadId,
    root: ThreadUsage,
    agents: HashMap<ThreadId, AgentUsage>,
    seen_responses: HashSet<(ThreadId, String)>,
    root_snapshot_emitted: bool,
}

#[derive(Default)]
struct ThreadUsage {
    by_model: HashMap<Option<String>, UsageAggregate>,
    runs: HashMap<String, RunUsage>,
}

struct AgentUsage {
    agent_path: AgentPath,
    default_model: Option<String>,
    usage: ThreadUsage,
    /// Successful dispatch observed before the child starts its corresponding turn.
    pending_run: bool,
    /// Child turn observed before its successful dispatch returned to the initiator.
    unregistered_run: bool,
}

#[derive(Default)]
struct RunUsage {
    in_flight: Option<InFlightCall>,
    observed_response: bool,
    terminal: bool,
}

struct InFlightCall {
    model: Option<String>,
}

#[derive(Clone, Default)]
struct UsageAggregate {
    usage: Option<TokenUsage>,
    incomplete: bool,
}

impl RootTurnUsageLedger {
    /// Records participation before the child has necessarily created its own turn id.
    pub(super) fn register_agent(
        &mut self,
        root_turn_id: &str,
        root_thread_id: ThreadId,
        agent_thread_id: ThreadId,
        agent_path: AgentPath,
        model: Option<String>,
    ) {
        let root = self.root_mut(root_turn_id, root_thread_id);
        let agent = root
            .agents
            .entry(agent_thread_id)
            .or_insert_with(|| AgentUsage {
                agent_path: agent_path.clone(),
                default_model: model.clone(),
                usage: ThreadUsage::default(),
                pending_run: false,
                unregistered_run: false,
            });
        agent.agent_path = agent_path;
        if model.is_some() {
            agent.default_model = model;
        }
        agent.register_run();
    }

    /// Opens the model-call boundary used to detect an interrupted response without `usage`.
    pub(super) fn begin_model_call(
        &mut self,
        root_turn_id: &str,
        root_thread_id: ThreadId,
        thread_id: ThreadId,
        turn_id: &str,
        model: &str,
    ) {
        let root = self.root_mut(root_turn_id, root_thread_id);
        let usage = root.thread_usage_mut(thread_id);
        let (previous_in_flight, new_run) = {
            let new_run = !usage.runs.contains_key(turn_id);
            let run = usage.runs.entry(turn_id.to_string()).or_default();
            if run.terminal {
                (None, new_run)
            } else {
                (
                    run.in_flight.replace(InFlightCall {
                        model: known_model(model),
                    }),
                    new_run,
                )
            }
        };
        if let Some(previous_in_flight) = previous_in_flight {
            usage
                .by_model
                .entry(previous_in_flight.model)
                .or_default()
                .incomplete = true;
        }
        if let Some(agent) = root.agents.get_mut(&thread_id) {
            if new_run {
                agent.observe_run();
            }
            if !model.is_empty() {
                agent.default_model = Some(model.to_string());
            }
        }
    }

    /// Rebinds the open call to the model slug reported by the upstream response headers.
    pub(super) fn update_model_call(
        &mut self,
        root_turn_id: &str,
        root_thread_id: ThreadId,
        thread_id: ThreadId,
        turn_id: &str,
        model: &str,
    ) {
        let root = self.root_mut(root_turn_id, root_thread_id);
        let usage = root.thread_usage_mut(thread_id);
        let run = usage.runs.entry(turn_id.to_string()).or_default();
        if !run.terminal {
            run.in_flight = Some(InFlightCall {
                model: known_model(model),
            });
        }
        if let Some(agent) = root.agents.get_mut(&thread_id)
            && !model.is_empty()
        {
            agent.default_model = Some(model.to_string());
        }
    }

    /// Adds one response exactly once and closes its in-flight call boundary.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn record_response(
        &mut self,
        root_turn_id: &str,
        root_thread_id: ThreadId,
        thread_id: ThreadId,
        turn_id: &str,
        response_id: &str,
        model: &str,
        usage: Option<&TokenUsage>,
    ) {
        let root = self.root_mut(root_turn_id, root_thread_id);
        if !root
            .seen_responses
            .insert((thread_id, response_id.to_string()))
        {
            return;
        }
        let new_run = {
            let thread_usage = root.thread_usage_mut(thread_id);
            let new_run = !thread_usage.runs.contains_key(turn_id);
            let run = thread_usage.runs.entry(turn_id.to_string()).or_default();
            run.in_flight = None;
            run.observed_response = true;
            thread_usage
                .by_model
                .entry(known_model(model))
                .or_default()
                .record_response(usage);
            new_run
        };
        if new_run && let Some(agent) = root.agents.get_mut(&thread_id) {
            agent.observe_run();
        }
    }

    /// Closes one child turn once and returns the agent's cumulative direct usage in this root turn.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn complete_agent_turn(
        &mut self,
        root_turn_id: &str,
        root_thread_id: ThreadId,
        agent_thread_id: ThreadId,
        agent_path: AgentPath,
        agent_turn_id: &str,
        fallback_model: Option<String>,
    ) -> Option<TokenUsageSnapshot> {
        let root = self.root_mut(root_turn_id, root_thread_id);
        let agent = root
            .agents
            .entry(agent_thread_id)
            .or_insert_with(|| AgentUsage {
                agent_path: agent_path.clone(),
                default_model: fallback_model.clone(),
                usage: ThreadUsage::default(),
                pending_run: false,
                unregistered_run: false,
            });
        agent.agent_path = agent_path;
        if fallback_model.is_some() {
            agent.default_model = fallback_model;
        }
        let new_run = !agent.usage.runs.contains_key(agent_turn_id);
        if new_run {
            agent.observe_run();
        }
        let (in_flight, observed_response) = {
            let run = agent
                .usage
                .runs
                .entry(agent_turn_id.to_string())
                .or_default();
            if run.terminal {
                return None;
            }
            run.terminal = true;
            (run.in_flight.take(), run.observed_response)
        };
        if let Some(in_flight) = in_flight {
            agent
                .usage
                .by_model
                .entry(in_flight.model)
                .or_default()
                .incomplete = true;
        } else if !observed_response {
            agent
                .usage
                .by_model
                .entry(agent.default_model.clone())
                .or_default()
                .incomplete = true;
        }
        agent.pending_run = false;
        let snapshot = agent.snapshot(/*include_in_flight*/ false);

        let remove_root = root.root_snapshot_emitted && !root.has_running_agents();
        if remove_root {
            self.roots.remove(root_turn_id);
        }
        Some(snapshot)
    }

    /// Freezes the root, all direct agent totals, and their per-model union for the terminal event.
    pub(super) fn complete_root_turn(
        &mut self,
        root_turn_id: &str,
        root_thread_id: ThreadId,
        root_turn_run_id: &str,
    ) -> RootTurnTokenUsageSnapshot {
        let root = self.root_mut(root_turn_id, root_thread_id);
        root.root.mark_run_terminal(root_turn_run_id);
        root.root_snapshot_emitted = true;
        let snapshot = root.snapshot();
        if !root.has_running_agents() {
            self.roots.remove(root_turn_id);
        }
        snapshot
    }

    fn root_mut(&mut self, root_turn_id: &str, root_thread_id: ThreadId) -> &mut RootTurnUsage {
        self.roots
            .entry(root_turn_id.to_string())
            .or_insert_with(|| RootTurnUsage {
                root_thread_id,
                root: ThreadUsage::default(),
                agents: HashMap::new(),
                seen_responses: HashSet::new(),
                root_snapshot_emitted: false,
            })
    }
}

impl RootTurnUsage {
    fn thread_usage_mut(&mut self, thread_id: ThreadId) -> &mut ThreadUsage {
        if thread_id == self.root_thread_id {
            &mut self.root
        } else {
            &mut self
                .agents
                .entry(thread_id)
                .or_insert_with(|| AgentUsage {
                    agent_path: AgentPath::root(),
                    default_model: None,
                    usage: ThreadUsage::default(),
                    pending_run: false,
                    unregistered_run: false,
                })
                .usage
        }
    }

    fn has_running_agents(&self) -> bool {
        self.agents.values().any(AgentUsage::is_running)
    }

    fn snapshot(&self) -> RootTurnTokenUsageSnapshot {
        let turn = self.root.snapshot(
            /*include_in_flight*/ false, /*default_model*/ None,
            /*running_run*/ false,
        );
        let mut agents_by_model = HashMap::new();
        for agent in self.agents.values() {
            merge_snapshot(
                &mut agents_by_model,
                &agent.snapshot(/*include_in_flight*/ true),
            );
        }
        let agents = snapshot_from_aggregates(agents_by_model);
        let mut total_by_model = HashMap::new();
        merge_snapshot(&mut total_by_model, &turn);
        merge_snapshot(&mut total_by_model, &agents);
        let agent_count = u64::try_from(self.agents.len()).unwrap_or(u64::MAX);
        let running_agent_count = u64::try_from(
            self.agents
                .values()
                .filter(|agent| agent.is_running())
                .count(),
        )
        .unwrap_or(u64::MAX);
        RootTurnTokenUsageSnapshot {
            turn,
            agents,
            total: snapshot_from_aggregates(total_by_model),
            agent_count,
            running_agent_count,
        }
    }
}

impl ThreadUsage {
    fn mark_run_terminal(&mut self, turn_id: &str) {
        let run = self.runs.entry(turn_id.to_string()).or_default();
        if let Some(in_flight) = run.in_flight.take() {
            self.by_model.entry(in_flight.model).or_default().incomplete = true;
        }
        run.terminal = true;
    }

    fn snapshot(
        &self,
        include_in_flight: bool,
        default_model: Option<String>,
        running_run: bool,
    ) -> TokenUsageSnapshot {
        let mut by_model = self.by_model.clone();
        if include_in_flight {
            for run in self.runs.values().filter(|run| !run.terminal) {
                if let Some(in_flight) = &run.in_flight {
                    by_model
                        .entry(in_flight.model.clone())
                        .or_default()
                        .incomplete = true;
                }
            }
        }
        if running_run {
            by_model
                .entry(default_model.clone())
                .or_default()
                .incomplete = true;
        }
        if by_model.is_empty() && default_model.is_some() {
            by_model.entry(default_model).or_default().incomplete = true;
        }
        snapshot_from_aggregates(by_model)
    }
}

impl AgentUsage {
    fn is_running(&self) -> bool {
        self.pending_run || self.usage.runs.values().any(|run| !run.terminal)
    }

    fn register_run(&mut self) {
        if self.unregistered_run {
            self.unregistered_run = false;
        } else if !self.usage.runs.values().any(|run| !run.terminal) {
            self.pending_run = true;
        }
    }

    fn observe_run(&mut self) {
        if self.pending_run {
            self.pending_run = false;
        } else {
            self.unregistered_run = true;
        }
    }

    fn snapshot(&self, include_in_flight: bool) -> TokenUsageSnapshot {
        self.usage.snapshot(
            include_in_flight,
            self.default_model.clone(),
            include_in_flight && self.is_running(),
        )
    }
}

impl UsageAggregate {
    fn record_response(&mut self, usage: Option<&TokenUsage>) {
        let Some(usage) = usage else {
            self.incomplete = true;
            return;
        };
        let usage = normalized_usage(usage);
        match &mut self.usage {
            Some(total) => saturating_add_usage(total, &usage),
            None => self.usage = Some(usage),
        }
    }

    fn merge(&mut self, other: &Self) {
        if let Some(other_usage) = &other.usage {
            match &mut self.usage {
                Some(usage) => saturating_add_usage(usage, other_usage),
                None => self.usage = Some(other_usage.clone()),
            }
        }
        self.incomplete |= other.incomplete;
    }
}

fn merge_snapshot(
    target: &mut HashMap<Option<String>, UsageAggregate>,
    snapshot: &TokenUsageSnapshot,
) {
    for model in &snapshot.models {
        target
            .entry(model.model.clone())
            .or_default()
            .merge(&UsageAggregate {
                usage: model.usage.clone(),
                incomplete: model.incomplete,
            });
    }
}

fn snapshot_from_aggregates(
    by_model: HashMap<Option<String>, UsageAggregate>,
) -> TokenUsageSnapshot {
    let mut models = by_model
        .into_iter()
        .map(|(model, aggregate)| ModelTokenUsageSnapshot {
            model,
            usage: aggregate.usage,
            incomplete: aggregate.incomplete,
        })
        .collect::<Vec<_>>();
    models.sort_by(|left, right| match (&left.model, &right.model) {
        (Some(left), Some(right)) => compare_model_slugs(left, right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    TokenUsageSnapshot { models }
}

fn known_model(model: &str) -> Option<String> {
    (!model.is_empty()).then(|| model.to_string())
}

fn normalized_usage(usage: &TokenUsage) -> TokenUsage {
    TokenUsage {
        input_tokens: usage.input_tokens.max(0),
        cached_input_tokens: usage.cached_input_tokens.max(0),
        cache_write_input_tokens: usage.cache_write_input_tokens.max(0),
        output_tokens: usage.output_tokens.max(0),
        reasoning_output_tokens: usage.reasoning_output_tokens.max(0),
        total_tokens: usage.total_tokens.max(0),
        codex_rollout_budget_units: None,
    }
}

fn saturating_add_usage(total: &mut TokenUsage, usage: &TokenUsage) {
    total.input_tokens = total.input_tokens.saturating_add(usage.input_tokens);
    total.cached_input_tokens = total
        .cached_input_tokens
        .saturating_add(usage.cached_input_tokens);
    total.cache_write_input_tokens = total
        .cache_write_input_tokens
        .saturating_add(usage.cache_write_input_tokens);
    total.output_tokens = total.output_tokens.saturating_add(usage.output_tokens);
    total.reasoning_output_tokens = total
        .reasoning_output_tokens
        .saturating_add(usage.reasoning_output_tokens);
    total.total_tokens = total.total_tokens.saturating_add(usage.total_tokens);
}

#[cfg(test)]
#[path = "token_usage_tests.rs"]
mod tests;
