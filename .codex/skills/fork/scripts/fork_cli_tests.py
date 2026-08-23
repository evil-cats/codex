"""Регрессионные тесты публичного поведения skill-owned fork CLI.

Тесты используют только временные checkout-структуры и локальные fixtures: они
не меняют рабочий репозиторий, не запускают сеть, сборку или Git-операции. Здесь
проверяются parser contracts, fork-карточки, JSON migration map и безопасные
ошибки лёгких CLI-команд; тяжёлые workflow gates остаются интеграционными
командами самого skill.
"""

import importlib.util
import os
import sys
import tempfile
import textwrap
import unittest
import unittest.mock
from pathlib import Path


SCRIPT_PATH = Path(__file__).with_name("fork_cli.py")
sys.path.insert(0, str(SCRIPT_PATH.parent))
SPEC = importlib.util.spec_from_file_location("fork_cli", SCRIPT_PATH)
assert SPEC is not None
fork_cli = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(fork_cli)


class RecordingLogSession:
    instances = []

    def __init__(self, **_kwargs) -> None:
        self.steps = []
        self.env_overrides = []
        self.output = []
        self.ok_extra = None
        RecordingLogSession.instances.append(self)

    def write(self, text: str) -> None:
        self.output.append(text)

    def check_command(self, _name: str) -> str:
        return "file"

    def run_step(
        self,
        label: str,
        argv: list[str],
        *,
        env_overrides: dict[str, str] | None = None,
    ) -> int:
        self.steps.append((label, argv))
        self.env_overrides.append(env_overrides)
        return 0

    def fail(
        self,
        *,
        label: str,
        exit_code: int = 1,
        argv: list[str] | tuple[str, ...] | None = None,
    ) -> int:
        self.output.append(f"failed: {label}: {argv}")
        return exit_code

    def ok(self, extra: list[str] | None = None) -> int:
        self.ok_extra = extra
        return 0


FORK_TESTS_BLOCK = textwrap.dedent(
    """\
    ```json
    {
      "schema": "fork-tests.v1",
      "tests": [
        {
          "purpose": "runtime contract",
          "argv": ["just", "test", "-p", "codex-core", "read_file"]
        },
        {
          "purpose": "tool visibility",
          "argv": [
            "just",
            "test",
            "-p",
            "codex-core",
            "environment_count_controls_environment_backed_tools"
          ]
        }
      ]
    }
    ```
    """
)


