import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT_PATH = Path(__file__).with_name("fork_cli.py")
SPEC = importlib.util.spec_from_file_location("fork_cli", SCRIPT_PATH)
assert SPEC is not None
fork_cli = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(fork_cli)


class CardTestFilterTests(unittest.TestCase):
    def make_repo(self) -> Path:
        temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(temp_dir.cleanup)

        repo_root = Path(temp_dir.name)
        docs_fork = repo_root / "docs/fork"
        docs_fork.mkdir(parents=True)
        (docs_fork / "core-read-file-tool.md").write_text(
            "---\n"
            "id: fork-core-read-file-tool\n"
            "status: active\n"
            "---\n"
            "# Read file\n",
            encoding="utf-8",
        )
        (docs_fork / "planned-only.md").write_text(
            "---\n"
            "id: fork-planned-only\n"
            "status: planned\n"
            "---\n"
            "# Planned only\n",
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
        self.assertIn("card(s) have no CARD_TESTS entries: fork-planned-only", error)


if __name__ == "__main__":
    unittest.main()
