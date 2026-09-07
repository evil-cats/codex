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


TEST_GIT_REVISION = "0123456789abcdef0123456789abcdef01234567"
OTHER_GIT_REVISION = "89abcdef0123456789abcdef0123456789abcdef"
SCRIPT_PATH = Path(__file__).with_name("fork_cli.py")
sys.path.insert(0, str(SCRIPT_PATH.parent))
SPEC = importlib.util.spec_from_file_location("fork_cli", SCRIPT_PATH)
assert SPEC is not None
fork_cli = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(fork_cli)


def binary_version_output(label: str, revision: str) -> str:
    """Формирует точный вывод версии для бинарника из метки проверки."""
    command_name = "codex-code-mode-host" if "Code Mode host" in label else "codex"
    return f"{command_name} 0.0.0\nrevision {revision}\n"


class RecordingLogSession:
    """Записывает workflow-шаги и возвращает детерминированные ответы проверок."""

    instances = []
    capture_responses = {}
    step_responses = {}

    def __init__(self, **kwargs) -> None:
        self.mode = kwargs.get("mode")
        self.steps = []
        self.capture_steps = []
        self.env_overrides = []
        self.env_removals = []
        self.output = []
        self.ok_extra = None
        RecordingLogSession.instances.append(self)

    def write(self, text: str) -> None:
        self.output.append(text)

    def check_command(self, name: str) -> str:
        return name

    def run_step(
        self,
        label: str,
        argv: list[str],
        *,
        env_overrides: dict[str, str] | None = None,
        env_removals: tuple[str, ...] = (),
    ) -> int:
        """Запоминает шаг и изменения окружения без запуска дочернего процесса."""
        self.steps.append((label, argv))
        self.env_overrides.append(env_overrides)
        self.env_removals.append(env_removals)
        return RecordingLogSession.step_responses.get(label, 0)

    def run_capture(self, label: str, argv: list[str]) -> tuple[int, str, str]:
        """Записывает вызов и имитирует ответы Git либо проверки версии."""
        self.capture_steps.append((label, argv))
        if label in RecordingLogSession.capture_responses:
            return RecordingLogSession.capture_responses[label]
        if label.endswith("Git worktree status"):
            return 0, "", ""
        if label.endswith("Git HEAD"):
            return 0, f"{TEST_GIT_REVISION}\n", ""
        if label.endswith(" revision"):
            revision = (
                fork_cli.DEVELOPMENT_REVISION
                if self.mode == "build-fast" or " source revision" in label
                else TEST_GIT_REVISION
            )
            return 0, binary_version_output(label, revision), ""
        return 0, "", ""

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