class ReleaseFastWorkflowTests(unittest.TestCase):
    def setUp(self) -> None:
        RecordingLogSession.instances.clear()

    def test_default_install_target_uses_home_local_bin(self) -> None:
        self.assertEqual(
            fork_cli.default_install_target({"HOME": "/home/slader"}),
            Path("/home/slader/.local/bin/codex-hermione"),
        )

    def test_default_install_target_requires_home(self) -> None:
        with self.assertRaisesRegex(ValueError, "HOME must be set"):
            fork_cli.default_install_target({})

    def test_install_temp_path_appends_new_suffix(self) -> None:
        self.assertEqual(
            fork_cli.install_temp_path(Path("/home/slader/.local/bin/codex-hermione")),
            Path("/home/slader/.local/bin/codex-hermione.new"),
        )

    def test_rustc_host_target_reads_verbose_version_output(self) -> None:
        self.assertEqual(
            fork_cli.rustc_host_target(
                "rustc 1.89.0\nbinary: rustc\nhost: x86_64-unknown-linux-gnu\n"
            ),
            "x86_64-unknown-linux-gnu",
        )

    def test_rustc_host_target_requires_host_line(self) -> None:
        with self.assertRaisesRegex(ValueError, "does not contain a host target"):
            fork_cli.rustc_host_target("rustc 1.89.0\nbinary: rustc\n")

    def test_codex_v8_cargo_env_scopes_repo_root_to_package_imports(self) -> None:
        """Передаёт package imports правильный root и восстанавливает process state."""
        repo_root = Path("/repo")
        host_target = "x86_64-unknown-linux-gnu"
        target_spec = object()
        observed_repo_roots: list[str | None] = []
        real_import = __import__

        def import_with_observed_repo_root(
            name: str,
            globals_: dict | None = None,
            locals_: dict | None = None,
            fromlist: tuple[str, ...] = (),
            level: int = 0,
        ) -> object:
            if name == "scripts.codex_package.targets":
                observed_repo_roots.append(os.environ.get("CODEX_REPO_ROOT"))
                return unittest.mock.Mock(TARGET_SPECS={host_target: target_spec})
            if name == "scripts.codex_package.v8":
                observed_repo_roots.append(os.environ.get("CODEX_REPO_ROOT"))
                return unittest.mock.Mock(
                    resolve_codex_v8_cargo_env=lambda spec: {
                        "TARGET_SPEC_MATCHED": str(spec is target_spec)
                    }
                )
            return real_import(name, globals_, locals_, fromlist, level)

        session = unittest.mock.Mock()
        session.check_command.return_value = "rustc"
        session.run_capture.return_value = (
            0,
            f"rustc 1.89.0\nbinary: rustc\nhost: {host_target}\n",
            "",
        )
        previous_path_count = sys.path.count(str(repo_root))
        with (
            unittest.mock.patch.dict(
                os.environ, {"CODEX_REPO_ROOT": "/previous/repo"}, clear=False
            ),
            unittest.mock.patch(
                "builtins.__import__", side_effect=import_with_observed_repo_root
            ),
        ):
            status, cargo_env = fork_cli.resolve_codex_v8_cargo_env_for_host(
                session, repo_root
            )

            self.assertEqual(status, 0)
            self.assertEqual(cargo_env, {"TARGET_SPEC_MATCHED": "True"})
            self.assertEqual(observed_repo_roots, [str(repo_root), str(repo_root)])
            self.assertEqual(os.environ["CODEX_REPO_ROOT"], "/previous/repo")
            self.assertEqual(sys.path.count(str(repo_root)), previous_path_count)

    def test_install_parser_accepts_source_and_target(self) -> None:
        parser = fork_cli.build_parser()
        args = parser.parse_args(
            [
                "install",
                "--source",
                "codex-rs/target/release-fast/codex",
                "--target",
                "/tmp/codex-hermione",
            ]
        )

        self.assertEqual(args.command, "install")
        self.assertEqual(args.source, "codex-rs/target/release-fast/codex")
        self.assertEqual(args.target, "/tmp/codex-hermione")

    def test_release_fast_artifacts_include_main_and_code_mode_host(self) -> None:
        repo_root = Path("/repo")

        self.assertEqual(
            fork_cli.release_fast_binary_artifacts(repo_root, "posix"),
            (
                fork_cli.BinaryArtifact(
                    contract=fork_cli.RELEASE_FAST_BINARY_CONTRACTS[0],
                    path=Path("/repo/codex-rs/target/release-fast/codex"),
                ),
                fork_cli.BinaryArtifact(
                    contract=fork_cli.RELEASE_FAST_BINARY_CONTRACTS[1],
                    path=Path(
                        "/repo/codex-rs/target/release-fast/codex-code-mode-host"
                    ),
                ),
            ),
        )

    def test_install_artifacts_derive_code_mode_host_siblings(self) -> None:
        self.assertEqual(
            fork_cli.install_binary_artifacts(
                Path("/build/codex"),
                Path("/bin/codex-hermione"),
                "posix",
            ),
            (
                fork_cli.InstallArtifact(
                    contract=fork_cli.RELEASE_FAST_BINARY_CONTRACTS[0],
                    source=Path("/build/codex"),
                    target=Path("/bin/codex-hermione"),
                ),
                fork_cli.InstallArtifact(
                    contract=fork_cli.RELEASE_FAST_BINARY_CONTRACTS[1],
                    source=Path("/build/codex-code-mode-host"),
                    target=Path("/bin/codex-code-mode-host"),
                ),
            ),
        )

    def test_install_stages_both_artifacts_and_replaces_main_last(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            source_dir = repo_root / "build"
            source_dir.mkdir()
            main_source = source_dir / "codex"
            host_source = source_dir / "codex-code-mode-host"
            main_source.write_text("main", encoding="utf-8")
            host_source.write_text("host", encoding="utf-8")
            main_source.chmod(0o755)
            host_source.chmod(0o755)
            main_target = repo_root / "install/codex-hermione"
            host_target = main_target.with_name("codex-code-mode-host")
            replacements = []
            real_replace = os.replace

            def recording_replace(source: Path, target: Path) -> None:
                replacements.append((Path(source), Path(target)))
                real_replace(source, target)

            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(repo_root),
                    "--source",
                    str(main_source),
                    "--target",
                    str(main_target),
                ]
            )
            with (
                unittest.mock.patch.object(
                    fork_cli, "LogSession", RecordingLogSession
                ),
                unittest.mock.patch.object(
                    fork_cli.os, "replace", side_effect=recording_replace
                ),
            ):
                result = fork_cli.cmd_install(args)

            self.assertEqual(result, 0)
            self.assertEqual(main_target.read_text(encoding="utf-8"), "main")
            self.assertEqual(host_target.read_text(encoding="utf-8"), "host")
            self.assertEqual(
                [target for _source, target in replacements],
                [host_target, main_target],
            )
            self.assertIn(
                (
                    "Code Mode host source binary probe",
                    [str(host_source), "--help"],
                ),
                RecordingLogSession.instances[0].steps,
            )

    def test_install_requires_host_before_replacing_main(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            main_source = repo_root / "codex"
            main_source.write_text("new main", encoding="utf-8")
            main_source.chmod(0o755)
            main_target = repo_root / "install/codex-hermione"
            main_target.parent.mkdir()
            main_target.write_text("old main", encoding="utf-8")
            main_target.chmod(0o755)
            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(repo_root),
                    "--source",
                    str(main_source),
                    "--target",
                    str(main_target),
                ]
            )

            with unittest.mock.patch.object(
                fork_cli, "LogSession", RecordingLogSession
            ):
                result = fork_cli.cmd_install(args)

            self.assertEqual(result, 1)
            self.assertEqual(main_target.read_text(encoding="utf-8"), "old main")


