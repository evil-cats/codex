import argparse
import os
import re
import shlex
import shutil
import subprocess
import sys
from datetime import UTC, datetime
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
SKILL_ROOT = SCRIPT_DIR.parent
DEFAULT_REPO_ROOT = SKILL_ROOT.parents[2]
SOURCE_COVERAGE = SKILL_ROOT / "references" / "source-coverage.md"

OPEN_STATUSES = (
    "draft",
    "pending",
    "требует проверки",
)

BLOCKING_MIGRATION_STATUSES = (
    "pending",
    "проверяется",
    "open question",
    "требует исправления",
)

COVERAGE_ROW_STATUSES = (
    "перенесено",
    "перенесено с нормализацией",
    "не переносится",
)

RETIRED_LEGACY_SCRIPTS = (
    "scripts/fork-migration/remote-prepare-host.sh",
    "scripts/fork-migration/remote-apply-patch.sh",
    "scripts/fork-migration/remote-tests.sh",
    "scripts/fork-migration/remote-build-fast.sh",
)

REQUIRED_FILES = (
    "SKILL.md",
    "references/source-coverage.md",
    "references/fork-rules.md",
    "references/fork-card-contract.md",
    "references/parent-migration.md",
    "references/subagent-one-card.md",
    "references/local-development.md",
    "references/checks-and-gates.md",
    "assets/templates/fork-card.md",
    "assets/templates/migration-card.md",
    "assets/templates/parent-subagent-prompt.md",
    "scripts/fork",
)

SKILL_MARKDOWN = (
    "SKILL.md",
    "references/source-coverage.md",
    "references/fork-rules.md",
    "references/fork-card-contract.md",
    "references/parent-migration.md",
    "references/subagent-one-card.md",
    "references/local-development.md",
    "references/checks-and-gates.md",
    "assets/templates/fork-card.md",
    "assets/templates/migration-card.md",
    "assets/templates/parent-subagent-prompt.md",
)

CARD_REQUIRED_SECTION_GROUPS = (
    ("Обзор", ("Обзор",)),
    ("Зачем это нужно", ("Зачем это нужно",)),
    ("Карта файлов", ("Карта файлов", "Карта файлов и смысл правок")),
    (
        "Итоговый контракт",
        (
            "Итоговый контракт",
            "Принятый контракт",
            "Контракт внутренних документов",
        ),
    ),
    (
        "Порядок повторения при переносе",
        (
            "Порядок повторения при переносе",
            "Пошаговое воспроизведение",
            "Пошаговое воспроизведение доработки",
            "Порядок реализации",
        ),
    ),
    (
        "Проверки",
        (
            "Проверки",
            "Проверки для будущего переноса",
            "Исторические проверки",
            "Регрессионное покрытие",
            "Регрессионное покрытие в diff",
        ),
    ),
    (
        "Риски и ограничения",
        (
            "Риски и ограничения",
            "Риски",
            "Ограничения",
            "Ограничения и gates",
        ),
    ),
    (
        "Проверка покрытия",
        (
            "Проверка покрытия",
            "Сводка покрытия",
            "Регрессионное покрытие",
            "Регрессионное покрытие в diff",
        ),
    ),
)