class LogSessionEnvironmentTests(unittest.TestCase):
    """Проверяет изоляцию окружения запускаемых workflow-шагов."""

    def test_run_step_removes_selected_variable_from_child_environment(self) -> None:
        """Удаление действует на копию окружения и не меняет исходный словарь."""
        with tempfile.TemporaryDirectory() as temp_dir:
            session = fork_cli.LogSession(
                repo_root=Path(temp_dir), log_kind="test", mode="environment"
            )
            source_env = {"NO_COLOR": "1", "PRESERVED": "value"}
            completed = unittest.mock.Mock(returncode=0)

            with (
                unittest.mock.patch.object(
                    fork_cli, "command_env", side_effect=lambda: dict(source_env)
                ),
                unittest.mock.patch.object(
                    fork_cli.subprocess, "run", return_value=completed
                ) as run,
            ):
                result = session.run_step(
                    "environment isolation",
                    ["test-command"],
                    env_removals=("NO_COLOR",),
                )

            self.assertEqual(result, 0)
            self.assertEqual(run.call_args.kwargs["env"], {"PRESERVED": "value"})
            self.assertEqual(source_env, {"NO_COLOR": "1", "PRESERVED": "value"})


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
        RecordingLogSession.capture_responses.clear()
        RecordingLogSession.step_responses.clear()

    def create_executable_pair(self, main_path: Path) -> None:
        """Создаёт минимальную пару файлов для mocked build/install workflow."""
        main_path.parent.mkdir(parents=True, exist_ok=True)
        for path, content in (
            (main_path, "main"),
            (main_path.with_name("codex-code-mode-host"), "host"),
        ):
            path.write_text(content, encoding="utf-8")
            path.chmod(0o755)

    def create_release_fast_pair(self, repo_root: Path) -> Path:
        """Создаёт пару в единственном каталоге кандидатов `fork install`."""
        main_path = repo_root / "codex-rs/target/release-fast/codex"
        self.create_executable_pair(main_path)
        return main_path

    def test_parse_binary_revision_requires_two_line_version_contract(self) -> None:
        """Парсер принимает точный двухстрочный формат и допустимую ревизию."""
        self.assertEqual(
            (
                fork_cli.parse_binary_revision("codex 0.0.0\nrevision dev\n", "codex"),
                fork_cli.parse_binary_revision(
                    f"codex-code-mode-host 0.0.0\nrevision {TEST_GIT_REVISION}\n",
                    "codex-code-mode-host",
                ),
            ),
            ("dev", TEST_GIT_REVISION),
        )
        for output in (
            "codex 0.0.0\n",
            "codex-cli 0.0.0\nrevision dev\n",
            "codex \nrevision dev\n",
            "codex 0.0.0\nrevision unknown\n",
            "codex 0.0.0\nrevision abc123\n",
            "codex 0.0.0\nrevision dev\nrevision dev\n",
        ):
            with self.subTest(output=output):
                with self.assertRaises(ValueError):
                    fork_cli.parse_binary_revision(output, "codex")

    def test_revision_section_payload_accepts_only_full_git_sha(self) -> None:
        """Release payload имеет точный размер ELF-секции и не принимает dev."""
        self.assertEqual(
            fork_cli.revision_section_payload(TEST_GIT_REVISION),
            TEST_GIT_REVISION.encode("ascii"),
        )
        for revision in ("dev", "abc123", TEST_GIT_REVISION.upper()):
            with self.subTest(revision=revision):
                with self.assertRaises(ValueError):
                    fork_cli.revision_section_payload(revision)

    def test_build_fast_forces_development_revision_without_clean_check(self) -> None:
        """Development build остаётся доступным для dirty checkout и всегда даёт dev."""
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            self.create_executable_pair(
                repo_root / "codex-rs/target/release-fast/codex"
            )
            args = fork_cli.build_parser().parse_args(
                [
                    "build-fast",
                    "--repo-root",
                    str(repo_root),
                    "--version",
                    "0.150.0",
                    "--skip-branch-check",
                ]
            )
            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli, "run_preconditions", return_value=0
                ),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {"V8_TEST": "1"}),
                ),
            ):
                result = fork_cli.cmd_build_fast(args)

        self.assertEqual(result, 0)
        session = RecordingLogSession.instances[0]
        build_index = [label for label, _argv in session.steps].index(
            "release-fast build"
        )
        self.assertEqual(
            session.env_overrides[build_index],
            {"V8_TEST": "1", "STABLE_GIT_COMMIT": "dev"},
        )
        self.assertEqual(
            session.capture_steps,
            [
                (
                    "Codex build revision",
                    [
                        str(repo_root / "codex-rs/target/release-fast/codex"),
                        "--version",
                    ],
                ),
                (
                    "Code Mode host build revision",
                    [
                        str(
                            repo_root
                            / "codex-rs/target/release-fast/codex-code-mode-host"
                        ),
                        "--version",
                    ],
                ),
            ],
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

    def test_default_install_rejects_dirty_checkout_before_build(self) -> None:
        """Обычная установка не собирает и не публикует binary из dirty checkout."""
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(repo_root),
                    "--target",
                    str(repo_root / "install/codex-hermione"),
                ]
            )
            RecordingLogSession.capture_responses[
                "before release build Git worktree status"
            ] = (0, " M codex-rs/cli/src/main.rs\n", "")
            with unittest.mock.patch.object(
                fork_cli, "LogSession", RecordingLogSession
            ):
                result = fork_cli.cmd_install(args)

        self.assertEqual(result, 1)
        self.assertNotIn(
            "release-fast freshness build",
            [label for label, _argv in RecordingLogSession.instances[0].steps],
        )

    def test_default_install_stamps_staging_and_verifies_all_stages(self) -> None:
        """Clean install сохраняет source=dev и сверяет stamped и installed пары."""
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            self.create_executable_pair(
                repo_root / "codex-rs/target/release-fast/codex"
            )
            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(repo_root),
                    "--target",
                    str(repo_root / "install/codex-hermione"),
                ]
            )
            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {"V8_TEST": "1"}),
                ),
            ):
                result = fork_cli.cmd_install(args)

        self.assertEqual(result, 0)
        session = RecordingLogSession.instances[0]
        build_index = [label for label, _argv in session.steps].index(
            "release-fast freshness build"
        )
        self.assertEqual(
            session.env_overrides[build_index],
            {"V8_TEST": "1", "STABLE_GIT_COMMIT": "dev"},
        )
        self.assertEqual(
            [
                label
                for label, _argv in session.capture_steps
                if label.endswith(" revision")
            ],
            [
                "Codex source revision",
                "Code Mode host source revision",
                "Codex staged revision",
                "Code Mode host staged revision",
                "Codex installed revision",
                "Code Mode host installed revision",
            ],
        )
        stamp_steps = [
            (label, argv)
            for label, argv in session.steps
            if label.endswith("stamp staged binary")
        ]
        self.assertEqual(
            [label for label, _argv in stamp_steps],
            ["Codex stamp staged binary", "Code Mode host stamp staged binary"],
        )
        for _label, argv in stamp_steps:
            self.assertEqual(argv[1], "--update-section")
            self.assertTrue(argv[2].startswith(f"{fork_cli.REVISION_ELF_SECTION}="))
        self.assertIn(f"REVISION: {TEST_GIT_REVISION}", session.ok_extra)

    def test_default_install_rejects_head_change_during_build(self) -> None:
        """Смена HEAD между clean-checks блокирует публикацию собранной пары."""
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            self.create_executable_pair(
                repo_root / "codex-rs/target/release-fast/codex"
            )
            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(repo_root),
                    "--target",
                    str(repo_root / "install/codex-hermione"),
                ]
            )
            RecordingLogSession.capture_responses["after release build Git HEAD"] = (
                0,
                f"{OTHER_GIT_REVISION}\n",
                "",
            )
            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {}),
                ),
            ):
                result = fork_cli.cmd_install(args)

        self.assertEqual(result, 1)
        self.assertNotIn(
            "sync local binaries",
            [label for label, _argv in RecordingLogSession.instances[0].steps],
        )

    def test_default_install_rejects_source_revision_other_than_dev(self) -> None:
        """Freshness build обязан оставить оба source-кандидата development-сборками."""
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            self.create_executable_pair(
                repo_root / "codex-rs/target/release-fast/codex"
            )
            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(repo_root),
                    "--target",
                    str(repo_root / "install/codex-hermione"),
                ]
            )
            for label in ("Codex source revision", "Code Mode host source revision"):
                RecordingLogSession.capture_responses[label] = (
                    0,
                    binary_version_output(label, OTHER_GIT_REVISION),
                    "",
                )
            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {}),
                ),
            ):
                result = fork_cli.cmd_install(args)

        self.assertEqual(result, 1)
        self.assertNotIn(
            "sync local binaries",
            [label for label, _argv in RecordingLogSession.instances[0].steps],
        )

    def test_install_rejects_unstamped_staged_revision(self) -> None:
        """После objcopy staging-пара уже не может оставаться `revision dev`."""
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            main_source = repo_root / "codex-rs/target/release-fast/codex"
            self.create_executable_pair(main_source)
            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(repo_root),
                    "--target",
                    str(repo_root / "install/codex-hermione"),
                ]
            )
            for label in ("Codex staged revision", "Code Mode host staged revision"):
                RecordingLogSession.capture_responses[label] = (
                    0,
                    binary_version_output(label, "dev"),
                    "",
                )
            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {}),
                ),
            ):
                result = fork_cli.cmd_install(args)

        self.assertEqual(result, 1)
        self.assertNotIn(
            "sync local binaries",
            [label for label, _argv in RecordingLogSession.instances[0].steps],
        )

    def test_install_rejects_mismatched_candidate_revisions(self) -> None:
        """Development-пара отклоняется до staging при разных ревизиях."""
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            main_source = repo_root / "codex-rs/target/release-fast/codex"
            self.create_executable_pair(main_source)
            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(repo_root),
                    "--target",
                    str(repo_root / "install/codex-hermione"),
                ]
            )
            RecordingLogSession.capture_responses["Codex source revision"] = (
                0,
                binary_version_output("Codex source revision", "dev"),
                "",
            )
            RecordingLogSession.capture_responses["Code Mode host source revision"] = (
                0,
                binary_version_output(
                    "Code Mode host source revision", OTHER_GIT_REVISION
                ),
                "",
            )
            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {}),
                ),
            ):
                result = fork_cli.cmd_install(args)

        self.assertEqual(result, 1)
        self.assertNotIn(
            "sync local binaries",
            [label for label, _argv in RecordingLogSession.instances[0].steps],
        )

    def test_install_parser_accepts_target(self) -> None:
        parser = fork_cli.build_parser()
        args = parser.parse_args(
            [
                "install",
                "--target",
                "/tmp/codex-hermione",
            ]
        )

        self.assertEqual(args.command, "install")
        self.assertEqual(args.target, "/tmp/codex-hermione")
        self.assertEqual(args.host, [])

    def test_install_parser_accepts_repeated_remote_hosts(self) -> None:
        args = fork_cli.build_parser().parse_args(
            ["install", "--host", "oleg.home", "--host", "f-ms-dev"]
        )

        self.assertEqual(args.host, ["oleg.home", "f-ms-dev"])

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

    def test_install_syncs_both_artifacts_with_one_rsync_command(self) -> None:
        """Явный локальный target сохраняет прямую публикацию общей парой."""
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            main_source = self.create_release_fast_pair(repo_root)
            host_source = main_source.with_name("codex-code-mode-host")
            main_target = repo_root / "install/codex-hermione"
            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(repo_root),
                    "--target",
                    str(main_target),
                ]
            )
            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {}),
                ),
            ):
                result = fork_cli.cmd_install(args)

            self.assertEqual(result, 0)
            session = RecordingLogSession.instances[0]
            sync_steps = [
                argv for label, argv in session.steps if label == "sync local binaries"
            ]
            self.assertEqual(len(sync_steps), 1)
            sync_command = sync_steps[0]
            self.assertEqual(sync_command[0], "rsync")
            self.assertNotIn("--no-owner", sync_command)
            self.assertNotIn("--no-group", sync_command)
            self.assertIn("--delay-updates", sync_command)
            self.assertEqual(Path(sync_command[-3]).name, "codex-hermione")
            self.assertEqual(Path(sync_command[-2]).name, "codex-code-mode-host")
            self.assertEqual(sync_command[-1], f"{main_target.parent}/")
            self.assertIn(
                (
                    "Code Mode host source binary probe",
                    [str(host_source), "--version"],
                ),
                session.steps,
            )

    def test_default_install_uses_sudo_only_for_local_publication(self) -> None:
        """Системный default повышает права только для итогового `rsync`."""
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            self.create_release_fast_pair(repo_root)
            args = fork_cli.build_parser().parse_args(
                ["install", "--repo-root", str(repo_root)]
            )
            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(fork_cli.Path, "mkdir") as mkdir,
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {}),
                ),
            ):
                result = fork_cli.cmd_install(args)

        self.assertEqual(result, 0)
        mkdir.assert_called_once_with(parents=True, exist_ok=True)
        session = RecordingLogSession.instances[0]
        sync_command = next(
            argv for label, argv in session.steps if label == "sync local binaries"
        )
        self.assertEqual(
            sync_command[:6],
            ["sudo", "--", "rsync", "--archive", "--no-owner", "--no-group"],
        )
        self.assertEqual(sync_command[-1], "/usr/local/bin/")
        self.assertEqual(
            [label for label, argv in session.steps if argv and argv[0] == "sudo"],
            ["sync local binaries"],
        )
        self.assertEqual(
            session.ok_extra,
            [
                f"SOURCE: {repo_root / 'codex-rs/target/release-fast/codex'}",
                "TARGET: /usr/local/bin/codex-hermione",
                "CODE_MODE_HOST_SOURCE: "
                f"{repo_root / 'codex-rs/target/release-fast/codex-code-mode-host'}",
                "CODE_MODE_HOST_TARGET: /usr/local/bin/codex-code-mode-host",
                f"REVISION: {TEST_GIT_REVISION}",
            ],
        )

    def test_default_install_requires_sudo_before_build(self) -> None:
        """Недоступный `sudo` останавливает системную установку до сборки."""
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            args = fork_cli.build_parser().parse_args(
                ["install", "--repo-root", str(repo_root)]
            )
            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    RecordingLogSession,
                    "check_command",
                    autospec=True,
                    side_effect=lambda _session, name: None if name == "sudo" else name,
                ),
            ):
                result = fork_cli.cmd_install(args)

        self.assertEqual(result, 1)
        self.assertNotIn(
            "release-fast freshness build",
            [label for label, _argv in RecordingLogSession.instances[0].steps],
        )

    def test_install_strips_temporary_binaries_without_changing_sources(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            main_source = root / "codex-rs/target/release-fast/codex"
            host_source = main_source.with_name("codex-code-mode-host")
            main_source.parent.mkdir(parents=True)
            main_source.write_text("main source", encoding="utf-8")
            host_source.write_text("host source", encoding="utf-8")
            main_source.chmod(0o755)
            host_source.chmod(0o755)
            main_target = root / "install/codex-hermione"
            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(root),
                    "--target",
                    str(main_target),
                ]
            )

            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {}),
                ),
            ):
                result = fork_cli.cmd_install(args)

            self.assertEqual(result, 0)
            self.assertEqual(main_source.read_text(encoding="utf-8"), "main source")
            self.assertEqual(host_source.read_text(encoding="utf-8"), "host source")
            steps = RecordingLogSession.instances[0].steps
            strip_targets = {
                label: Path(argv[-1]).name
                for label, argv in steps
                if label.endswith("strip staged binary")
            }
            self.assertEqual(
                strip_targets,
                {
                    "Codex strip staged binary": "codex-hermione",
                    "Code Mode host strip staged binary": "codex-code-mode-host",
                },
            )
            self.assertEqual(
                [
                    label
                    for label, _argv in RecordingLogSession.instances[0].capture_steps
                    if "staged revision" in label
                ],
                ["Codex staged revision", "Code Mode host staged revision"],
            )

    def test_install_rejects_objcopy_failure_before_sync(self) -> None:
        """Ошибка обновления ELF-секции не должна публиковать ни один binary."""
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.create_release_fast_pair(root)
            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(root),
                    "--target",
                    str(root / "install/codex-hermione"),
                ]
            )
            RecordingLogSession.step_responses["Codex stamp staged binary"] = 1
            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {}),
                ),
            ):
                result = fork_cli.cmd_install(args)

        self.assertEqual(result, 1)
        self.assertNotIn(
            "sync local binaries",
            [label for label, _argv in RecordingLogSession.instances[0].steps],
        )

    def test_install_rejects_revision_changed_after_strip(self) -> None:
        """Неверный stamp во временной копии блокирует публикацию через rsync."""
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.create_release_fast_pair(root)
            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(root),
                    "--target",
                    str(root / "install/codex-hermione"),
                ]
            )
            for label in ("Codex staged revision", "Code Mode host staged revision"):
                RecordingLogSession.capture_responses[label] = (
                    0,
                    binary_version_output(label, OTHER_GIT_REVISION),
                    "",
                )
            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {}),
                ),
            ):
                result = fork_cli.cmd_install(args)

        self.assertEqual(result, 1)
        self.assertNotIn(
            "sync local binaries",
            [label for label, _argv in RecordingLogSession.instances[0].steps],
        )

    def test_remote_install_syncs_and_probes_each_host(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.create_release_fast_pair(root)
            args = fork_cli.build_parser().parse_args(
                [
                    "install",
                    "--repo-root",
                    str(root),
                    "--host",
                    "oleg.home",
                    "--host",
                    "f-ms-dev",
                ]
            )

            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {}),
                ),
            ):
                result = fork_cli.cmd_install(args)

            self.assertEqual(result, 0)
            steps = RecordingLogSession.instances[0].steps
            labels = [label for label, _argv in steps]
            for host in ("oleg.home", "f-ms-dev"):
                self.assertIn(f"{host} create install directory", labels)
                self.assertIn(f"{host} sync binaries", labels)
                self.assertIn(f"{host} probe installed binaries", labels)
                self.assertIn(
                    f"{host} Codex installed revision",
                    [
                        label
                        for label, _argv in RecordingLogSession.instances[
                            0
                        ].capture_steps
                    ],
                )
                self.assertIn(
                    f"{host} Code Mode host installed revision",
                    [
                        label
                        for label, _argv in RecordingLogSession.instances[
                            0
                        ].capture_steps
                    ],
                )
            sync_commands = [
                argv for label, argv in steps if label.endswith("sync binaries")
            ]
            self.assertEqual(len(sync_commands), 2)
            for command, host in zip(
                sync_commands, ("oleg.home", "f-ms-dev"), strict=True
            ):
                self.assertIn("--delay-updates", command)
                self.assertEqual(command[-1], f"{host}:.local/bin/")

    def test_install_requires_code_mode_host_before_syncing_main(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            main_source = repo_root / "codex-rs/target/release-fast/codex"
            main_source.parent.mkdir(parents=True)
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
                    "--target",
                    str(main_target),
                ]
            )

            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {}),
                ),
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
            unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
            unittest.mock.patch.object(
                fork_cli,
                "resolve_codex_v8_cargo_env_for_host",
                return_value=(0, cargo_env),
            ),
        ):
            result = fork_cli.cmd_fix(args)

        self.assertEqual(result, 0)
        session = RecordingLogSession.instances[-1]
        self.assertEqual(session.steps, [("rust lint fix", ["just", "fix"])])
        self.assertEqual(session.env_overrides, [cargo_env])


