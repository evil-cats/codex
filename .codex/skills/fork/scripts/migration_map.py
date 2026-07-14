"""Машинная карта одноразовой миграции Hermione fork.

Модуль владеет форматом ``fork-migration.v1``: создаёт минимальный JSON из
переданного снимка активных owner-карточек, проверяет закрытые enum и структуру,
вычисляет прогресс и выполняет атомарные обновления одной записи. Он не ищет
старые карты, не принимает решение о возобновлении миграции и не запускает
агентов или внешние команды. Единственный файловый side effect — явная запись
карты по пути, переданному вызывающим skill-owned CLI.
"""

import copy
import json
import os
import re
from collections.abc import Callable, Mapping, Sequence
from pathlib import Path


MIGRATION_MAP_SCHEMA = "fork-migration.v1"
MIGRATION_VERSION_RE = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")
MIGRATION_STATUSES = ("active", "completed")
CARD_STATUSES = (
    "pending",
    "inProgress",
    "migrated",
    "notApplicable",
    "reverted",
    "blocked",
    "needsFix",
)
FINAL_CARD_STATUSES = ("migrated", "notApplicable", "reverted")
GATE_NAMES = (
    "preflight",
    "format",
    "generators",
    "cardsValidate",
    "cardTests",
    "fullTests",
    "buildFast",
    "install",
)
GATE_STATUSES = ("pending", "passed", "failed", "skipped")
INITIAL_GATE_STATUSES = {
    "preflight": "pending",
    "format": "pending",
    "generators": "pending",
    "cardsValidate": "pending",
    "cardTests": "pending",
    "fullTests": "skipped",
    "buildFast": "pending",
    "install": "skipped",
}


def validate_migration_version(version: str) -> str:
    """Возвращает безопасную версию ``X.Y.Z`` или поднимает ``ValueError``."""

    if not MIGRATION_VERSION_RE.fullmatch(version):
        raise ValueError("migration version must have the form X.Y.Z")
    return version


def migration_map_path(repo_root: Path, version: str) -> Path:
    """Возвращает канонический путь карты для явно выбранной версии."""

    validate_migration_version(version)
    return repo_root / "docs/fork/migration" / f"{version}.json"


def new_migration_map(
    version: str, cards: Sequence[Mapping[str, str]]
) -> dict[str, object]:
    """Создаёт новую активную карту из переданного снимка owner-карточек.

    ``cards`` обязаны содержать только ``id`` и ``path``. Функция сортирует
    записи по пути, присваивает каждой статус ``pending`` и не обращается к
    файловой системе. Полная проверка результата выполняется отдельным
    валидатором, чтобы один контракт использовался и для новых, и для загруженных
    карт.
    """

    validate_migration_version(version)
    return {
        "schema": MIGRATION_MAP_SCHEMA,
        "migration": {
            "version": version,
            "upstreamTag": f"rust-v{version}",
            "branch": f"hermione-{version}",
            "status": "active",
        },
        "cards": [
            {
                "id": card["id"],
                "path": card["path"],
                "status": "pending",
            }
            for card in sorted(cards, key=lambda card: card["path"])
        ],
        "gates": dict(INITIAL_GATE_STATUSES),
    }


def _exact_key_errors(value: object, expected: set[str], label: str) -> list[str]:
    """Проверяет, что JSON-объект содержит ровно ожидаемый набор ключей."""

    if not isinstance(value, Mapping):
        return [f"{label} must be an object"]
    actual = set(value)
    errors = []
    missing = sorted(expected - actual)
    unexpected = sorted(actual - expected)
    if missing:
        errors.append(f"{label} missing keys: {', '.join(missing)}")
    if unexpected:
        errors.append(f"{label} unexpected keys: {', '.join(unexpected)}")
    return errors


def _string_field_errors(
    value: Mapping[str, object],
    field: str,
    label: str,
) -> list[str]:
    """Проверяет обязательное непустое строковое поле JSON-объекта."""

    field_value = value.get(field)
    if not isinstance(field_value, str) or not field_value:
        return [f"{label}.{field} must be a non-empty string"]
    return []