CARD_TESTS = (
    ("codex-agent-env-var", ("just", "test", "-p", "codex-core", "exec_env")),
    ("codex-agent-env-var", ("just", "test", "-p", "codex-core", "agent_name")),
    ("codex-agent-env-var", ("just", "test", "-p", "codex-core", "thread_info")),
    (
        "codex-agent-env-var",
        (
            "just",
            "test",
            "-p",
            "codex-core",
            "maybe_wrap_shell_lc_with_snapshot_restores_codex_identity_from_env",
        ),
    ),
    (
        "codex-agent-env-var",
        (
            "just",
            "test",
            "-p",
            "codex-core",
            "env_overlay_for_exec_server_keeps_runtime_changes_only",
        ),
    ),
    (
        "codex-agent-env-var",
        (
            "just",
            "test",
            "-p",
            "codex-core",
            "shell_command_handler_to_exec_params_uses_session_shell_and_turn_context",
        ),
    ),
    ("codex-agent-env-var", ("just", "test", "-p", "codex-protocol", "shell_environment")),
    ("core-system-time-tool", ("just", "test", "-p", "codex-core", "system_time")),
    (
        "core-system-time-tool",
        ("just", "test", "-p", "codex-core", "prompt_tools_are_consistent_across_requests"),
    ),
    ("core-thread-info-tool", ("just", "test", "-p", "codex-core", "agent_name")),
    ("core-thread-info-tool", ("just", "test", "-p", "codex-core", "thread_info")),
    (
        "core-thread-info-tool",
        ("just", "test", "-p", "codex-core", "prompt_tools_are_consistent_across_requests"),
    ),
    ("developer-instructions-files", ("just", "test", "-p", "codex-core", "developer_instructions")),
    ("environment-context-project-name", ("just", "test", "-p", "codex-core", "environment_context")),
    ("exec-command-output-spill-files", ("just", "test", "-p", "codex-core", "inline_output_max_tokens")),
    ("exec-command-output-spill-files", ("just", "test", "-p", "codex-core", "output_spill")),
    (
        "exec-command-output-spill-files",
        (
            "just",
            "test",
            "-p",
            "codex-core",
            "exec_command_tool_output_formats_spill",
        ),
    ),
    (
        "exec-command-output-spill-files",
        (
            "just",
            "test",
            "-p",
            "codex-core",
            "exec_command_spills_large_completed_output_to_file",
        ),
    ),
    (
        "exec-command-output-spill-files",
        (
            "just",
            "test",
            "-p",
            "codex-core",
            "unified_exec_enforces_glob_deny_read_policy",
        ),
    ),
    (
        "exec-command-output-spill-files",
        (
            "just",
            "test",
            "-p",
            "codex-core",
            "unified_exec_timeout_and_followup_poll",
        ),
    ),
    ("hermione-version-metadata", ("just", "test", "-p", "codex-cli")),
    (
        "hermione-version-metadata",
        (
            "just",
            "test",
            "-p",
            "codex-tui",
            "--",
            "--skip",
            "ide_context::ipc::tests::fetch_ide_context_uses_unregistered_request_route",
        ),
    ),
    (
        "memory-read-template-path",
        (
            "cargo",
            "build",
            "--manifest-path",
            "codex-rs/Cargo.toml",
            "-p",
            "codex-rmcp-client",
            "--bin",
            "test_stdio_server",
        ),
    ),
    ("memory-read-template-path", ("just", "test", "-p", "codex-core", "config")),
    ("memory-read-template-path", ("just", "test", "-p", "codex-memories-extension")),
    ("terminal-title-session-label", ("just", "test", "-p", "codex-tui", "terminal_title")),
    ("tui-history-image-previews", ("just", "test", "-p", "codex-core", "view_image")),
    (
        "tui-history-image-previews",
        (
            "just",
            "test",
            "-p",
            "codex-tui",
            "--",
            "--skip",
            "ide_context::ipc::tests::fetch_ide_context_uses_unregistered_request_route",
        ),
    ),
    ("tui-history-image-previews", ("just", "test", "-p", "codex-app-server-protocol")),
    ("tui-history-image-previews", ("just", "test", "-p", "codex-protocol")),
    (
        "tui-snapshots",
        ("cargo", "insta", "pending-snapshots", "--manifest-path", "codex-rs/tui/Cargo.toml"),
    ),
)


def find_repo_root(start: Path | None = None) -> Path:
    current = (start or Path.cwd()).resolve()
    for candidate in (current, *current.parents):
        if (candidate / ".git").exists() and (candidate / ".codex/skills/fork").exists():
            return candidate
    return DEFAULT_REPO_ROOT


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def command_env() -> dict[str, str]:
    env = dict(os.environ)
    home = env.get("HOME")
    if home:
        env["PATH"] = f"{home}/.cargo/bin:{home}/.local/bin:{env.get('PATH', '')}"
    return env


