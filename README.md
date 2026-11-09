# Ranuka

Launchpad на Solana с бондинговой кривой. Главный продукт — on-chain программа
и SDK для ботов; TUI — второй клиент поверх того же SDK.

**Статус:** готовы математика кривой (`ranuka-core`) и первая часть программы:
настройки платформы и управление ими. Создания токена, торговли, SDK и клиента
ещё нет.

## Документы

- [docs/architecture.md](docs/architecture.md) — как устроен проект. Начинать отсюда.
- [docs/decisions.md](docs/decisions.md) — журнал решений: что выбрали и почему.
- [CLAUDE.md](CLAUDE.md) — правила работы над кодом.

## Команды

```sh
./scripts/check.sh        # все проверки: сборка программы, форматирование, линтер, тесты
./scripts/check-env.sh    # что установлено на машине
cargo run -p ranuka-cli   # запустить бинарник
```

Без Solana CLI и Anchor `check.sh` проверяет всё, кроме тестов программы,
и пишет об этом в конце. В CI они запускаются всегда.

## Окружение

Для математики (`ranuka-core`) достаточно Rust и git.

```sh
# Arch Linux: sudo pacman -S rustup && rustup default stable
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup component add rustfmt clippy
```

Solana CLI и Anchor понадобятся, когда начнётся работа над программой:

- https://solana.com/docs/intro/installation
- https://solana.com/docs/intro/installation/dependencies