def _completed_state_errors(value: Mapping[str, object]) -> list[str]:
    """Проверяет инварианты уже помеченной ``completed`` карты."""

    errors = []
    cards = value.get("cards")
    if isinstance(cards, list):
        for index, card in enumerate(cards):
            if (
                isinstance(card, Mapping)
                and card.get("status") not in FINAL_CARD_STATUSES
            ):
                errors.append(f"cards[{index}] is not final")
    gates = value.get("gates")
    if isinstance(gates, Mapping):
        for gate in GATE_NAMES:
            if gates.get(gate) not in ("passed", "skipped"):
                errors.append(f"gates.{gate} is not final")
    return errors


def migration_map_validation_errors(
    value: object,
    *,
    expected_version: str | None = None,
) -> list[str]:
    """Возвращает все структурные ошибки карты без изменения входных данных."""

    errors = _exact_key_errors(value, {"schema", "migration", "cards", "gates"}, "root")
    if not isinstance(value, Mapping):
        return errors

    if value.get("schema") != MIGRATION_MAP_SCHEMA:
        errors.append(f"schema must be {MIGRATION_MAP_SCHEMA}")

    migration = value.get("migration")
    errors.extend(
        _exact_key_errors(
            migration,
            {"version", "upstreamTag", "branch", "status"},
            "migration",
        )
    )
    if isinstance(migration, Mapping):
        for field in ("version", "upstreamTag", "branch", "status"):
            errors.extend(_string_field_errors(migration, field, "migration"))
        version = migration.get("version")
        if isinstance(version, str) and version:
            if not MIGRATION_VERSION_RE.fullmatch(version):
                errors.append("migration.version must have the form X.Y.Z")
            if expected_version is not None and version != expected_version:
                errors.append(
                    f"migration.version must match requested version {expected_version}"
                )
            if migration.get("upstreamTag") != f"rust-v{version}":
                errors.append("migration.upstreamTag does not match migration.version")
            if migration.get("branch") != f"hermione-{version}":
                errors.append("migration.branch does not match migration.version")
        if migration.get("status") not in MIGRATION_STATUSES:
            errors.append(
                f"migration.status must be one of: {', '.join(MIGRATION_STATUSES)}"
            )

    cards = value.get("cards")
    card_ids = []
    card_paths = []
    if not isinstance(cards, list) or not cards:
        errors.append("cards must be a non-empty array")
    else:
        for index, card in enumerate(cards):
            label = f"cards[{index}]"
            errors.extend(_exact_key_errors(card, {"id", "path", "status"}, label))
            if not isinstance(card, Mapping):
                continue
            for field in ("id", "path", "status"):
                errors.extend(_string_field_errors(card, field, label))
            card_id = card.get("id")
            card_path = card.get("path")
            if isinstance(card_id, str):
                card_ids.append(card_id)
            if isinstance(card_path, str):
                card_paths.append(card_path)
            if card.get("status") not in CARD_STATUSES:
                errors.append(
                    f"{label}.status must be one of: {', '.join(CARD_STATUSES)}"
                )
        if len(card_ids) != len(set(card_ids)):
            errors.append("card ids must be unique")
        if len(card_paths) != len(set(card_paths)):
            errors.append("card paths must be unique")
        if card_paths != sorted(card_paths):
            errors.append("cards must be sorted by path")

    gates = value.get("gates")
    errors.extend(_exact_key_errors(gates, set(GATE_NAMES), "gates"))
    if isinstance(gates, Mapping):
        for gate in GATE_NAMES:
            status = gates.get(gate)
            if status not in GATE_STATUSES:
                errors.append(
                    f"gates.{gate} must be one of: {', '.join(GATE_STATUSES)}"
                )

    if isinstance(migration, Mapping) and migration.get("status") == "completed":
        errors.extend(_completed_state_errors(value))
    return errors


def repository_validation_errors(
    value: object,
    *,
    repo_root: Path,
    card_id: Callable[[Path], str],
) -> list[str]:
    """Проверяет связи структурно корректной карты с owner-карточками checkout.

    Проверка намеренно не требует, чтобы owner-карточка всё ещё имела статус
    ``active``: карта является снимком scope в момент ``init``, а карточка может
    стать ``reverted`` в ходе этой же миграции.
    """

    errors = migration_map_validation_errors(value)
    if errors or not isinstance(value, Mapping):
        return errors
    cards = value["cards"]
    assert isinstance(cards, list)
    for index, card in enumerate(cards):
        assert isinstance(card, Mapping)
        relative = Path(str(card["path"]))
        if (
            relative.is_absolute()
            or len(relative.parts) != 3
            or relative.parts[:2] != ("docs", "fork")
            or relative.suffix != ".md"
            or relative.name.startswith("migration-")
        ):
            errors.append(f"cards[{index}].path is not an owner-card path")
            continue
        path = repo_root / relative
        if not path.is_file():
            errors.append(f"cards[{index}].path does not exist: {relative}")
            continue
        actual_id = card_id(path)
        if actual_id != card["id"]:
            errors.append(
                f"cards[{index}].id does not match {relative}: "
                f"expected {actual_id or '<missing>'}"
            )
    return errors


