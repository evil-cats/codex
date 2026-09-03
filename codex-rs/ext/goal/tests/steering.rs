//! Проверяет, что фильтрация active-goal context по доступным возможностям
//! сохраняет пользовательские данные.
#![allow(dead_code)]

#[path = "../src/steering.rs"]
mod steering;

use codex_protocol::ThreadId;
use codex_protocol::protocol::ThreadGoal;
use codex_protocol::protocol::ThreadGoalStatus;
use codex_utils_template::Template;
use pretty_assertions::assert_eq;

/// При доступном `update_plan` active context должен совпадать с полным шаблоном.
#[test]
fn enabled_checklist_preserves_the_original_active_context() {
    let goal = test_goal("Finish the feature.");
    let original = Template::parse(include_str!("../templates/goals/active_context.md"))
        .expect("original active context template")
        .render([
            ("objective", goal.objective.as_str()),
            ("token_budget", "10000"),
        ])
        .expect("render original active context prompt");

    assert_eq!(steering::active_goal_context_prompt(&goal, true), original);
}

/// Фильтрация инструкций отключённого `update_plan` не должна изменять
/// совпадающий текст внутри objective.
#[test]
fn disabled_checklist_preserves_goal_text_that_mentions_the_tool() {
    let objective = "Inspect update_plan.\n\n## Plan tool\nThis is user task data.";
    let text = steering::active_goal_context_prompt(&test_goal(objective), false);

    assert!(text.contains(objective));
    assert!(!text.contains("If update_plan is available"));
    assert!(text.contains("Completion audit:"));
}

fn test_goal(objective: &str) -> ThreadGoal {
    ThreadGoal {
        thread_id: ThreadId::new(),
        objective: objective.to_string(),
        status: ThreadGoalStatus::Active,
        token_budget: Some(10_000),
        tokens_used: 100,
        time_used_seconds: 0,
        created_at: 0,
        updated_at: 0,
    }
}
