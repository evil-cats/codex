import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
from collections.abc import Mapping
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
SKILL_ROOT = SCRIPT_DIR.parent
DEFAULT_REPO_ROOT = SKILL_ROOT.parents[2]
SOURCE_COVERAGE = SKILL_ROOT / "references" / "source-coverage.md"
FORK_TESTS_SCHEMA = "fork-tests.v1"

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
    "scripts/fork_cli_tests.py",
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

TRANSFER_SECTION_ALIASES = (
    "Порядок повторения при переносе",
    "Пошаговое воспроизведение",
    "Пошаговое воспроизведение доработки",
    "Порядок реализации",
)

CHECKS_SECTION_ALIASES = (
    "Проверки",
    "Проверки для будущего переноса",
    "Исторические проверки",
    "Регрессионное покрытие",
    "Регрессионное покрытие в diff",
)

NORMATIVE_COMMAND_SECTION_GROUPS = (
    ("Порядок повторения при переносе", TRANSFER_SECTION_ALIASES),
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
        TRANSFER_SECTION_ALIASES,
    ),
    (
        "Проверки",
        CHECKS_SECTION_ALIASES,
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

STRICT_CHECK_SUBSECTIONS = (
    "Смысловое покрытие",
    "Владелец исполняемой карты",
    "Дополнительные gates",
    "Исторические результаты",
    "Известные падения и пропуски",
)

COMMAND_RUNBOOK_RE = re.compile(
    r"(?:^|\s)`?(?:just\s+\S+|cargo\s+\S+|"
    r"\.codex/skills/fork/scripts/fork\s+\S+|fork\s+\S+\s+--\S+)"
)

SOURCE_LIKE_SUFFIXES = (
    ".bazel",
    ".bzl",
    ".css",
    ".html",
    ".js",
    ".json",
    ".jsx",
    ".md",
    ".proto",
    ".py",
    ".rs",
    ".scss",
    ".sh",
    ".snap",
    ".sql",
    ".toml",
    ".ts",
    ".tsx",
    ".yaml",
    ".yml",
)

SOURCE_LIKE_NAMES = (
    "AGENTS.md",
    "BUILD",
    "BUILD.bazel",
    "Cargo.lock",
    "Cargo.toml",
    "Dockerfile",
    "Justfile",
    "MODULE.bazel",
    "README.md",
    "SKILL.md",
    "fork",
    "package-lock.json",
    "package.json",
    "pnpm-lock.yaml",
    "yarn.lock",
)

UNTRACKED_ARTIFACT_DIRS = (
    ".cache",
    ".git",
    ".mypy_cache",
    ".next",
    ".pytest_cache",
    ".ruff_cache",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
)

UNTRACKED_ARTIFACT_SUFFIXES = (
    ".d",
    ".dll",
    ".dylib",
    ".exe",
    ".gcda",
    ".gcno",
    ".log",
    ".o",
    ".profraw",
    ".pyc",
    ".pyo",
    ".rlib",
    ".rmeta",
    ".snap.new",
    ".so",
    ".stderr",
    ".stdout",
    ".temp",
    ".tmp",
)


@dataclass(frozen=True)
class CardTest:
    card_id: str
    argv: tuple[str, ...]
    purpose: str


def card_test(card_id: str, purpose: str, *argv: str) -> CardTest:
    return CardTest(card_id=card_id, purpose=purpose, argv=argv)


def fenced_json_blocks(text: str) -> list[tuple[int, str]]:
    blocks: list[tuple[int, str]] = []
    in_json_block = False
    block_start = 0
    block_lines: list[str] = []
    for line_number, line in enumerate(text.splitlines(), start=1):
        stripped = line.strip()
        if not in_json_block:
            if stripped == "```json":
                in_json_block = True
                block_start = line_number + 1
                block_lines = []
            continue

        if stripped == "```":
            blocks.append((block_start, "\n".join(block_lines)))
            in_json_block = False
            block_lines = []
            continue

        block_lines.append(line)
    return blocks


def card_tests_from_payload(
    path: Path, card_id: str, block_start: int, payload: object
) -> tuple[list[CardTest], list[str]]:
    block_label = f"{path}: fork-tests.v1 block at line {block_start}"
    if not isinstance(payload, dict):
        return [], [f"{block_label}: payload must be a JSON object"]

    schema = payload.get("schema")
    if schema != FORK_TESTS_SCHEMA:
        return [], [f"{block_label}: schema must be {FORK_TESTS_SCHEMA!r}"]

    tests_value = payload.get("tests")
    if not isinstance(tests_value, list):
        return [], [f"{block_label}: `tests` must be a JSON array"]

    tests: list[CardTest] = []
    errors: list[str] = []
    seen_purposes: set[str] = set()
    for index, entry in enumerate(tests_value, start=1):
        entry_label = f"{block_label}: tests[{index}]"
        if not isinstance(entry, dict):
            errors.append(f"{entry_label}: entry must be a JSON object")
            continue

        raw_purpose = entry.get("purpose")
        argv = entry.get("argv")
        entry_errors: list[str] = []
        if not isinstance(raw_purpose, str) or not raw_purpose.strip():
            entry_errors.append(f"{entry_label}: `purpose` must be a non-empty string")
            purpose = ""
        else:
            purpose = raw_purpose.strip()

        if purpose and purpose in seen_purposes:
            entry_errors.append(f"{entry_label}: duplicate purpose {purpose!r}")

        if not isinstance(argv, list) or not argv:
            entry_errors.append(f"{entry_label}: `argv` must be a non-empty array")
        elif not all(isinstance(arg, str) and arg for arg in argv):
            entry_errors.append(f"{entry_label}: `argv` entries must be non-empty strings")

        if entry_errors:
            errors.extend(entry_errors)
            continue

        assert isinstance(argv, list)
        seen_purposes.add(purpose)
        tests.append(card_test(card_id, purpose, *argv))

    return tests, errors


def card_tests_in_card(path: Path) -> tuple[list[CardTest], list[str]]:
    text = read_text(path)
    matching_blocks = [
        (block_start, raw)
        for block_start, raw in fenced_json_blocks(text)
        if FORK_TESTS_SCHEMA in raw
    ]
    if not matching_blocks:
        return [], []

    card_id = first_id(path)
    if not card_id:
        return [], [f"{path}: fork-tests.v1 block exists but frontmatter id is missing"]

    tests: list[CardTest] = []
    errors: list[str] = []
    for block_start, raw in matching_blocks:
        try:
            payload = json.loads(raw)
        except json.JSONDecodeError as exc:
            errors.append(
                f"{path}: invalid fork-tests.v1 JSON block at line "
                f"{block_start}: {exc.msg}"
            )
            continue

        block_tests, block_errors = card_tests_from_payload(
            path, card_id, block_start, payload
        )
        tests.extend(block_tests)
        errors.extend(block_errors)

    return tests, errors


def card_tests(
    repo_root: Path, *, active_only: bool = True
) -> tuple[list[CardTest], list[str]]:
    tests: list[CardTest] = []
    errors: list[str] = []
    for path in fork_doc_paths(repo_root):
        if path.name.startswith("migration-"):
            continue
        if active_only and first_status(path) != "active":
            continue
        card_tests_for_path, path_errors = card_tests_in_card(path)
        tests.extend(card_tests_for_path)
        errors.extend(path_errors)
    return tests, errors


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


def default_install_target(env: Mapping[str, str] | None = None) -> Path:
    source_env = os.environ if env is None else env
    home = source_env.get("HOME")
    if not home:
        raise ValueError("HOME must be set to resolve the default install target")
    return Path(home) / ".local/bin/codex-hermione"


def resolve_path_arg(value: str, repo_root: Path) -> Path:
    expanded = Path(os.path.expanduser(os.path.expandvars(value)))
    if expanded.is_absolute():
        return expanded
    return repo_root / expanded


def install_temp_path(target: Path) -> Path:
    return target.with_name(f"{target.name}.new")


def timestamp() -> str:
    return datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")


def tail_lines(path: Path, count: int = 10) -> list[str]:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    return lines[-count:]


def shell_quote(argv: list[str] | tuple[str, ...]) -> str:
    return shlex.join(argv)


def git_untracked_paths(
    repo_root: Path, pathspecs: list[str]
) -> tuple[int, list[Path], str]:
    argv = ["git", "status", "--porcelain=v1", "-z", "--untracked-files=all"]
    if pathspecs:
        argv.extend(["--", *pathspecs])

    result = subprocess.run(
        argv,
        cwd=repo_root,
        env=command_env(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    paths = [
        Path(entry[3:])
        for entry in result.stdout.split("\0")
        if entry.startswith("?? ")
    ]
    return result.returncode, paths, result.stderr


def is_untracked_artifact(path: Path) -> bool:
    if any(part in UNTRACKED_ARTIFACT_DIRS for part in path.parts):
        return True
    return str(path).endswith(UNTRACKED_ARTIFACT_SUFFIXES)


def is_source_like_untracked(path: Path) -> bool:
    if is_untracked_artifact(path):
        return False
    return path.name in SOURCE_LIKE_NAMES or path.suffix in SOURCE_LIKE_SUFFIXES


def preflight_untracked_pathspecs(repo_root: Path, scope: str) -> list[str]:
    if scope == "skill":
        try:
            return [str(SKILL_ROOT.relative_to(repo_root))]
        except ValueError:
            return [str(SKILL_ROOT)]
    return []


def check_source_like_untracked(session: "LogSession", scope: str) -> int:
    pathspecs = preflight_untracked_pathspecs(session.repo_root, scope)
    session.log_command(
        "source-like untracked files",
        ["git", "status", "--porcelain=v1", "-z", "--untracked-files=all"]
        + (["--", *pathspecs] if pathspecs else []),
    )

    result, paths, stderr = git_untracked_paths(session.repo_root, pathspecs)
    if result != 0:
        if stderr:
            session.write(stderr)
        return session.fail(label="git status untracked files", exit_code=result)

    source_like = sorted(path for path in paths if is_source_like_untracked(path))
    ignored = sorted(path for path in paths if is_untracked_artifact(path))

    if ignored:
        session.write("ignored artifact-like untracked files:\n")
        for path in ignored:
            session.write(f"- {path}\n")

    if not source_like:
        session.write("source-like untracked files: none\n")
        return 0

    session.write("source-like untracked files require explicit handling:\n")
    for path in source_like:
        session.write(f"- {path}\n")
    session.write("\n")
    session.write("If a file is task-owned source, run `git add -N <path>`.\n")
    session.write(
        "Do not add build artifacts, logs, cache, temporary output, or unrelated files.\n"
    )
    session.write(
        "If a source-like file is unrelated, classify it explicitly before finalizing.\n"
    )
    return session.fail(label="source-like untracked files")


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


def section_text(text: str, heading: str, level: int = 2) -> str:
    marker = "#" * level
    next_marker = "#" * level
    pattern = re.compile(
        rf"^{re.escape(marker)}\s+{re.escape(heading)}\s*$",
        re.MULTILINE,
    )
    match = pattern.search(text)
    if not match:
        return ""

    start = match.end()
    next_match = re.search(rf"^{re.escape(next_marker)}\s+", text[start:], re.MULTILINE)
    end = start + next_match.start() if next_match else len(text)
    return text[start:end]


def section_text_for_aliases(
    text: str, aliases: tuple[str, ...], level: int = 2
) -> tuple[str, str]:
    marker = "#" * level
    pattern = re.compile(rf"^{re.escape(marker)}\s+(.+?)\s*$", re.MULTILINE)
    for match in pattern.finditer(text):
        heading = match.group(1).strip()
        if not has_heading([heading], aliases):
            continue

        start = match.end()
        next_match = re.search(rf"^{re.escape(marker)}\s+", text[start:], re.MULTILINE)
        end = start + next_match.start() if next_match else len(text)
        return heading, text[start:end]
    return "", ""


def subsection_text(section: str, heading: str) -> str:
    return section_text(section, heading, level=3)


def remove_subsection(section: str, heading: str) -> str:
    marker = "###"
    pattern = re.compile(
        rf"^{re.escape(marker)}\s+{re.escape(heading)}\s*$",
        re.MULTILINE,
    )
    match = pattern.search(section)
    if not match:
        return section

    next_match = re.search(rf"^{re.escape(marker)}\s+", section[match.end() :], re.MULTILINE)
    end = match.end() + next_match.start() if next_match else len(section)
    return section[: match.start()] + section[end:]


def command_runbook_errors(section_name: str, section: str) -> list[str]:
    errors = []
    if re.search(r"^```(?:bash|sh)\s*$", section, re.MULTILINE):
        errors.append(f"runbook command block in `{section_name}`")
    for line_number, line in enumerate(section.splitlines(), start=1):
        if COMMAND_RUNBOOK_RE.search(line):
            errors.append(
                f"runbook command leakage in `{section_name}` at section line "
                f"{line_number}: {line.strip()}"
            )
    return errors


def card_test_ids(tests: list[CardTest]) -> set[str]:
    return {test.card_id for test in tests}


def add_card_alias(aliases: dict[str, str], alias: str, card_id: str) -> None:
    alias = alias.strip()
    if alias:
        aliases.setdefault(alias, card_id)


def card_test_aliases(repo_root: Path, tests: list[CardTest]) -> dict[str, str]:
    aliases: dict[str, str] = {}
    for card_id in sorted(card_test_ids(tests)):
        add_card_alias(aliases, card_id, card_id)
        if card_id.startswith("fork-"):
            add_card_alias(aliases, card_id.removeprefix("fork-"), card_id)

    for path in fork_doc_paths(repo_root):
        card_id = first_id(path)
        if not card_id:
            continue

        add_card_alias(aliases, card_id, card_id)
        if card_id.startswith("fork-"):
            add_card_alias(aliases, card_id.removeprefix("fork-"), card_id)
        add_card_alias(aliases, path.name, card_id)
        add_card_alias(aliases, path.stem, card_id)
        add_card_alias(aliases, str(path), card_id)
        try:
            add_card_alias(aliases, str(path.relative_to(repo_root)), card_id)
        except ValueError:
            pass

    return aliases


def available_card_test_ids(tests: list[CardTest]) -> str:
    return "\n".join(f"  {card_id}" for card_id in sorted(card_test_ids(tests)))


def card_tests_for_filters(
    repo_root: Path, card_filters: list[str] | None
) -> tuple[list[CardTest], str | None]:
    all_tests, load_errors = card_tests(repo_root)
    if load_errors:
        return [], "\n".join(load_errors)

    if not card_filters:
        return all_tests, None

    aliases = card_test_aliases(repo_root, all_tests)
    requested: list[str] = []
    unknown: list[str] = []
    for raw_filter in card_filters:
        value = raw_filter.strip()
        card_id = aliases.get(value)
        if not card_id:
            path = Path(value)
            if not path.is_absolute():
                path = repo_root / path
            if path.exists():
                card_id = first_id(path)

        if card_id:
            requested.append(card_id)
        else:
            unknown.append(raw_filter)

    if unknown:
        return (
            [],
            "unknown card filter(s): "
            + ", ".join(unknown)
            + "\nAvailable card ids with tests:\n"
            + available_card_test_ids(all_tests),
        )

    requested_ids = set(requested)
    tests = [test for test in all_tests if test.card_id in requested_ids]
    tested_ids = {test.card_id for test in tests}
    missing = sorted(requested_ids - tested_ids)
    if missing:
        return (
            [],
            "card(s) have no fork-tests.v1 entries: "
            + ", ".join(missing)
            + "\nAvailable card ids with tests:\n"
            + available_card_test_ids(all_tests),
        )

    return tests, None


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


def first_id(path: Path) -> str:
    for line in read_text(path).splitlines()[:80]:
        match = re.match(r"\s*id:\s*`?([^`]+?)`?\s*$", line)
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


def strict_card_validation_errors(path: Path) -> list[str]:
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

    if status != "active":
        return errors

    card_id = first_id(path)
    if not card_id:
        errors.append("active card missing frontmatter id")

    checks_heading, checks = section_text_for_aliases(text, CHECKS_SECTION_ALIASES)
    if not checks:
        errors.append("active card missing strict `Проверки` section")
        return errors

    for subsection in STRICT_CHECK_SUBSECTIONS:
        if not subsection_text(checks, subsection):
            errors.append(f"active card missing strict `Проверки` subsection: {subsection}")

    owner = subsection_text(checks, "Владелец исполняемой карты")
    has_fork_tests_owner = "`fork tests`" in owner or "fork tests" in owner
    has_manual_exception = "`manual-required`" in owner or "`not-applicable`" in owner
    card_level_tests, card_test_errors = card_tests_in_card(path)
    errors.extend(card_test_errors)
    has_card_tests = bool(card_level_tests)
    if has_card_tests and not has_fork_tests_owner:
        errors.append(
            "fork-tests.v1 block exists but `Владелец исполняемой карты` "
            "does not name `fork tests`"
        )
    if not has_card_tests and not has_manual_exception:
        errors.append(
            "active card has no fork-tests.v1 block and no "
            "`manual-required`/`not-applicable` exception"
        )

    normative_checks = remove_subsection(checks, "Исторические результаты")
    for error in command_runbook_errors(checks_heading, normative_checks):
        errors.append(f"{error} outside `Исторические результаты`")

    for group_name, aliases in NORMATIVE_COMMAND_SECTION_GROUPS:
        heading, section = section_text_for_aliases(text, aliases)
        if not section:
            continue
        for error in command_runbook_errors(heading or group_name, section):
            errors.append(error)
    return errors


def cmd_cards_validate(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    cards = fork_doc_paths(repo_root)
    if not cards:
        print(f"ERROR: no fork cards found under {repo_root / 'docs/fork'}", file=sys.stderr)
        return 2

    failures = []
    active_card_ids = set()
    for card in cards:
        if not card.name.startswith("migration-") and first_status(card) == "active":
            card_id = first_id(card)
            if card_id:
                active_card_ids.add(card_id)

        errors = strict_card_validation_errors(card)
        if errors:
            rel = card.relative_to(repo_root)
            for error in errors:
                failures.append(f"{rel}: {error}")

    all_tests, _ = card_tests(repo_root, active_only=False)
    for test_id in sorted(card_test_ids(all_tests) - active_card_ids):
        failures.append(f"fork-tests.v1: label has no active fork card: {test_id}")

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

    result = check_source_like_untracked(session, scope)
    if result != 0:
        return result

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

    if scope in ("skill", "all"):
        result = session.run_step(
            "fork cli unit tests",
            [sys.executable, str(SKILL_ROOT / "scripts/fork_cli_tests.py")],
        )
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
    card_filters = args.card or []

    if card_filters and args.mode == "full":
        print(
            "ERROR: --card is only supported with --mode list or --mode cards",
            file=sys.stderr,
        )
        return 2

    selected_tests, filter_error = card_tests_for_filters(repo_root, card_filters)
    if filter_error:
        print(f"ERROR: {filter_error}", file=sys.stderr)
        return 2

    if args.mode == "list":
        for test in selected_tests:
            print(f"{test.card_id:<36} {test.purpose:<24} {shell_quote(test.argv)}")
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
        for test in selected_tests:
            result = session.run_step(f"{test.card_id}: {test.purpose}", list(test.argv))
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


def cmd_install(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    session = LogSession(repo_root=repo_root, log_kind="install", mode="install")

    source_path = (
        resolve_path_arg(args.source, repo_root)
        if args.source
        else repo_root / "codex-rs/target/release-fast/codex"
    )
    try:
        target_path = (
            resolve_path_arg(args.target, repo_root) if args.target else default_install_target()
        )
    except ValueError as exc:
        session.write(f"{exc}\n")
        return session.fail(label="resolve install target")

    temp_path = install_temp_path(target_path)
    session.write(f"source: {source_path}\n")
    session.write(f"target: {target_path}\n")
    session.write(f"temporary: {temp_path}\n")

    file_cmd = session.check_command("file")
    if not file_cmd:
        return session.fail(label="require command: file")

    if not source_path.is_file():
        session.write(f"source binary not found: {source_path}\n")
        return session.fail(label="source binary exists")
    if not os.access(source_path, os.X_OK):
        session.write(f"source binary is not executable: {source_path}\n")
        return session.fail(label="source binary executable")

    if source_path.resolve() == target_path.resolve(strict=False):
        session.write("source and target resolve to the same path\n")
        return session.fail(label="source target distinct")
    if source_path.resolve() == temp_path.resolve(strict=False):
        session.write("source and temporary path resolve to the same path\n")
        return session.fail(label="source temporary distinct")

    for label, argv in (
        ("source binary metadata", [file_cmd, str(source_path)]),
        ("source binary version", [str(source_path), "--version"]),
    ):
        result = session.run_step(label, argv)
        if result != 0:
            return result

    try:
        target_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source_path, temp_path)
        temp_path.chmod(0o755)
    except OSError as exc:
        session.write(f"failed to copy temporary binary: {exc}\n")
        return session.fail(label="copy temporary binary")

    for label, argv in (
        ("temporary binary metadata", [file_cmd, str(temp_path)]),
        ("temporary binary version", [str(temp_path), "--version"]),
    ):
        result = session.run_step(label, argv)
        if result != 0:
            return result

    try:
        os.replace(temp_path, target_path)
    except OSError as exc:
        session.write(f"failed to replace installed binary: {exc}\n")
        return session.fail(label="replace installed binary")

    for label, argv in (
        ("installed binary metadata", [file_cmd, str(target_path)]),
        ("installed binary version", [str(target_path), "--version"]),
    ):
        result = session.run_step(label, argv)
        if result != 0:
            return result

    return session.ok([f"SOURCE: {source_path}", f"TARGET: {target_path}"])


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
    tests.add_argument(
        "--card",
        action="append",
        metavar="CARD",
        help=(
            "Limit --mode list/cards to a card id, docs/fork/*.md path, file name, "
            "or id without the fork- prefix. Can be repeated."
        ),
    )
    tests.set_defaults(func=cmd_tests)

    build_fast = sub.add_parser("build-fast")
    build_fast.add_argument("--repo-root")
    build_fast.add_argument("--version")
    build_fast.add_argument("--skip-branch-check", action="store_true")
    build_fast.set_defaults(func=cmd_build_fast)

    install = sub.add_parser("install")
    install.add_argument("--repo-root")
    install.add_argument(
        "--source",
        help="Binary to install. Defaults to codex-rs/target/release-fast/codex.",
    )
    install.add_argument(
        "--target",
        help="Install target. Defaults to ${HOME}/.local/bin/codex-hermione.",
    )
    install.set_defaults(func=cmd_install)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)