def read_migration_map(path: Path) -> dict[str, object]:
    """Читает JSON-карту или поднимает ``ValueError`` с короткой причиной."""

    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise ValueError(f"could not read migration map: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise ValueError(
            f"invalid migration map JSON at line {exc.lineno}, column {exc.colno}"
        ) from exc
    if not isinstance(value, dict):
        raise ValueError("migration map root must be an object")
    return value


def write_migration_map_atomic(path: Path, value: Mapping[str, object]) -> None:
    """Стабильно сериализует карту и атомарно заменяет целевой файл.

    Временный файл создаётся рядом с целью, поэтому ``os.replace`` не пересекает
    границы файловых систем. При ошибке временный файл удаляется, а существующая
    карта остаётся неизменной.
    """

    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    try:
        temporary.write_text(
            json.dumps(value, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def migration_progress(value: Mapping[str, object]) -> tuple[int, int, int]:
    """Возвращает ``processed``, ``total`` и ``remaining`` по статусам карточек."""

    cards = value.get("cards")
    assert isinstance(cards, list)
    total = len(cards)
    processed = sum(
        1
        for card in cards
        if isinstance(card, Mapping) and card.get("status") in FINAL_CARD_STATUSES
    )
    return processed, total, total - processed


def next_open_card(value: Mapping[str, object]) -> Mapping[str, object] | None:
    """Возвращает первую незавершённую запись, не меняя карту."""

    cards = value.get("cards")
    assert isinstance(cards, list)
    for card in cards:
        if isinstance(card, Mapping) and card.get("status") not in FINAL_CARD_STATUSES:
            return card
    return None


def with_card_status(
    value: Mapping[str, object],
    *,
    selector: str,
    status: str,
) -> dict[str, object]:
    """Возвращает копию карты с новым статусом ровно одной карточки.

    ``selector`` совпадает либо с ``id``, либо с полным ``path``. Неизвестный или
    неоднозначный selector и недопустимый статус приводят к ``ValueError``.
    """

    if status not in CARD_STATUSES:
        raise ValueError(f"unknown card status: {status}")
    updated = copy.deepcopy(value)
    cards = updated.get("cards")
    assert isinstance(cards, list)
    matches = [
        card
        for card in cards
        if isinstance(card, dict) and selector in (card.get("id"), card.get("path"))
    ]
    if len(matches) != 1:
        raise ValueError(f"card selector must match exactly one entry: {selector}")
    matches[0]["status"] = status
    return updated


def with_gate_status(
    value: Mapping[str, object],
    *,
    gate: str,
    status: str,
) -> dict[str, object]:
    """Возвращает копию карты с новым статусом одного фиксированного gate."""

    if gate not in GATE_NAMES:
        raise ValueError(f"unknown migration gate: {gate}")
    if status not in GATE_STATUSES:
        raise ValueError(f"unknown gate status: {status}")
    updated = copy.deepcopy(value)
    gates = updated.get("gates")
    assert isinstance(gates, dict)
    gates[gate] = status
    return updated


def completion_errors(value: object) -> list[str]:
    """Возвращает причины, по которым активную карту нельзя завершить."""

    errors = migration_map_validation_errors(value)
    if errors or not isinstance(value, Mapping):
        return errors
    migration = value["migration"]
    assert isinstance(migration, Mapping)
    if migration.get("status") != "active":
        errors.append("migration.status must be active before completion")
    errors.extend(_completed_state_errors(value))
    return errors


def completed_migration_map(value: Mapping[str, object]) -> dict[str, object]:
    """Возвращает завершённую копию карты после проверки всех финальных gates."""

    errors = completion_errors(value)
    if errors:
        raise ValueError("; ".join(errors))
    updated = copy.deepcopy(value)
    migration = updated.get("migration")
    assert isinstance(migration, dict)
    migration["status"] = "completed"
    return updated
