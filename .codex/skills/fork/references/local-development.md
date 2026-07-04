# Local development checkout

Статус: `перенесено`.

Этот файл заменяет host-specific формулировку текущего legacy workflow на
переносимое правило локального checkout.

## Правило

Source of truth для fork-работы - текущий локальный checkout репозитория, в
котором агент выполняет разработку, генераторы, форматирование, тесты, сборку,
diff review, staging и commit.

Не строй обычный workflow вокруг подготовки patch в одном checkout и переноса в
другой checkout, если пользователь явно не попросил такой режим.

## Рабочий порядок

1. Перед существенным шагом проверь фактическое состояние checkout:
   `git status --short --branch`.
2. Если нужен свежий upstream, обновляй refs в этом же checkout.
3. После создания нового source/task-owned файла добавь его в Git index минимум
   через `git add -N <path>`, чтобы `git diff` и review видели содержимое.
   Не добавляй build artifacts, logs, cache, temporary output и unrelated
   untracked files.
4. Если генератор, форматирование, тест или сборка изменили файлы, проверяй
   diff в этом же checkout.
5. Destructive sync/reset операции сначала ограничь inspect/report и выполняй
   только после отдельного подтверждения пользователя.

## Host-specific details

Не закрепляй в skill обязательный server name, hostname или абсолютный
пользовательский путь. Такие сведения могут жить в памяти или локальном
операционном handoff, но не в переносимом fork skill.