class FixCommandTests(unittest.TestCase):
    def setUp(self) -> None:
        RecordingLogSession.instances.clear()

    def test_fix_parser_accepts_repeated_packages(self) -> None:
        parser = fork_cli.build_parser()
        args = parser.parse_args(
            [
                "fix",
                "--package",
                "codex-core",
                "--package",
                "codex-goal-extension",
            ]
        )

        self.assertEqual(args.command, "fix")
        self.assertEqual(
            args.package,
            ["codex-core", "codex-goal-extension"],
        )

    def test_fix_argv_uses_workspace_by_default(self) -> None:
        self.assertEqual(
            fork_cli.fix_argv("/usr/bin/just", []),
            ["/usr/bin/just", "fix"],
        )

    def test_fix_argv_forwards_each_package(self) -> None:
        self.assertEqual(
            fork_cli.fix_argv(
                "/usr/bin/just",
                ["codex-core", "codex-goal-extension"],
            ),
            [
                "/usr/bin/just",
                "fix",
                "-p",
                "codex-core",
                "-p",
                "codex-goal-extension",
            ],
        )

    def test_fix_passes_codex_v8_environment_to_lint_recipe(self) -> None:
        """Передаёт разрешённые Codex V8 artifacts в запускаемый `just fix`."""
        cargo_env = {
            "RUSTY_V8_ARCHIVE": "/cache/v8.a",
            "RUSTY_V8_SRC_BINDING_PATH": "/cache/src_binding.rs",
        }
        args = unittest.mock.Mock(repo_root="/repo", package=None)

        with (
            unittest.mock.patch.object(
                fork_cli, "LogSession", RecordingLogSession
            ),
            unittest.mock.patch.object(
                fork_cli,
                "resolve_codex_v8_cargo_env_for_host",
                return_value=(0, cargo_env),
            ),
        ):
            result = fork_cli.cmd_fix(args)

        self.assertEqual(result, 0)
        session = RecordingLogSession.instances[-1]
        self.assertEqual(session.steps, [("rust lint fix", ["file", "fix"])])
        self.assertEqual(session.env_overrides, [cargo_env])


