"""Лёгкие CLI-команды для JSON-карты миграции Hermione fork.

Модуль связывает ``argparse`` с data layer ``migration_map.py``: создаёт снимок
активных owner-карточек, показывает состояние, обновляет одну запись и явно
завершает карту. Он не сканирует исторические migration artifacts, не принимает
решение о resume и не запускает Git, агентов или тяжёлые проверки. Все записи
проходят структурную и repository-aware валидацию до атомарного ``replace``.
"""

import argparse
import re
import sys
from collections.abc import Mapping
from pathlib import Path

import migration_map as migration_map_model


SCRIPT_DIR = Path(__file__).resolve().parent
SKILL_ROOT = SCRIPT_DIR.parent
DEFAULT_REPO_ROOT = SKILL_ROOT.parents[2]


def find_repo_root(start: Path | None = None) -> Path:
    """Находит checkout с fork skill или возвращает checkout самого skill."""

    current = (start or Path.cwd()).resolve()
    for candidate in (current, *current.parents):
        if (candidate / ".git").exists() and (
            candidate / ".codex/skills/fork"
        ).exists():
            return candidate
    return DEFAULT_REPO_ROOT


def _first_card_value(path: Path, field: str) -> str:
    """Читает строковое поле обзорного слоя owner-карточки без полного parse."""

    for line in path.read_text(encoding="utf-8").splitlines()[:80]:
        match = re.match(rf"\s*{re.escape(field)}:\s*`?([^`]+?)`?\s*$", line)
        if match:
            return match.group(1).strip()
    return ""


def first_card_id(path: Path) -> str:
    """Возвращает frontmatter ``id`` owner-карточки."""

    return _first_card_value(path, "id")


def first_card_status(path: Path) -> str:
    """Возвращает первый machine-readable ``status`` owner-карточки."""

    return _first_card_value(path, "status") or _first_card_value(path, "Статус")


def owner_card_paths(repo_root: Path) -> list[Path]:
    """Возвращает только корневые Markdown owner-карточки, без старых карт."""

    docs_fork = repo_root / "docs/fork"
    if not docs_fork.exists():
        return []
    return sorted(
        path
        for path in docs_fork.glob("*.md")
        if not path.name.startswith("migration-")
    )


def migration_map_errors(
    value: object,
    *,
    repo_root: Path,
    version: str,
) -> list[str]:
    """Проверяет формат карты и её связи с owner-карточками checkout."""

    errors = migration_map_model.migration_map_validation_errors(
        value,
        expected_version=version,
    )
    if errors:
        return errors
    return migration_map_model.repository_validation_errors(
        value,
        repo_root=repo_root,
        card_id=first_card_id,
    )


def load_validated_migration_map(
    repo_root: Path,
    version: str,
) -> tuple[dict[str, object] | None, list[str]]:
    """Загружает карту выбранной версии и возвращает все ошибки без исключений."""

    path = migration_map_model.migration_map_path(repo_root, version)
    if not path.is_file():
        return None, [f"migration map does not exist: {path}"]
    try:
        value = migration_map_model.read_migration_map(path)
    except ValueError as exc:
        return None, [str(exc)]
    return value, migration_map_errors(value, repo_root=repo_root, version=version)


def active_owner_card_entries(
    repo_root: Path,
) -> tuple[list[dict[str, str]], list[str]]:
    """Собирает минимальный снимок активных owner-карточек для ``init``."""

    cards = []
    errors = []
    for path in owner_card_paths(repo_root):
        if first_card_status(path) != "active":
            continue
        card_id = first_card_id(path)
        relative = path.relative_to(repo_root).as_posix()
        if not card_id:
            errors.append(f"active owner card has no id: {relative}")
            continue
        cards.append({"id": card_id, "path": relative})
    if not cards and not errors:
        errors.append("no active owner cards found")
    return cards, errors


def print_migration_errors(errors: list[str]) -> None:
    """Печатает короткий стабильный список ошибок migration-команды."""

    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)