class CodeModeHostBuildTests(unittest.TestCase):
    def setUp(self) -> None:
        RecordingLogSession.instances.clear()

    def test_parser_accepts_code_mode_host_build(self) -> None:
        """Регистрирует отдельную skill-owned команду узкой сборки host."""
        args = fork_cli.build_parser().parse_args(
            ["build-code-mode-host", "--repo-root", "/repo"]
        )

        self.assertEqual(args.command, "build-code-mode-host")
        self.assertEqual(args.repo_root, "/repo")

    def test_build_passes_v8_environment_and_probes_debug_binary(self) -> None:
        """Передаёт V8-окружение сборке и проверяет созданный отладочный host."""
        cargo_env = {
            "RUSTY_V8_ARCHIVE": "/cache/v8.a",
            "RUSTY_V8_SRC_BINDING_PATH": "/cache/src_binding.rs",
        }
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            binary_path = fork_cli.debug_code_mode_host_binary(repo_root)
            binary_path.parent.mkdir(parents=True)
            binary_path.write_text("host", encoding="utf-8")
            binary_path.chmod(0o755)
            args = fork_cli.build_parser().parse_args(
                ["build-code-mode-host", "--repo-root", str(repo_root)]
            )

            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    RecordingLogSession,
                    "check_command",
                    return_value="/usr/bin/cargo",
                ),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, cargo_env),
                ),
            ):
                result = fork_cli.cmd_build_code_mode_host(args)

        self.assertEqual(result, 0)
        session = RecordingLogSession.instances[-1]
        self.assertEqual(
            session.steps,
            [
                (
                    "сборка отладочного Code Mode host",
                    [
                        "/usr/bin/cargo",
                        "build",
                        "--manifest-path",
                        str(repo_root / "codex-rs/Cargo.toml"),
                        "-p",
                        "codex-code-mode-host",
                        "--bin",
                        "codex-code-mode-host",
                    ],
                ),
                (
                    "проверка исполняемого файла Code Mode host",
                    [str(binary_path), "--version"],
                ),
            ],
        )
        self.assertEqual(session.env_overrides, [cargo_env, None])
        self.assertEqual(
            session.ok_extra,
            [f"CODE_MODE_HOST_BINARY: {binary_path}"],
        )

    def test_build_fails_when_cargo_did_not_create_debug_binary(self) -> None:
        """Не объявляет предусловие готовым без ожидаемого исполняемого файла."""
        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            args = fork_cli.build_parser().parse_args(
                ["build-code-mode-host", "--repo-root", str(repo_root)]
            )

            with (
                unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
                unittest.mock.patch.object(
                    fork_cli,
                    "resolve_codex_v8_cargo_env_for_host",
                    return_value=(0, {}),
                ),
            ):
                result = fork_cli.cmd_build_code_mode_host(args)

        self.assertEqual(result, 1)
        session = RecordingLogSession.instances[-1]
        self.assertEqual(len(session.steps), 1)
        self.assertIn(
            "не найден исполняемый файл Code Mode host",
            "".join(session.output),
        )


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
            "fork-not-applicable-only not-applicable Отдельного card-level test нет.",
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