class CardTestFilterTests(unittest.TestCase):
    def make_repo(self) -> Path:
        temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(temp_dir.cleanup)

        repo_root = Path(temp_dir.name)
        docs_fork = repo_root / "docs/fork"
        docs_fork.mkdir(parents=True)
        (docs_fork / "core-read-file-tool.md").write_text(
            (
                "---\n"
                "id: fork-core-read-file-tool\n"
                "status: active\n"
                "---\n"
                "# Read file\n"
                "\n"
                "## Проверки\n"
                "\n"
                "### Владелец исполняемой карты\n"
                "\n"
                "`fork tests` владеет запуском.\n"
                "\n"
                f"{FORK_TESTS_BLOCK}"
            ),
            encoding="utf-8",
        )
        (docs_fork / "planned-only.md").write_text(
            "---\nid: fork-planned-only\nstatus: planned\n---\n# Planned only\n",
            encoding="utf-8",
        )
        (docs_fork / "planned-with-tests.md").write_text(
            (
                "---\n"
                "id: fork-planned-with-tests\n"
                "status: planned\n"
                "---\n"
                "# Planned with tests\n"
                "\n"
                f"{FORK_TESTS_BLOCK}"
            ),
            encoding="utf-8",
        )
        (docs_fork / "manual-only.md").write_text(
            (
                "---\n"
                "id: fork-manual-only\n"
                "status: active\n"
                "---\n"
                "# Manual only\n"
                "\n"
                "## Проверки\n"
                "\n"
                "`manual-required`: Поведение подтверждается интерактивно\n"
                "в TUI.\n"
            ),
            encoding="utf-8",
        )
        (docs_fork / "not-applicable-only.md").write_text(
            (
                "---\n"
                "id: fork-not-applicable-only\n"
                "status: active\n"
                "---\n"
                "# Not applicable only\n"
                "\n"
                "## Проверки\n"
                "\n"
                "`not-applicable`: Отдельного card-level test нет.\n"
            ),
            encoding="utf-8",
        )
        return repo_root

    def test_filters_by_card_id(self) -> None:
        tests, error = fork_cli.card_tests_for_filters(
            self.make_repo(), ["fork-core-read-file-tool"]
        )

        self.assertIsNone(error)
        self.assertEqual(
            [test.purpose for test in tests],
            ["runtime contract", "tool visibility"],
        )

    def test_filters_by_card_path(self) -> None:
        tests, error = fork_cli.card_tests_for_filters(
            self.make_repo(), ["docs/fork/core-read-file-tool.md"]
        )

        self.assertIsNone(error)
        self.assertEqual({test.card_id for test in tests}, {"fork-core-read-file-tool"})

    def test_filters_by_id_without_fork_prefix(self) -> None:
        tests, error = fork_cli.card_tests_for_filters(
            self.make_repo(), ["core-read-file-tool"]
        )

        self.assertIsNone(error)
        self.assertEqual({test.card_id for test in tests}, {"fork-core-read-file-tool"})

    def test_ignores_non_active_card_test_blocks(self) -> None:
        tests, error = fork_cli.card_tests_for_filters(self.make_repo(), None)

        self.assertIsNone(error)
        self.assertEqual({test.card_id for test in tests}, {"fork-core-read-file-tool"})

    def test_unknown_card_reports_available_ids(self) -> None:
        tests, error = fork_cli.card_tests_for_filters(self.make_repo(), ["missing"])

        self.assertEqual(tests, [])
        self.assertIsNotNone(error)
        assert error is not None
        self.assertIn("unknown card filter(s): missing", error)
        self.assertIn("fork-core-read-file-tool", error)

    def test_card_without_tests_reports_missing_card_tests_entry(self) -> None:
        tests, error = fork_cli.card_tests_for_filters(
            self.make_repo(), ["docs/fork/planned-only.md"]
        )

        self.assertEqual(tests, [])
        self.assertIsNotNone(error)
        assert error is not None
        self.assertIn("card(s) have no fork-tests.v1 entries: fork-planned-only", error)

    def test_list_command_prints_filtered_test_exceptions_with_reasons(self) -> None:
        repo_root = self.make_repo()
        args = type(
            "Args",
            (),
            {
                "repo_root": str(repo_root),
                "version": "0.146.1",
                "mode": "list",
                "card": ["fork-manual-only", "fork-not-applicable-only"],
            },
        )()

        with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as stdout:
            with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as stderr:
                with unittest.mock.patch("sys.stdout", stdout):
                    with unittest.mock.patch("sys.stderr", stderr):
                        result = fork_cli.cmd_tests(args)
                stdout.seek(0)
                output = stdout.read()

        self.assertEqual(result, 0)
        normalized_lines = [" ".join(line.split()) for line in output.splitlines()]
        self.assertIn(
            "fork-manual-only manual-required "
            "Поведение подтверждается интерактивно в TUI.",
            normalized_lines,
        )
        self.assertIn(
            "fork-not-applicable-only not-applicable "
            "Отдельного card-level test нет.",
            normalized_lines,
        )

    def test_executable_filter_rejects_manual_test_exception(self) -> None:
        tests, error = fork_cli.card_tests_for_filters(
            self.make_repo(), ["fork-manual-only"]
        )

        self.assertEqual(tests, [])
        self.assertIsNotNone(error)
        assert error is not None
        self.assertIn("card(s) have no fork-tests.v1 entries: fork-manual-only", error)


