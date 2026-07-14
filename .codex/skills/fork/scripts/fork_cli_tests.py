"""Регрессионные тесты публичного поведения skill-owned fork CLI.

Тесты используют только временные checkout-структуры и локальные fixtures: они
не меняют рабочий репозиторий, не запускают сеть, сборку или Git-операции. Здесь
проверяются parser contracts, fork-карточки, JSON migration map и безопасные
ошибки лёгких CLI-команд; тяжёлые workflow gates остаются интеграционными
командами самого skill.
"""

import importlib.util
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


class InstallPathTests(unittest.TestCase):
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


class CardValidationTests(unittest.TestCase):
    def write_card(
        self,
        *,
        transfer_body: str = "1. Проверь owner-файлы и перенеси контракт.",
        historical_results: str = (
            "| `just test -p codex-core read_file` | `passed` | Исторический запуск |"
        ),
        owner_detail: str = "`fork tests` владеет запуском; внутренние argv не являются runbook.",
        fork_tests_block: str = FORK_TESTS_BLOCK,
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

                ## Порядок повторения при переносе

                {{transfer_body}}

                ## Проверки

                ### Смысловое покрытие

                | Контракт | Обязательность | Где покрывается |
                | --- | --- | --- |
                | Runtime-контракт | `required` | `read_file` |

                ### Владелец исполняемой карты

                {{owner_detail}}

                {{fork_tests_block}}

                ### Дополнительные gates

                `not-applicable`: дополнительных gates нет.

                ### Исторические результаты

                | Проверка | Результат | Примечание |
                | --- | --- | --- |
                {{historical_results}}

                ### Известные падения и пропуски

                Нет.

                ## Риски и ограничения

                Нет.

                ## Проверка покрытия

                | Смысловой пункт | Статус | Где покрыто |
                | --- | --- | --- |
                | Контракт | `перенесено в карточку` | `Итоговый контракт` |
                """
        )
        path.write_text(
            template.replace("{{transfer_body}}", transfer_body)
            .replace("{{owner_detail}}", owner_detail)
            .replace("{{fork_tests_block}}", fork_tests_block)
            .replace("{{historical_results}}", historical_results),
            encoding="utf-8",
        )
        return path

    def test_allows_historical_command_results(self) -> None:
        errors = fork_cli.strict_card_validation_errors(self.write_card())

        self.assertEqual(errors, [])

    def test_rejects_local_log_artifact_in_active_card(self) -> None:
        errors = fork_cli.strict_card_validation_errors(
            self.write_card(
                historical_results=(
                    "| `fork build-fast` | `passed` | Wrapper-log: "
                    "`target/fork-migration/build-logs/build-fast.log` |"
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

    def test_rejects_internal_fork_argv_in_checks_owner(self) -> None:
        errors = fork_cli.strict_card_validation_errors(
            self.write_card(
                owner_detail=(
                    "`fork tests` владеет запуском через "
                    "`fork tests --mode cards --card fork-core-read-file-tool`."
                )
            )
        )

        self.assertTrue(
            any(
                "runbook command leakage in `Проверки`" in error
                and "Исторические результаты" in error
                for error in errors
            ),
            errors,
        )


if __name__ == "__main__":
    unittest.main()