class CardTestPlatformTests(unittest.TestCase):
    def setUp(self) -> None:
        RecordingLogSession.instances.clear()

    def make_repo(self) -> Path:
        temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(temp_dir.cleanup)

        repo_root = Path(temp_dir.name)
        docs_fork = repo_root / "docs/fork"
        docs_fork.mkdir(parents=True)
        (docs_fork / "platform-tests.md").write_text(
            textwrap.dedent(
                """\
                ---
                id: fork-platform-tests
                status: active
                ---
                # Platform tests

                ## Проверки

                ```json
                {
                  "schema": "fork-tests.v1",
                  "tests": [
                    {
                      "purpose": "portable behavior",
                      "argv": ["portable-test"]
                    },
                    {
                      "purpose": "Windows behavior",
                      "platforms": ["windows"],
                      "argv": ["windows-test"]
                    }
                  ]
                }
                ```
                """
            ),
            encoding="utf-8",
        )
        return repo_root

    def args(self, repo_root: Path, mode: str) -> object:
        return type(
            "Args",
            (),
            {
                "repo_root": str(repo_root),
                "version": "0.149.0",
                "mode": mode,
                "card": ["fork-platform-tests"],
                "skip_branch_check": False,
            },
        )()

    def test_platforms_are_part_of_parsed_card_test(self) -> None:
        tests, errors = fork_cli.card_tests_in_card(
            self.make_repo() / "docs/fork/platform-tests.md"
        )

        self.assertEqual(errors, [])
        self.assertEqual(
            tests,
            [
                fork_cli.card_test(
                    "fork-platform-tests", "portable behavior", "portable-test"
                ),
                fork_cli.card_test(
                    "fork-platform-tests",
                    "Windows behavior",
                    "windows-test",
                    platforms=("windows",),
                ),
            ],
        )

    def test_unknown_platform_is_rejected(self) -> None:
        tests, errors = fork_cli.card_tests_from_payload(
            Path("card.md"),
            "fork-platform-tests",
            1,
            {
                "schema": "fork-tests.v1",
                "tests": [
                    {
                        "purpose": "unknown platform",
                        "platforms": ["freebsd"],
                        "argv": ["test"],
                    }
                ],
            },
        )

        self.assertEqual(tests, [])
        self.assertEqual(
            errors,
            [
                "card.md: fork-tests.v1 block at line 1: tests[1]: "
                "unsupported platforms: freebsd"
            ],
        )

    def test_list_marks_non_matching_platform_as_skipped(self) -> None:
        args = self.args(self.make_repo(), "list")

        with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as stdout:
            with (
                unittest.mock.patch("sys.stdout", stdout),
                unittest.mock.patch.object(
                    fork_cli, "current_fork_test_platform", return_value="linux"
                ),
            ):
                result = fork_cli.cmd_tests(args)
            stdout.seek(0)
            output = stdout.read()

        self.assertEqual(result, 0)
        normalized_lines = [" ".join(line.split()) for line in output.splitlines()]
        self.assertIn(
            "fork-platform-tests run portable behavior portable-test",
            normalized_lines,
        )
        self.assertIn(
            "fork-platform-tests skip-platform Windows behavior "
            "windows-test platforms=windows",
            normalized_lines,
        )

    def test_cards_skip_non_matching_platform_and_report_counts(self) -> None:
        """Запускается только подходящая запись с нормализованным окружением."""
        args = self.args(self.make_repo(), "cards")

        with (
            unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
            unittest.mock.patch.object(fork_cli, "run_preconditions", return_value=0),
            unittest.mock.patch.object(
                fork_cli, "current_fork_test_platform", return_value="linux"
            ),
        ):
            result = fork_cli.cmd_tests(args)

        self.assertEqual(result, 0)
        session = RecordingLogSession.instances[-1]
        self.assertEqual(
            session.steps,
            [("fork-platform-tests: portable behavior", ["portable-test"])],
        )
        self.assertEqual(session.env_removals, [fork_cli.CARD_TEST_ENV_REMOVALS])
        self.assertEqual(
            session.ok_extra,
            ["TESTS_PASSED: 1", "TESTS_SKIPPED_PLATFORM: 1"],
        )
        self.assertIn(
            "SKIP_PLATFORM: current=linux required=windows",
            "".join(session.output),
        )

    def test_cards_run_matching_platform(self) -> None:
        """Все подходящие записи получают одинаковые удаления из окружения."""
        args = self.args(self.make_repo(), "cards")

        with (
            unittest.mock.patch.object(fork_cli, "LogSession", RecordingLogSession),
            unittest.mock.patch.object(fork_cli, "run_preconditions", return_value=0),
            unittest.mock.patch.object(
                fork_cli, "current_fork_test_platform", return_value="windows"
            ),
        ):
            result = fork_cli.cmd_tests(args)

        self.assertEqual(result, 0)
        session = RecordingLogSession.instances[-1]
        self.assertEqual(
            session.steps,
            [
                ("fork-platform-tests: portable behavior", ["portable-test"]),
                ("fork-platform-tests: Windows behavior", ["windows-test"]),
            ],
        )
        self.assertEqual(
            session.env_removals,
            [fork_cli.CARD_TEST_ENV_REMOVALS, fork_cli.CARD_TEST_ENV_REMOVALS],
        )
        self.assertEqual(
            session.ok_extra,
            ["TESTS_PASSED: 2", "TESTS_SKIPPED_PLATFORM: 0"],
        )


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
                    self.write_card(extra_sections=f"{alias}\n\nНеканонический раздел.")
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
                    "Wrapper-log: `target/fork-migration/build-logs/build-fast.log`."
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
                    "Запусти `fork tests --mode cards --card fork-core-read-file-tool`."
                )
            )
        )

        self.assertTrue(
            any("runbook command leakage in `Проверки`" in error for error in errors),
            errors,
        )


if __name__ == "__main__":
    unittest.main()