class CardsListTests(unittest.TestCase):
    def test_cards_list_excludes_migration_cards(self) -> None:
        temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(temp_dir.cleanup)

        repo_root = Path(temp_dir.name)
        docs_fork = repo_root / "docs/fork"
        docs_fork.mkdir(parents=True)
        (docs_fork / "core-read-file-tool.md").write_text(
            "---\nid: fork-core-read-file-tool\nstatus: active\n---\n# Read file\n",
            encoding="utf-8",
        )
        (docs_fork / "migration-0.143.0.md").write_text(
            "---\n"
            "id: fork-migration-0.143.0\n"
            "status: completed\n"
            "---\n"
            "# Migration check: `0.143.0`\n",
            encoding="utf-8",
        )

        args = type(
            "Args",
            (),
            {
                "repo_root": str(repo_root),
                "format": "tsv",
            },
        )()

        with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as stdout:
            with unittest.mock.patch("sys.stdout", stdout):
                result = fork_cli.cmd_cards_list(args)
            stdout.seek(0)
            output = stdout.read()

        self.assertEqual(result, 0)
        self.assertIn("docs/fork/core-read-file-tool.md\tactive\tRead file", output)
        self.assertNotIn("migration-0.143.0.md", output)


class MigrationMapTests(unittest.TestCase):
    def make_repo(self, *cards: tuple[str, str]) -> Path:
        """Создаёт временный ``docs/fork`` с заданными active owner cards."""

        temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(temp_dir.cleanup)
        repo_root = Path(temp_dir.name)
        docs_fork = repo_root / "docs/fork"
        docs_fork.mkdir(parents=True)
        for filename, card_id in cards:
            (docs_fork / filename).write_text(
                f"---\nid: {card_id}\nstatus: active\n---\n# {card_id}\n",
                encoding="utf-8",
            )
        return repo_root

    def args(self, repo_root: Path, **values: object) -> object:
        """Создаёт минимальный argparse-подобный объект для прямого вызова command."""

        return type(
            "Args",
            (),
            {
                "repo_root": str(repo_root),
                "version": "0.144.3",
                **values,
            },
        )()

    def run_quietly(self, command: object, args: object) -> int:
        """Запускает лёгкую CLI-функцию, не смешивая её вывод с unittest output."""

        with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as stdout:
            with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as stderr:
                with unittest.mock.patch("sys.stdout", stdout):
                    with unittest.mock.patch("sys.stderr", stderr):
                        return command(args)  # type: ignore[operator]

    def init_map(self, repo_root: Path) -> dict[str, object]:
        """Создаёт карту через public command и возвращает распарсенный JSON."""

        self.assertEqual(
            self.run_quietly(
                fork_cli.migration_cli.cmd_migration_init,
                self.args(repo_root),
            ),
            0,
        )
        path = fork_cli.migration_map_path(repo_root, "0.144.3")
        return fork_cli.migration_map_model.read_migration_map(path)

    def test_init_generates_only_the_machine_contract(self) -> None:
        repo_root = self.make_repo(
            ("z-card.md", "fork-z-card"),
            ("a-card.md", "fork-a-card"),
        )

        value = self.init_map(repo_root)

        self.assertEqual(
            value,
            {
                "schema": "fork-migration.v1",
                "migration": {
                    "version": "0.144.3",
                    "upstreamTag": "rust-v0.144.3",
                    "branch": "hermione-0.144.3",
                    "status": "active",
                },
                "cards": [
                    {
                        "id": "fork-a-card",
                        "path": "docs/fork/a-card.md",
                        "status": "pending",
                    },
                    {
                        "id": "fork-z-card",
                        "path": "docs/fork/z-card.md",
                        "status": "pending",
                    },
                ],
                "gates": {
                    "preflight": "pending",
                    "format": "pending",
                    "generators": "pending",
                    "cardsValidate": "pending",
                    "cardTests": "pending",
                    "fullTests": "skipped",
                    "buildFast": "pending",
                    "install": "skipped",
                },
            },
        )

    def test_set_card_status_changes_only_selected_record(self) -> None:
        repo_root = self.make_repo(
            ("a-card.md", "fork-a-card"),
            ("b-card.md", "fork-b-card"),
        )
        before = self.init_map(repo_root)

        result = self.run_quietly(
            fork_cli.migration_cli.cmd_migration_set_card_status,
            self.args(repo_root, card="fork-b-card", status="migrated"),
        )
        after = fork_cli.migration_map_model.read_migration_map(
            fork_cli.migration_map_path(repo_root, "0.144.3")
        )

        expected = dict(before)
        expected["cards"] = [
            {
                "id": "fork-a-card",
                "path": "docs/fork/a-card.md",
                "status": "pending",
            },
            {
                "id": "fork-b-card",
                "path": "docs/fork/b-card.md",
                "status": "migrated",
            },
        ]
        self.assertEqual(result, 0)
        self.assertEqual(after, expected)

    def test_set_gate_status_changes_only_selected_record(self) -> None:
        repo_root = self.make_repo(("a-card.md", "fork-a-card"))
        before = self.init_map(repo_root)

        result = self.run_quietly(
            fork_cli.migration_cli.cmd_migration_set_gate_status,
            self.args(repo_root, gate="preflight", status="passed"),
        )
        after = fork_cli.migration_map_model.read_migration_map(
            fork_cli.migration_map_path(repo_root, "0.144.3")
        )

        expected = dict(before)
        expected["gates"] = {
            **before["gates"],  # type: ignore[dict-item]
            "preflight": "passed",
        }
        self.assertEqual(result, 0)
        self.assertEqual(after, expected)

    def test_next_is_read_only(self) -> None:
        repo_root = self.make_repo(("a-card.md", "fork-a-card"))
        self.init_map(repo_root)
        path = fork_cli.migration_map_path(repo_root, "0.144.3")
        before = path.read_bytes()

        result = self.run_quietly(
            fork_cli.migration_cli.cmd_migration_next,
            self.args(repo_root),
        )

        self.assertEqual(result, 0)
        self.assertEqual(path.read_bytes(), before)

    def test_validate_rejects_arbitrary_top_level_text(self) -> None:
        repo_root = self.make_repo(("a-card.md", "fork-a-card"))
        value = self.init_map(repo_root)
        value["notes"] = "arbitrary text"
        fork_cli.migration_map_model.write_migration_map_atomic(
            fork_cli.migration_map_path(repo_root, "0.144.3"),
            value,
        )

        result = self.run_quietly(
            fork_cli.migration_cli.cmd_migration_validate,
            self.args(repo_root),
        )

        self.assertEqual(result, 1)

    def test_complete_requires_final_cards_and_gates(self) -> None:
        repo_root = self.make_repo(("a-card.md", "fork-a-card"))
        value = self.init_map(repo_root)
        blocked = self.run_quietly(
            fork_cli.migration_cli.cmd_migration_complete,
            self.args(repo_root),
        )
        value = fork_cli.migration_map_model.with_card_status(
            value,
            selector="fork-a-card",
            status="migrated",
        )
        for gate in fork_cli.migration_map_model.GATE_NAMES:
            value = fork_cli.migration_map_model.with_gate_status(
                value,
                gate=gate,
                status="passed",
            )
        fork_cli.migration_map_model.write_migration_map_atomic(
            fork_cli.migration_map_path(repo_root, "0.144.3"),
            value,
        )

        completed = self.run_quietly(
            fork_cli.migration_cli.cmd_migration_complete,
            self.args(repo_root),
        )
        after = fork_cli.migration_map_model.read_migration_map(
            fork_cli.migration_map_path(repo_root, "0.144.3")
        )

        self.assertEqual(blocked, 1)
        self.assertEqual(completed, 0)
        self.assertEqual(after["migration"]["status"], "completed")  # type: ignore[index]

    def test_completed_map_is_immutable(self) -> None:
        repo_root = self.make_repo(("a-card.md", "fork-a-card"))
        value = self.init_map(repo_root)
        value = fork_cli.migration_map_model.with_card_status(
            value,
            selector="fork-a-card",
            status="migrated",
        )
        for gate in fork_cli.migration_map_model.GATE_NAMES:
            value = fork_cli.migration_map_model.with_gate_status(
                value,
                gate=gate,
                status="skipped",
            )
        value = fork_cli.migration_map_model.completed_migration_map(value)
        fork_cli.migration_map_model.write_migration_map_atomic(
            fork_cli.migration_map_path(repo_root, "0.144.3"),
            value,
        )

        result = self.run_quietly(
            fork_cli.migration_cli.cmd_migration_set_card_status,
            self.args(repo_root, card="fork-a-card", status="pending"),
        )

        self.assertEqual(result, 1)

    def test_old_markdown_map_does_not_infer_version(self) -> None:
        repo_root = self.make_repo(("a-card.md", "fork-a-card"))
        (repo_root / "docs/fork/migration-0.143.0.md").write_text(
            "# Historical migration\n",
            encoding="utf-8",
        )

        with unittest.mock.patch.object(
            fork_cli, "current_branch", return_value="main"
        ):
            version = fork_cli.infer_version(repo_root)

        self.assertIsNone(version)

    def test_parser_exposes_explicit_migration_commands(self) -> None:
        parser = fork_cli.build_parser()

        args = parser.parse_args(
            [
                "migration",
                "set-card-status",
                "--version",
                "0.144.3",
                "--card",
                "fork-a-card",
                "--status",
                "needsFix",
            ]
        )

        self.assertEqual(args.command, "migration")
        self.assertEqual(args.migration_command, "set-card-status")
        self.assertEqual(args.version, "0.144.3")
        self.assertEqual(args.card, "fork-a-card")
        self.assertEqual(args.status, "needsFix")

    def test_parser_rejects_version_that_is_not_x_y_z(self) -> None:
        parser = fork_cli.build_parser()

        with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as stderr:
            with unittest.mock.patch("sys.stderr", stderr):
                with self.assertRaises(SystemExit):
                    parser.parse_args(
                        [
                            "migration",
                            "show",
                            "--version",
                            "../../outside",
                        ]
                    )