def timestamp() -> str:
    return datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")


def tail_lines(path: Path, count: int = 10) -> list[str]:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    return lines[-count:]


def shell_quote(argv: list[str] | tuple[str, ...]) -> str:
    return shlex.join(argv)


class LogSession:
    def __init__(
        self,
        *,
        repo_root: Path,
        log_kind: str,
        mode: str,
        version: str | None = None,
    ) -> None:
        self.repo_root = repo_root
        self.mode = mode
        log_dir = repo_root / "target/fork-migration" / f"{log_kind}-logs"
        log_dir.mkdir(parents=True, exist_ok=True)
        prefix = f"{version}-" if version else ""
        self.log_file = log_dir / f"{prefix}{mode}-{timestamp()}.log"
        self.log_file.write_text("", encoding="utf-8")

    def write(self, text: str) -> None:
        with self.log_file.open("a", encoding="utf-8") as log:
            log.write(text)

    def log_command(self, label: str, argv: list[str] | tuple[str, ...] | None = None) -> None:
        self.write(f"\n==> {label}\n")
        if argv:
            self.write(f"argv: {shell_quote(argv)}\n")

    def fail(
        self,
        *,
        label: str,
        exit_code: int = 1,
        argv: list[str] | tuple[str, ...] | None = None,
    ) -> int:
        print("RESULT: failed")
        print(f"MODE: {self.mode}")
        print(f"FAILED_STEP: {label}")
        print(f"LOG: {self.log_file}")
        print(f"EXIT_CODE: {exit_code}")
        if argv:
            print(f"ARGV: {shell_quote(argv)}")
        print()
        print("Last log lines:")
        for line in tail_lines(self.log_file):
            print(line)
        return exit_code

    def ok(self, extra: list[str] | None = None) -> int:
        print("RESULT: ok")
        print(f"MODE: {self.mode}")
        if extra:
            for line in extra:
                print(line)
        print(f"LOG: {self.log_file}")
        return 0

    def check_command(self, name: str) -> str | None:
        executable = shutil.which(name, path=command_env().get("PATH"))
        self.log_command(f"require command: {name}")
        if executable:
            self.write(f"found: {executable}\n")
            return executable
        self.write(f"required command not found: {name}\n")
        return None

    def run_step(self, label: str, argv: list[str]) -> int:
        print()
        print(f"==> {label}")
        self.log_command(label, argv)
        with self.log_file.open("a", encoding="utf-8") as log:
            result = subprocess.run(
                argv,
                cwd=self.repo_root,
                env=command_env(),
                stdout=log,
                stderr=subprocess.STDOUT,
                text=True,
                check=False,
            )

        if result.returncode == 0:
            print("OK")
            return 0
        return self.fail(label=label, exit_code=result.returncode, argv=argv)

    def run_capture(self, label: str, argv: list[str]) -> tuple[int, str, str]:
        self.log_command(label, argv)
        result = subprocess.run(
            argv,
            cwd=self.repo_root,
            env=command_env(),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
        self.write(result.stdout)
        self.write(result.stderr)
        return result.returncode, result.stdout, result.stderr

    def rg_no_match(self, label: str, pattern: str, args: list[str]) -> int:
        rg = self.check_command("rg")
        if not rg:
            return self.fail(label="require command: rg")

        print()
        print(f"==> {label}")
        argv = [rg, "-n", pattern, *args]
        status, stdout, stderr = self.run_capture(label, argv)
        if status == 1:
            print("OK")
            return 0
        if status == 0:
            if stdout:
                print(stdout, end="")
            return self.fail(label=label, argv=argv)
        if stderr:
            print(stderr, end="", file=sys.stderr)
        return self.fail(label=label, exit_code=status, argv=argv)


def markdownlint_config(repo_root: Path) -> Path:
    project_config = repo_root / "docs/.markdownlint-cli2.yaml"
    if project_config.exists():
        return project_config
    return Path.home() / ".codex/.markdownlint-cli2.yaml"


def skill_markdown_paths() -> list[Path]:
    return [SKILL_ROOT / rel for rel in SKILL_MARKDOWN]


def fork_doc_paths(repo_root: Path) -> list[Path]:
    docs_fork = repo_root / "docs/fork"
    if not docs_fork.exists():
        return []
    return sorted(docs_fork.glob("*.md"))


def current_branch(repo_root: Path) -> str:
    result = subprocess.run(
        ["git", "branch", "--show-current"],
        cwd=repo_root,
        env=command_env(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return ""
    return result.stdout.strip()


def infer_version(repo_root: Path) -> str | None:
    branch = current_branch(repo_root)
    match = re.fullmatch(r"hermione-(.+)", branch)
    if match:
        return match.group(1)

    migration_cards = sorted((repo_root / "docs/fork").glob("migration-*.md"))
    if len(migration_cards) == 1:
        name = migration_cards[0].stem
        return name.removeprefix("migration-")
    return None


def resolved_version(args: argparse.Namespace, repo_root: Path) -> str | None:
    return getattr(args, "version", None) or infer_version(repo_root)


def migration_card(repo_root: Path, version: str) -> Path:
    return repo_root / "docs/fork" / f"migration-{version}.md"


def check_branch(session: LogSession, version: str, skip_branch_check: bool) -> int:
    label = "branch check"
    print()
    print(f"==> {label}")
    session.log_command(label)
    expected = f"hermione-{version}"
    actual = current_branch(session.repo_root)
    session.write(f"expected: {expected}\nactual: {actual}\n")
    if skip_branch_check or actual == expected:
        print("OK")
        return 0
    return session.fail(label=label)


def check_migration_card_exists(session: LogSession, path: Path) -> int:
    label = "migration card exists"
    print()
    print(f"==> {label}")
    session.log_command(label)
    session.write(f"path: {path}\n")
    if path.exists():
        print("OK")
        return 0
    return session.fail(label=label)


def check_unfinished_migration_rows(session: LogSession, path: Path) -> int:
    statuses = "|".join(
        re.escape(status).replace(r"\ ", " ") for status in BLOCKING_MIGRATION_STATUSES
    )
    return session.rg_no_match(
        "unfinished migration rows",
        rf"\| [^|]*\.md[^|]* \| [^|]*({statuses})[^|]* \|",
        [str(path)],
    )


def check_reverted_card_row(session: LogSession, path: Path) -> int:
    label = "reverted card row"
    pattern = r"^\| [^|]*multi-agent-v2-task-depth\.md[^|]* \|"
    rg = session.check_command("rg")
    if not rg:
        return session.fail(label="require command: rg")

    print()
    print(f"==> {label}")
    status, stdout, stderr = session.run_capture(label, [rg, "-n", pattern, str(path)])
    if status == 1 or "reverted" in stdout:
        print("OK")
        return 0
    if status == 0:
        if stdout:
            print(stdout, end="")
        return session.fail(label=label)
    if stderr:
        print(stderr, end="", file=sys.stderr)
    return session.fail(label=label, exit_code=status)


def run_preconditions(
    *,
    session: LogSession,
    version: str | None,
    skip_branch_check: bool,
    require_migration_card: bool,
) -> int:
    for command in ("git", "rg", "just", "cargo"):
        if not session.check_command(command):
            return session.fail(label=f"require command: {command}")

    if not require_migration_card:
        return 0

    if not version:
        session.write("migration version could not be inferred\n")
        return session.fail(label="resolve migration version")

    card = migration_card(session.repo_root, version)
    for check in (
        lambda: check_migration_card_exists(session, card),
        lambda: check_branch(session, version, skip_branch_check),
        lambda: check_unfinished_migration_rows(session, card),
    ):
        result = check()
        if result != 0:
            return result
    return 0


def current_legacy_scripts(repo_root: Path) -> list[str]:
    scripts_dir = repo_root / "scripts/fork-migration"
    if not scripts_dir.exists():
        return []
    return sorted(str(path.relative_to(repo_root)) for path in scripts_dir.glob("*.sh"))


def legacy_script_rows(text: str) -> dict[str, tuple[str, str, str]]:
    rows = {}
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("| `scripts/fork-migration/"):
            continue

        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if len(cells) < 4:
            continue

        script = cells[0].strip("`")
        command = cells[1].strip("`")
        status = cells[2].strip("`")
        note = cells[3]
        rows[script] = (command, status, note)
    return rows


def heading_names(path: Path) -> list[str]:
    names = []
    for line in read_text(path).splitlines():
        if line.startswith("## "):
            names.append(line[3:].strip())
    return names


def has_heading(headings: list[str], aliases: tuple[str, ...]) -> bool:
    for heading in headings:
        for alias in aliases:
            if heading == alias or heading.startswith(f"{alias}:"):
                return True
    return False


def status_count(text: str, status: str) -> int:
    return len(re.findall(rf"\|\s*{re.escape(status)}\s*\|", text))


def cmd_check_source_coverage(args: argparse.Namespace) -> int:
    path = Path(args.path).resolve() if args.path else SOURCE_COVERAGE
    if not path.exists():
        print(f"ERROR: missing coverage file: {path}", file=sys.stderr)
        return 2

    repo_root = find_repo_root()
    text = read_text(path)
    counts = {status: status_count(text, status) for status in OPEN_STATUSES}
    placeholder_count = len(re.findall(r"\b(?:TODO|FIXME|TBD)\b", text))
    missing_targets = []

    for rel in REQUIRED_FILES:
        if not (SKILL_ROOT / rel).exists():
            missing_targets.append(rel)

    rows = legacy_script_rows(text)
    current_scripts = current_legacy_scripts(repo_root)
    missing_current_scripts = [script for script in current_scripts if script not in rows]
    missing_retired_scripts = [script for script in RETIRED_LEGACY_SCRIPTS if script not in rows]
    invalid_script_rows = []

    for script, (command, status, note) in rows.items():
        if status not in COVERAGE_ROW_STATUSES:
            invalid_script_rows.append(f"{script}: invalid status: {status}")
        if status != "не переносится" and command in ("", "-", "не переносится"):
            invalid_script_rows.append(f"{script}: missing skill command")
        if status == "не переносится" and len(note.strip()) < 20:
            invalid_script_rows.append(f"{script}: missing non-transfer rationale")

    print(f"COVERAGE: {path}")
    for status, count in counts.items():
        print(f"{status}: {count}")
    print(f"placeholder_markers: {placeholder_count}")
    print(f"missing_targets: {len(missing_targets)}")
    for rel in missing_targets:
        print(f"missing: {rel}")
    print(f"missing_current_legacy_scripts: {len(missing_current_scripts)}")
    for script in missing_current_scripts:
        print(f"missing_legacy_script_row: {script}")
    print(f"missing_retired_legacy_scripts: {len(missing_retired_scripts)}")
    for script in missing_retired_scripts:
        print(f"missing_retired_script_row: {script}")
    print(f"invalid_script_rows: {len(invalid_script_rows)}")
    for error in invalid_script_rows:
        print(f"invalid_script_row: {error}")

    if args.strict and (
        any(counts.values())
        or placeholder_count
        or missing_targets
        or missing_current_scripts
        or missing_retired_scripts
        or invalid_script_rows
    ):
        print("RESULT: blocked")
        return 1

    print("RESULT: ok" if not args.strict else "RESULT: structural-strict-ok")
    return 0


def cmd_render_subagent_prompt(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    card = Path(args.card)
    card_path = card if card.is_absolute() else repo_root / card
    if not args.allow_missing_card and not card_path.exists():
        print(f"ERROR: card not found: {card_path}", file=sys.stderr)
        return 2

    template_path = SKILL_ROOT / "assets/templates/parent-subagent-prompt.md"
    template = read_text(template_path)
    replacements = {
        "{{repo_root}}": str(repo_root),
        "{{branch}}": args.branch,
        "{{version}}": args.version,
        "{{card}}": args.card,
    }
    for needle, value in replacements.items():
        template = template.replace(needle, value)

    print(template, end="" if template.endswith("\n") else "\n")
    return 0


def first_heading(path: Path) -> str:
    for line in read_text(path).splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    return ""


def first_status(path: Path) -> str:
    for line in read_text(path).splitlines()[:80]:
        match = re.match(r"\s*(?:status|Статус):\s*`?([^`]+?)`?\s*$", line)
        if match:
            return match.group(1).strip()
    return ""


def cmd_cards_list(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    docs_fork = repo_root / "docs/fork"
    if not docs_fork.exists():
        print(f"ERROR: missing docs/fork directory: {docs_fork}", file=sys.stderr)
        return 2

    cards = sorted(docs_fork.glob("*.md"))
    if args.format == "tsv":
        print("path\tstatus\ttitle")
        for path in cards:
            rel = path.relative_to(repo_root)
            print(f"{rel}\t{first_status(path)}\t{first_heading(path)}")
    else:
        for path in cards:
            rel = path.relative_to(repo_root)
            print(f"- {rel} [{first_status(path)}] {first_heading(path)}")
    return 0


def card_validation_errors(path: Path) -> list[str]:
    text = read_text(path)
    errors = []
    if not first_heading(path):
        errors.append("missing top-level heading")

    status = first_status(path)
    if path.name.startswith("migration-") or status == "reverted":
        return errors

    headings = heading_names(path)
    for group_name, aliases in CARD_REQUIRED_SECTION_GROUPS:
        if not has_heading(headings, aliases):
            errors.append(f"missing owner-card section group: {group_name}")
    return errors


def cmd_cards_validate(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    cards = fork_doc_paths(repo_root)
    if not cards:
        print(f"ERROR: no fork cards found under {repo_root / 'docs/fork'}", file=sys.stderr)
        return 2

    failures = []
    for card in cards:
        errors = card_validation_errors(card)
        if errors:
            rel = card.relative_to(repo_root)
            for error in errors:
                failures.append(f"{rel}: {error}")

    for failure in failures:
        print(f"ERROR: {failure}", file=sys.stderr)

    print(f"cards_checked: {len(cards)}")
    print(f"card_errors: {len(failures)}")
    if failures:
        print("RESULT: blocked")
        return 1

    print("RESULT: ok")
    return 0


def run_markdownlint(paths: list[Path], session: LogSession) -> int:
    executable = session.check_command("markdownlint-cli2")
    if not executable:
        return session.fail(label="require command: markdownlint-cli2")

    config = markdownlint_config(session.repo_root)
    if not config.exists():
        session.write(f"markdownlint config not found: {config}\n")
        return session.fail(label="markdownlint config exists")

    if not paths:
        session.write("markdownlint paths: none\n")
        return 0

    return session.run_step(
        "markdownlint",
        [executable, "--config", str(config), *[str(path) for path in paths]],
    )


def cmd_preflight(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    scope = "skill" if args.skill_only else args.scope
    version = resolved_version(args, repo_root)
    session = LogSession(
        repo_root=repo_root,
        log_kind="preflight",
        mode="preflight",
        version=version if scope != "skill" else None,
    )

    failures = []
    session.write(f"scope: {scope}\n")
    session.write(f"version: {version or ''}\n")

    for rel in REQUIRED_FILES:
        if not (SKILL_ROOT / rel).exists():
            failures.append(f"missing required file: {rel}")

    for path in (SKILL_ROOT / "scripts").glob("*"):
        if path.is_file() and path.name != "fork_cli.py":
            text = read_text(path)
            if "scripts/fork-migration/" in text:
                failures.append(f"legacy runtime dependency in script: {path}")

    for failure in failures:
        session.write(f"{failure}\n")

    markdown_paths = skill_markdown_paths()
    marker_roots = [SKILL_ROOT]

    if scope in ("fork-docs", "all"):
        markdown_paths.extend(fork_doc_paths(repo_root))
        marker_roots = [repo_root]

    if failures:
        return session.fail(label="static skill checks")

    if scope in ("fork-docs", "all"):
        if not version:
            session.write("migration version could not be inferred\n")
            return session.fail(label="resolve migration version")

        card = migration_card(repo_root, version)
        for check in (
            lambda: check_branch(session, version, args.skip_branch_check),
            lambda: check_migration_card_exists(session, card),
            lambda: check_unfinished_migration_rows(session, card),
            lambda: check_reverted_card_row(session, card),
        ):
            result = check()
            if result != 0:
                return result

        result = session.run_step("git diff whitespace check", ["git", "diff", "--check"])
        if result != 0:
            return result

    result = session.rg_no_match(
        "conflict marker scan",
        r"^(<<<<<<<|>>>>>>>|=======$)",
        [
            "--glob",
            "!target/**",
            "--glob",
            "!codex-rs/target/**",
            "--glob",
            "!.git/**",
            "--glob",
            "!node_modules/**",
            *[str(path) for path in marker_roots],
        ],
    )
    if result != 0:
        return result

    existing_markdown_paths = [path for path in markdown_paths if path.exists()]
    result = run_markdownlint(existing_markdown_paths, session)
    if result != 0:
        return result

    coverage_args = argparse.Namespace(path=None, strict=args.strict_coverage)
    coverage_result = cmd_check_source_coverage(coverage_args)
    if coverage_result != 0:
        session.write("source coverage is not strict-clean\n")
        return session.fail(label="source coverage")

    return session.ok()


def cmd_format(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    if args.check == args.fix:
        print("ERROR: choose exactly one of --check or --fix", file=sys.stderr)
        return 2

    mode = "check" if args.check else "fix"
    session = LogSession(repo_root=repo_root, log_kind="format", mode=mode)
    just = session.check_command("just")
    if not just:
        return session.fail(label="require command: just")

    if args.check:
        result = session.run_step("format check", [just, "fmt-check"])
        if result != 0:
            return result
        return session.ok()

    result = session.run_step("format fix", [just, "fmt"])
    if result != 0:
        return result
    return session.ok()


def cmd_generators(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    session = LogSession(repo_root=repo_root, log_kind="generator", mode="generators")

    just = session.check_command("just")
    if not just:
        return session.fail(label="require command: just")
    if not session.check_command("cargo"):
        return session.fail(label="require command: cargo")

    for label, argv in (
        ("config schema", [just, "write-config-schema"]),
        ("app-server experimental schema", [just, "write-app-server-schema", "--experimental"]),
        ("app-server schema", [just, "write-app-server-schema"]),
    ):
        result = session.run_step(label, argv)
        if result != 0:
            return result

    return session.ok(
        [
            "EXPECTED_GENERATED_PATHS:",
            "codex-rs/core/config.schema.json",
            "codex-rs/app-server-protocol/schema/",
        ]
    )


def cmd_tests(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    version = resolved_version(args, repo_root)

    if args.mode == "list":
        for label, argv in CARD_TESTS:
            print(f"{label:<34} {shell_quote(argv)}")
        return 0

    session = LogSession(repo_root=repo_root, log_kind="test", mode=args.mode, version=version)
    result = run_preconditions(
        session=session,
        version=version,
        skip_branch_check=args.skip_branch_check,
        require_migration_card=True,
    )
    if result != 0:
        return result

    if args.mode == "cards":
        for label, argv in CARD_TESTS:
            result = session.run_step(label, list(argv))
            if result != 0:
                return result
        return session.ok()

    just = session.check_command("just")
    if not just:
        return session.fail(label="require command: just")
    for label, argv in (
        ("full test suite", [just, "test"]),
        ("tui pending snapshots", ["cargo", "insta", "pending-snapshots", "-p", "codex-tui"]),
    ):
        result = session.run_step(label, argv)
        if result != 0:
            return result
    return session.ok()


def cmd_build_fast(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    version = resolved_version(args, repo_root)
    session = LogSession(repo_root=repo_root, log_kind="build", mode="build-fast", version=version)

    result = run_preconditions(
        session=session,
        version=version,
        skip_branch_check=args.skip_branch_check,
        require_migration_card=True,
    )
    if result != 0:
        return result

    just = session.check_command("just")
    if not just:
        return session.fail(label="require command: just")
    file_cmd = session.check_command("file")
    if not file_cmd:
        return session.fail(label="require command: file")

    result = session.run_step("release-fast build", [just, "build-fast-release"])
    if result != 0:
        return result

    binary_path = repo_root / "codex-rs/target/release-fast/codex"
    if not binary_path.is_file() or not os.access(binary_path, os.X_OK):
        session.write(f"expected executable not found: {binary_path}\n")
        return session.fail(label="binary existence")

    for label, argv in (
        ("binary file metadata", [file_cmd, str(binary_path)]),
        ("binary version", [str(binary_path), "--version"]),
    ):
        result = session.run_step(label, argv)
        if result != 0:
            return result

    return session.ok([f"BINARY: {binary_path}"])


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="fork",
        description="Skill-owned CLI for the Codex fork workflow.",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    coverage = sub.add_parser("check-source-coverage")
    coverage.add_argument("--path")
    coverage.add_argument("--strict", action="store_true")
    coverage.set_defaults(func=cmd_check_source_coverage)

    render = sub.add_parser("render-subagent-prompt")
    render.add_argument("--repo-root")
    render.add_argument("--branch", required=True)
    render.add_argument("--version", required=True)
    render.add_argument("--card", required=True)
    render.add_argument("--allow-missing-card", action="store_true")
    render.set_defaults(func=cmd_render_subagent_prompt)

    cards = sub.add_parser("cards")
    cards_sub = cards.add_subparsers(dest="cards_command", required=True)
    cards_list = cards_sub.add_parser("list")
    cards_list.add_argument("--repo-root")
    cards_list.add_argument("--format", choices=("tsv", "markdown"), default="tsv")
    cards_list.set_defaults(func=cmd_cards_list)
    cards_validate = cards_sub.add_parser("validate")
    cards_validate.add_argument("--repo-root")
    cards_validate.set_defaults(func=cmd_cards_validate)

    preflight = sub.add_parser("preflight")
    preflight.add_argument("--skill-only", action="store_true")
    preflight.add_argument("--repo-root")
    preflight.add_argument("--version")
    preflight.add_argument("--skip-branch-check", action="store_true")
    preflight.add_argument(
        "--scope",
        choices=("skill", "fork-docs", "all"),
        default="all",
        help="Select markdown/check scope. --skill-only is an alias for --scope skill.",
    )
    preflight.add_argument("--strict-coverage", action="store_true")
    preflight.set_defaults(func=cmd_preflight)

    fmt = sub.add_parser("format")
    fmt.add_argument("--repo-root")
    fmt.add_argument("--check", action="store_true")
    fmt.add_argument("--fix", action="store_true")
    fmt.set_defaults(func=cmd_format)

    generators = sub.add_parser("generators")
    generators.add_argument("--repo-root")
    generators.set_defaults(func=cmd_generators)

    tests = sub.add_parser("tests")
    tests.add_argument("--mode", choices=("list", "cards", "full"), required=True)
    tests.add_argument("--repo-root")
    tests.add_argument("--version")
    tests.add_argument("--skip-branch-check", action="store_true")
    tests.set_defaults(func=cmd_tests)

    build_fast = sub.add_parser("build-fast")
    build_fast.add_argument("--repo-root")
    build_fast.add_argument("--version")
    build_fast.add_argument("--skip-branch-check", action="store_true")
    build_fast.set_defaults(func=cmd_build_fast)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)
