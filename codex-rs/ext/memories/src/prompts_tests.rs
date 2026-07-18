use super::*;
use codex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use tempfile::tempdir;
use tokio::fs as tokio_fs;

#[tokio::test]
async fn build_memory_tool_developer_instructions_renders_embedded_template() {
    let temp = tempdir().unwrap();
    let codex_home = AbsolutePathBuf::from_absolute_path(temp.path()).unwrap();
    let memories_dir = codex_home.join("memories");
    tokio_fs::create_dir_all(&memories_dir).await.unwrap();
    tokio_fs::write(
        memories_dir.join("memory_summary.md"),
        "Short memory summary for tests.",
    )
    .await
    .unwrap();

    let instructions = build_memory_tool_developer_instructions(&codex_home)
        .await
        .unwrap();

    assert!(instructions.contains(&format!(
        "- {}/memory_summary.md (already provided below; do NOT open again)",
        memories_dir.display()
    )));
    assert!(instructions.contains("Short memory summary for tests."));
    assert!(instructions.contains("A direct \"remember this\""));
    assert!(!instructions.contains("only when explicitly asked by the user"));
    assert_eq!(
        instructions
            .matches("========= MEMORY_SUMMARY BEGINS =========")
            .count(),
        1
    );
}

#[tokio::test]
async fn build_memory_tool_developer_instructions_bounds_memory_summary() {
    let temp = tempdir().unwrap();
    let codex_home = AbsolutePathBuf::from_absolute_path(temp.path()).unwrap();
    let memories_dir = codex_home.join("memories");
    tokio_fs::create_dir_all(&memories_dir).await.unwrap();
    tokio_fs::write(
        memories_dir.join("memory_summary.md"),
        format!(
            "Summary prefix. {}Summary tail must be truncated.",
            "memory ".repeat(10_000)
        ),
    )
    .await
    .unwrap();

    let instructions = build_memory_tool_developer_instructions(&codex_home)
        .await
        .unwrap();

    assert!(instructions.contains("Summary prefix."));
    assert!(instructions.contains("tokens truncated"));
    assert!(instructions.contains("Summary tail must be truncated."));
}

#[tokio::test]
async fn build_memory_tool_developer_instructions_skips_unusable_summary() {
    let temp = tempdir().unwrap();
    let codex_home = AbsolutePathBuf::from_absolute_path(temp.path()).unwrap();
    let memories_dir = codex_home.join("memories");
    tokio_fs::create_dir_all(&memories_dir).await.unwrap();
    let memory_summary_path = memories_dir.join("memory_summary.md");

    assert_eq!(
        build_memory_tool_developer_instructions(&codex_home).await,
        None
    );

    tokio_fs::write(&memory_summary_path, " \n\t ")
        .await
        .unwrap();
    assert_eq!(
        build_memory_tool_developer_instructions(&codex_home).await,
        None
    );

    tokio_fs::write(&memory_summary_path, [0xff]).await.unwrap();
    assert_eq!(
        build_memory_tool_developer_instructions(&codex_home).await,
        None
    );
}

#[test]
#[should_panic(expected = "unsupported placeholder")]
fn parse_embedded_template_rejects_unknown_placeholder() {
    let _ = parse_embedded_template("{{ unknown }}", "test-template.md");
}
