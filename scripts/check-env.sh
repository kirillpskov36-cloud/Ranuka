#!/usr/bin/env bash
# Проверяет, что установлены инструменты, нужные для работы над Ranuka.
# Запуск:  ./scripts/check-env.sh
#
# Обязательные инструменты нужны с темы 1.
# Solana CLI и Anchor нужны только с темы 2 — их отсутствие не ошибка.

set -u

missing_required=0

# check <команда> <аргумент версии> <required|optional> <пояснение>
check() {
    local cmd="$1" version_arg="$2" kind="$3" note="$4"
    if command -v "$cmd" >/dev/null 2>&1; then
        # Берём первую строку вывода версии.
        local version
        version="$("$cmd" "$version_arg" 2>/dev/null | head -n 1)"
        printf '  [ok]   %-8s %s\n' "$cmd" "$version"
    elif [ "$kind" = "required" ]; then
        printf '  [НЕТ]  %-8s %s\n' "$cmd" "$note"
        missing_required=1
    else
        printf '  [позже] %-7s %s\n' "$cmd" "$note"
    fi
}

echo "Обязательно (с темы 1):"
check rustc   --version required "установи Rust через rustup"
check cargo   --version required "ставится вместе с Rust"
check rustfmt --version required "rustup component add rustfmt"
check git     --version required "установи git"

# clippy — подкоманда cargo, отдельного бинарника в PATH может не быть.
if cargo clippy --version >/dev/null 2>&1; then
    printf '  [ok]   %-8s %s\n' "clippy" "$(cargo clippy --version 2>/dev/null | head -n 1)"
else
    printf '  [НЕТ]  %-8s %s\n' "clippy" "rustup component add clippy"
    missing_required=1
fi

echo
echo "Понадобится с темы 2:"
check solana --version optional "Solana CLI, см. README"
check anchor --version optional "Anchor CLI, см. README"

echo
if [ "$missing_required" -eq 0 ]; then
    echo "Всё обязательное на месте."
else
    echo "Не хватает обязательных инструментов (отмечены [НЕТ])."
    exit 1
fi
