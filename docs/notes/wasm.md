# Проверка WASM

2026-10-28, Rust 1.94.0.

Ядро отделяет файловую систему feature-флагом `fs` (включён по умолчанию).
Без него доступны криптография, модели Loro, поиск и разбор ссылок.

Сборка под браузер **проходит**:

```
cargo check -p crypt-core --target wasm32-unknown-unknown --no-default-features --features wasm
```

Что для этого нужно:

- feature `wasm` включает `getrandom/js` (getrandom 0.2, его тянут `rand` 0.8 и `chacha20poly1305`)
  и `getrandom03` с фичей `wasm_js` (getrandom 0.3, его тянет `ulid` → `rand` 0.9);
- `.cargo/config.toml` задаёт для `wasm32-unknown-unknown` флаг
  `--cfg getrandom_backend="wasm_js"`, без него getrandom 0.3 не собирается.

Без `fs` остаётся одно предупреждение: приватные методы секретов в `RecordDoc`
вызывает только файловый фасад `Vault`. На обычную сборку и на Clippy с
`-D warnings` это не влияет.