class RetiredCommandTests(unittest.TestCase):
    def test_parser_rejects_retired_source_coverage_command(self) -> None:
        parser = fork_cli.build_parser()

        with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as stderr:
            with unittest.mock.patch("sys.stderr", stderr):
                with self.assertRaises(SystemExit):
                    parser.parse_args(["check-source-coverage"])


class CardValidationTests(unittest.TestCase):
    def write_card(
        self,
        *,
        transfer_body: str = "1. Проверь owner-файлы и перенеси контракт.",
        architecture_section: str = textwrap.dedent(
            """\
            ## Архитектурное решение

            Handler и исполняемая карта разделены.
            """
        ),
        checks_intro: str = "Исполняемая карта проверяет runtime-контракт.",
        fork_tests_block: str = FORK_TESTS_BLOCK,
        extra_sections: str = "",
    ) -> Path:
        temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(temp_dir.cleanup)
        path = Path(temp_dir.name) / "core-read-file-tool.md"
        template = textwrap.dedent(
            """\
                ---
                id: fork-core-read-file-tool
                status: active
                ---
                # Read file

                ## Обзор

                Активная карточка.

                ## Зачем это нужно

                Проверяет контракт.

                ## Карта файлов

                | Файл | Роль |
                | --- | --- |
                | `codex-rs/core/src/tools/handlers/read_file.rs` | Runtime-контракт |

                ## Итоговый контракт

                Контракт сохраняется.

                {{architecture_section}}

                ## Порядок повторения при переносе

                {{transfer_body}}

                ## Проверки

                {{checks_intro}}

                {{fork_tests_block}}

                ## Риски и ограничения

                Постоянные ограничения описаны здесь.

                {{extra_sections}}
                """
        )
        path.write_text(
            template.replace("{{transfer_body}}", transfer_body)
            .replace("{{architecture_section}}", architecture_section)
            .replace("{{checks_intro}}", checks_intro)
            .replace("{{fork_tests_block}}", fork_tests_block)
            .replace("{{extra_sections}}", extra_sections),
            encoding="utf-8",
        )
        return path

    def test_accepts_minimal_current_state_card(self) -> None:
        errors = fork_cli.strict_card_validation_errors(self.write_card())

        self.assertEqual(errors, [])

    def test_accepts_manual_test_exception_with_reason(self) -> None:
        errors = fork_cli.strict_card_validation_errors(
            self.write_card(
                checks_intro=(
                    "manual-required: Поведение подтверждается интерактивно в TUI."
                ),
                fork_tests_block="",
            )
        )

        self.assertEqual(errors, [])

    def test_rejects_missing_test_map_without_exception(self) -> None:
        errors = fork_cli.strict_card_validation_errors(
            self.write_card(fork_tests_block="")
        )

        self.assertTrue(
            any("no fork-tests.v1 block and no" in error for error in errors),
            errors,
        )

    def test_rejects_manual_test_exception_without_reason(self) -> None:
        errors = fork_cli.strict_card_validation_errors(
            self.write_card(
                checks_intro="manual-required:",
                fork_tests_block="",
            )
        )

        self.assertTrue(
            any("no fork-tests.v1 block and no" in error for error in errors),
            errors,
        )

    def test_requires_architectural_decision_section(self) -> None:
        errors = fork_cli.strict_card_validation_errors(
            self.write_card(architecture_section="")
        )

        self.assertIn(
            "missing owner-card section group: Архитектурное решение",
            errors,
        )

    def test_rejects_noncanonical_owner_card_section_aliases(self) -> None:
        aliases = (
            "## Карта файлов и смысл правок",
            "## Контракт внутренних документов",
            "## Пошаговое воспроизведение",
            "## Риски",
            "## Примеры поведения",
        )

        for alias in aliases:
            with self.subTest(alias=alias):
                errors = fork_cli.strict_card_validation_errors(
                    self.write_card(
                        extra_sections=f"{alias}\n\nНеканонический раздел."
                    )
                )

                self.assertTrue(
                    any(
                        "unexpected owner-card section is not allowed" in error
                        for error in errors
                    ),
                    errors,
                )

    def test_rejects_legacy_owner_card_sections(self) -> None:
        legacy_sections = (
            "Согласованные решения:",
            "Отклоненные альтернативы:",
            "## Отклонённые альтернативы",
            "### Смысловое покрытие",
            "## Ожидаемое покрытие diff",
            "### Исторические результаты",
            "## Исторические lint-заметки",
            "## Цепочка коммитов",
            "## Commit chain",
            "### Известные падения и пропуски",
            "## Runtime, сборка и установка",
            "## Выполнение, сборка и установка",
            "## Проверка покрытия",
            "## Аудит миграции `rust-v0.146.0`",
            "## Миграция на `0.146.0`",
            "## Migration repair: `0.146.0`",
            "## Migration check: `0.146.0`",
        )

        for section in legacy_sections:
            with self.subTest(section=section):
                errors = fork_cli.strict_card_validation_errors(
                    self.write_card(
                        extra_sections=f"{section}\n\nУстаревшее содержимое."
                    )
                )

                self.assertTrue(
                    any(
                        "legacy owner-card section is not allowed" in error
                        for error in errors
                    ),
                    errors,
                )

    def test_rejects_local_log_artifact_in_active_card(self) -> None:
        errors = fork_cli.strict_card_validation_errors(
            self.write_card(
                checks_intro=(
                    "Wrapper-log: "
                    "`target/fork-migration/build-logs/build-fast.log`."
                )
            )
        )

        self.assertTrue(
            any(
                "committed local log artifact breadcrumb" in error
                and "target/fork-migration" in error
                for error in errors
            ),
            errors,
        )

    def test_rejects_local_log_artifact_in_migration_card(self) -> None:
        temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(temp_dir.cleanup)
        path = Path(temp_dir.name) / "migration-0.142.5.md"
        path.write_text(
            textwrap.dedent(
                """\
                # Миграция Codex fork на `0.142.5`

                Статус: `complete`

                ## Общие проверки

                - `fork preflight --version 0.142.5`: `OK`;
                  `target/fork-migration/preflight-logs/preflight.log`.
                """
            ),
            encoding="utf-8",
        )

        errors = fork_cli.strict_card_validation_errors(path)

        self.assertTrue(
            any(
                "committed local log artifact breadcrumb" in error
                and "target/fork-migration" in error
                for error in errors
            ),
            errors,
        )

    def test_rejects_invalid_fork_tests_json(self) -> None:
        errors = fork_cli.strict_card_validation_errors(
            self.write_card(
                fork_tests_block=(
                    '```json\n{ "schema": "fork-tests.v1", "tests": [\n```\n'
                )
            )
        )

        self.assertTrue(
            any("invalid fork-tests.v1 JSON block" in error for error in errors),
            errors,
        )

    def test_rejects_command_runbook_in_transfer_section(self) -> None:
        errors = fork_cli.strict_card_validation_errors(
            self.write_card(
                transfer_body="1. Запусти `just test -p codex-core read_file`."
            )
        )

        self.assertTrue(
            any(
                "runbook command leakage in `Порядок повторения при переносе`" in error
                for error in errors
            ),
            errors,
        )

    def test_rejects_command_block_in_transfer_section(self) -> None:
        errors = fork_cli.strict_card_validation_errors(
            self.write_card(
                transfer_body="```bash\njust test -p codex-core read_file\n```"
            )
        )

        self.assertTrue(
            any(
                "runbook command block in `Порядок повторения при переносе`" in error
                for error in errors
            ),
            errors,
        )

    def test_rejects_command_runbook_in_checks(self) -> None:
        errors = fork_cli.strict_card_validation_errors(
            self.write_card(
                checks_intro=(
                    "Запусти "
                    "`fork tests --mode cards --card fork-core-read-file-tool`."
                )
            )
        )

        self.assertTrue(
            any(
                "runbook command leakage in `Проверки`" in error
                for error in errors
            ),
            errors,
        )


if __name__ == "__main__":
    unittest.main()