def write_validated_migration_map(
    *,
    repo_root: Path,
    version: str,
    value: Mapping[str, object],
) -> list[str]:
    """Валидирует и атомарно записывает карту, возвращая ошибки без исключений."""

    errors = migration_map_errors(value, repo_root=repo_root, version=version)
    if errors:
        return errors
    try:
        migration_map_model.write_migration_map_atomic(
            migration_map_model.migration_map_path(repo_root, version),
            value,
        )
    except OSError as exc:
        return [f"could not write migration map: {exc}"]
    return []


def active_migration_error(value: Mapping[str, object]) -> str | None:
    """Запрещает изменение завершённого исторического артефакта."""

    migration = value.get("migration")
    if not isinstance(migration, Mapping) or migration.get("status") != "active":
        return "completed migration map is immutable"
    return None


def migration_version(value: str) -> str:
    """Проверяет CLI-значение версии до построения пути к JSON-карте."""

    try:
        return migration_map_model.validate_migration_version(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError(str(exc)) from exc


def cmd_migration_init(args: argparse.Namespace) -> int:
    """Генерирует новую JSON-карту из текущего снимка active owner cards."""

    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    version = args.version
    path = migration_map_model.migration_map_path(repo_root, version)
    if path.exists():
        print(f"ERROR: migration map already exists: {path}", file=sys.stderr)
        return 2
    cards, errors = active_owner_card_entries(repo_root)
    if errors:
        print_migration_errors(errors)
        return 1
    value = migration_map_model.new_migration_map(version, cards)
    errors = write_validated_migration_map(
        repo_root=repo_root,
        version=version,
        value=value,
    )
    if errors:
        print_migration_errors(errors)
        return 1
    print(f"created: {path.relative_to(repo_root)}")
    return 0


def cmd_migration_show(args: argparse.Namespace) -> int:
    """Показывает зафиксированное состояние без выбора дальнейшего действия."""

    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    value, errors = load_validated_migration_map(repo_root, args.version)
    if errors or value is None:
        print_migration_errors(errors)
        return 1
    migration = value["migration"]
    cards = value["cards"]
    gates = value["gates"]
    assert isinstance(migration, Mapping)
    assert isinstance(cards, list)
    assert isinstance(gates, Mapping)
    processed, total, remaining = migration_map_model.migration_progress(value)
    print(f"version: {migration['version']}")
    print(f"status: {migration['status']}")
    print(f"processed: {processed}")
    print(f"total: {total}")
    print(f"remaining: {remaining}")
    print("cards:")
    for card in cards:
        assert isinstance(card, Mapping)
        print(f"{card['id']}\t{card['status']}\t{card['path']}")
    print("gates:")
    for gate in migration_map_model.GATE_NAMES:
        print(f"{gate}\t{gates[gate]}")
    return 0


def cmd_migration_next(args: argparse.Namespace) -> int:
    """Показывает первую незавершённую карточку и не меняет карту."""

    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    value, errors = load_validated_migration_map(repo_root, args.version)
    if errors or value is None:
        print_migration_errors(errors)
        return 1
    card = migration_map_model.next_open_card(value)
    if card is None:
        print("none")
        return 0
    print(f"{card['id']}\t{card['status']}\t{card['path']}")
    return 0


def _updated_map(args: argparse.Namespace, *, card: bool) -> int:
    """Обновляет одну выбранную запись карточки или gate и записывает карту."""

    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    value, errors = load_validated_migration_map(repo_root, args.version)
    if errors or value is None:
        print_migration_errors(errors)
        return 1
    active_error = active_migration_error(value)
    if active_error:
        print_migration_errors([active_error])
        return 1
    try:
        if card:
            updated = migration_map_model.with_card_status(
                value,
                selector=args.card,
                status=args.status,
            )
            subject = args.card
        else:
            updated = migration_map_model.with_gate_status(
                value,
                gate=args.gate,
                status=args.status,
            )
            subject = args.gate
    except ValueError as exc:
        print_migration_errors([str(exc)])
        return 2
    errors = write_validated_migration_map(
        repo_root=repo_root,
        version=args.version,
        value=updated,
    )
    if errors:
        print_migration_errors(errors)
        return 1
    print(f"updated: {subject} -> {args.status}")
    return 0


def cmd_migration_set_card_status(args: argparse.Namespace) -> int:
    """Обновляет статус ровно одной явно выбранной карточки."""

    return _updated_map(args, card=True)


def cmd_migration_set_gate_status(args: argparse.Namespace) -> int:
    """Обновляет статус одного фиксированного migration gate."""

    return _updated_map(args, card=False)


def cmd_migration_validate(args: argparse.Namespace) -> int:
    """Проверяет JSON-контракт и owner-card связи без изменения файла."""

    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    _, errors = load_validated_migration_map(repo_root, args.version)
    if errors:
        print_migration_errors(errors)
        print("RESULT: blocked")
        return 1
    print("RESULT: ok")
    return 0


def cmd_migration_complete(args: argparse.Namespace) -> int:
    """Явно завершает карту только после финальных карточек и gates."""

    repo_root = Path(args.repo_root).resolve() if args.repo_root else find_repo_root()
    value, errors = load_validated_migration_map(repo_root, args.version)
    if errors or value is None:
        print_migration_errors(errors)
        return 1
    try:
        updated = migration_map_model.completed_migration_map(value)
    except ValueError as exc:
        print_migration_errors([str(exc)])
        return 1
    errors = write_validated_migration_map(
        repo_root=repo_root,
        version=args.version,
        value=updated,
    )
    if errors:
        print_migration_errors(errors)
        return 1
    print("RESULT: completed")
    return 0


def add_migration_parser(sub: argparse._SubParsersAction) -> None:
    """Регистрирует закрытый набор ``fork migration`` subcommands."""

    migration = sub.add_parser("migration")
    migration_sub = migration.add_subparsers(dest="migration_command", required=True)

    migration_init = migration_sub.add_parser("init")
    migration_init.add_argument("--repo-root")
    migration_init.add_argument("--version", type=migration_version, required=True)
    migration_init.set_defaults(func=cmd_migration_init)

    migration_show = migration_sub.add_parser("show")
    migration_show.add_argument("--repo-root")
    migration_show.add_argument("--version", type=migration_version, required=True)
    migration_show.set_defaults(func=cmd_migration_show)

    migration_next = migration_sub.add_parser("next")
    migration_next.add_argument("--repo-root")
    migration_next.add_argument("--version", type=migration_version, required=True)
    migration_next.set_defaults(func=cmd_migration_next)

    migration_card_status = migration_sub.add_parser("set-card-status")
    migration_card_status.add_argument("--repo-root")
    migration_card_status.add_argument(
        "--version", type=migration_version, required=True
    )
    migration_card_status.add_argument("--card", required=True)
    migration_card_status.add_argument(
        "--status",
        choices=migration_map_model.CARD_STATUSES,
        required=True,
    )
    migration_card_status.set_defaults(func=cmd_migration_set_card_status)

    migration_gate_status = migration_sub.add_parser("set-gate-status")
    migration_gate_status.add_argument("--repo-root")
    migration_gate_status.add_argument(
        "--version", type=migration_version, required=True
    )
    migration_gate_status.add_argument(
        "--gate",
        choices=migration_map_model.GATE_NAMES,
        required=True,
    )
    migration_gate_status.add_argument(
        "--status",
        choices=migration_map_model.GATE_STATUSES,
        required=True,
    )
    migration_gate_status.set_defaults(func=cmd_migration_set_gate_status)

    migration_validate = migration_sub.add_parser("validate")
    migration_validate.add_argument("--repo-root")
    migration_validate.add_argument("--version", type=migration_version, required=True)
    migration_validate.set_defaults(func=cmd_migration_validate)

    migration_complete = migration_sub.add_parser("complete")
    migration_complete.add_argument("--repo-root")
    migration_complete.add_argument("--version", type=migration_version, required=True)
    migration_complete.set_defaults(func=cmd_migration_complete)
