# Crypt Core (план 1 из 3) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rust-крейт `crypt-core` — зашифрованное хранилище Crypt: сид-фраза и ключи, шифрование файлов, записи/типы/папки/секреты в CRDT (Loro), корзина и полная вычистка секретов, поиск, wiki-ссылки и бэклинки. Без UI; всё покрыто тестами.

**Architecture:** Хранилище — папка: открытый заголовок `vault.json` + зашифрованные файлы. Каждая запись (включая секрет) — отдельный Loro-документ в своём файле `objects/<id>.bin`, поэтому удаление секрета = удаление файла. Типы, папки, виды и надгробия — в одном документе `meta.bin`. Ключи выводятся из BIP39-сид-фразы через HKDF; каждый файл — конверт XChaCha20-Poly1305 с AAD, привязывающим его к роли и id. Индекс поиска и бэклинков строится в памяти при открытии.

**Tech Stack:** Rust 1.94 (edition 2024), `loro` 1.x, `bip39` 2.x (feature `rand`), `hkdf` 0.12 + `sha2` 0.10, `chacha20poly1305` 0.10, `rand` 0.8, `zeroize` 1 (feature `derive`), `ulid` 1 (feature `serde`), `serde`/`serde_json`, `thiserror` 2, `regex` 1; dev: `tempfile` 3.

**Spec:** `docs/specs/2026-10-28-crypt-design.md`

**Следующие планы (пишутся после этого):** план 2 — приложение Tauri 2 + Vue + shadcn-vue, разблокировка (Keychain + LocalAuthentication, PIN), экраны записи/таблицы/типов; план 3 — UI секретов, буфер обмена, генератор, TOTP, ssh-agent, импорт `~/.ssh`, меню-бар.

## Global Constraints

- Всё содержимое на диске зашифровано; открыто лежат только `vault.json` (версия формата, `vault_id`, `created_at`) и `device.json` (peer id устройства).
- Сид-фраза: BIP39, **24 слова**, английский словарь, BIP39-passphrase пустая.
- Шифр: XChaCha20-Poly1305, случайный 24-байтный nonce на каждую запись файла.
- Значения секретов никогда не попадают в индекс поиска, логи, `Debug`-вывод и сообщения ошибок.
- Удаление секрета = полная вычистка (файл и история значений), плюс надгробие в `meta`. Удаление обычной записи = корзина с восстановлением.
- При смене значения секрета прошлое значение сохраняется в его истории.
- Значения полей хранятся по `FieldId`, не по имени; значения удалённого из типа поля сохраняются скрытыми.
- Запись на диск атомарная (temp-файл в той же папке → fsync → rename), права файлов `0600`.
- Объём: до ~10 000 записей; всё держим в памяти.
- `crypt-core` без `tokio`, без сети, без платформенных API (Keychain и Touch ID — в плане 2), чтобы ядро оставалось пригодным для WASM.
- Никакой телеметрии и сетевых запросов.

## Review Focus

1. **Сид-фраза введена «грязно» или неверно:** лишние пробелы, переносы, заглавные буквы должны приниматься; валидная, но чужая фраза → `CryptError::WrongKey`, без паники и без мусорных данных. → тесты в Task 1 и Task 3.
2. **Сбой посреди записи:** оставшийся `*.tmp` в `objects/` не должен ломать открытие, прежние данные целы. → тест в Task 3 и Task 5.
3. **Один повреждённый файл записи:** хранилище всё равно открывается, остальные записи доступны, сломанная перечислена в `Vault::load_errors()`. → тест в Task 5.
4. **Удалённый секрет не восстановим:** файла нет, поиск и бэклинки его не находят, надгробие есть; поиск по точному значению пароля ничего не находит даже до удаления. → тесты в Task 6 и Task 7.
5. **Тип поменялся под существующими записями:** поле удалено или сменило вид — записи открываются, старое значение сохранено скрытым; `set_field` с несовпадающим видом → `CryptError::Validation`. → тесты в Task 4 и Task 5.

---

## Структура файлов

```
crypt/
  Cargo.toml                         # workspace
  .gitignore
  crates/crypt-core/
    Cargo.toml
    src/lib.rs                       # pub mod + реэкспорт Vault, CryptError
    src/error.rs                     # CryptError
    src/crypto/mod.rs
    src/crypto/secret_key.rs         # SecretKey (32 байта, zeroize)
    src/crypto/seed.rs               # SeedPhrase
    src/crypto/keys.rs               # VaultKeys, VaultId
    src/crypto/envelope.rs           # seal / open
    src/storage/mod.rs
    src/storage/fs.rs                # write_atomic, read_file
    src/storage/layout.rs            # VaultPaths, Aad
    src/model/mod.rs
    src/model/ids.rs                 # RecordId, TypeId, FieldId, FolderId, OptionId, ViewId
    src/model/types.rs               # TypeDef, FieldDef, FieldKind, FieldValue, starter_types()
    src/model/meta.rs                # MetaDoc (типы, папки, виды, надгробия)
    src/model/record.rs              # RecordDoc
    src/model/secret.rs              # SecretKind, имена значений
    src/index/mod.rs                 # Index, SearchHit
    src/index/links.rs               # parse_wikilinks
    src/vault.rs                     # Vault — единственный публичный фасад для плана 2
    tests/vault_e2e.rs               # сквозной тест
```

---

### Task 1: Каркас workspace, ошибки, сид-фраза и вывод ключей

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `crates/crypt-core/Cargo.toml`, `crates/crypt-core/src/lib.rs`, `src/error.rs`, `src/crypto/{mod,secret_key,seed,keys}.rs`
- Test: юнит-тесты внутри `seed.rs` и `keys.rs`

**Interfaces:**
- Produces:
  - `pub enum CryptError` (thiserror) с вариантами: `Io(#[from] std::io::Error)`, `BadMagic`, `UnsupportedEnvelope(u8)`, `Decrypt`, `InvalidSeed`, `WrongKey`, `VaultExists`, `NotAVault`, `UnsupportedFormat(u32)`, `NotFound(String)`, `Validation(String)`, `Crdt(String)`, `Json(#[from] serde_json::Error)`, `Corrupt { file: String }`. `pub type Result<T> = std::result::Result<T, CryptError>;`
  - `pub struct SecretKey([u8; 32])` — `#[derive(Zeroize, ZeroizeOnDrop)]`, `fn from_bytes([u8;32]) -> Self`, `fn as_bytes(&self) -> &[u8; 32]`, `Debug` печатает `SecretKey(***)`, `Clone`.
  - `pub struct SeedPhrase` (хранит `Zeroizing<String>`): `fn generate() -> SeedPhrase`, `fn parse(input: &str) -> Result<SeedPhrase>`, `fn words(&self) -> Vec<&str>`, `fn as_str(&self) -> &str`; `Debug` не печатает слова.
  - `pub struct VaultId(String)` (32 hex-символа, `Display`, serde как строка).
  - `pub struct VaultKeys { pub data_key: SecretKey, pub sync_key: SecretKey, pub vault_id: VaultId }`, `fn derive(seed: &SeedPhrase) -> VaultKeys`.

- [ ] **Step 1: Создать workspace и git-репозиторий**

`git init` в `/Users/anton/Documents/pets/crypt`; корневой `Cargo.toml` с `[workspace] members = ["crates/*"]`, `resolver = "3"`; `.gitignore`: `target/`, `node_modules/`, `.DS_Store`. Крейт `crypt-core` с зависимостями из Tech Stack, `edition = "2024"`.

- [ ] **Step 2: Написать падающие тесты для `SeedPhrase`**

```rust
#[test] fn generate_gives_24_valid_words() {
    let s = SeedPhrase::generate();
    assert_eq!(s.words().len(), 24);
    assert!(SeedPhrase::parse(s.as_str()).is_ok());
}
#[test] fn parse_normalizes_whitespace_and_case() {
    let messy = "  ABANDON abandon\nabandon abandon abandon abandon abandon abandon abandon abandon abandon abandon\tabandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon ART ";
    let s = SeedPhrase::parse(messy).unwrap();
    assert_eq!(s.words().len(), 24);
    assert_eq!(s.words()[23], "art");
}
#[test] fn parse_rejects_bad_checksum_and_wrong_length() {
    assert!(matches!(SeedPhrase::parse(&"abandon ".repeat(24)), Err(CryptError::InvalidSeed)));
    assert!(matches!(SeedPhrase::parse(&"abandon ".repeat(11)), Err(CryptError::InvalidSeed)));
}
#[test] fn debug_does_not_leak_words() {
    assert!(!format!("{:?}", SeedPhrase::generate()).contains(' '));
}
```

Принимаются только фразы из 24 слов (12-словные отклоняются).

- [ ] **Step 3: Запустить и убедиться, что падают**

Run: `cargo test -p crypt-core seed`
Expected: ошибки компиляции / FAIL — `SeedPhrase` не определён.

- [ ] **Step 4: Реализовать `SecretKey`, `CryptError`, `SeedPhrase`**

`parse`: lowercase, split по whitespace, join одним пробелом, `bip39::Mnemonic::parse_in(Language::English, ..)`, проверка `word_count() == 24`.

- [ ] **Step 5: Написать падающие тесты для `VaultKeys::derive`**

```rust
const ABANDON_ART: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
#[test] fn derive_is_deterministic_and_keys_are_distinct() {
    let s = SeedPhrase::parse(ABANDON_ART).unwrap();
    let a = VaultKeys::derive(&s); let b = VaultKeys::derive(&s);
    assert_eq!(a.data_key.as_bytes(), b.data_key.as_bytes());
    assert_ne!(a.data_key.as_bytes(), a.sync_key.as_bytes());
    assert_eq!(a.vault_id.to_string().len(), 32);
}
#[test] fn derive_golden_values() {
    // Зафиксировать при первом прогоне: формат ключей нельзя менять без миграции.
    let k = VaultKeys::derive(&SeedPhrase::parse(ABANDON_ART).unwrap());
    assert_eq!(k.vault_id.to_string(), GOLDEN_VAULT_ID);
    assert_eq!(hex(k.data_key.as_bytes()), GOLDEN_DATA_KEY);
}
```

- [ ] **Step 6: Реализовать `derive`**

`bip39_seed = mnemonic.to_seed("")` (64 байта); `Hkdf::<Sha256>::new(Some(b"crypt/v1"), &bip39_seed)`; `expand` с info `b"crypt/v1/data-key"` → 32 байта, `b"crypt/v1/sync-auth"` → 32 байта, `b"crypt/v1/vault-id"` → 16 байт → lowercase hex. Промежуточные буферы — в `Zeroizing`. Запустить `derive_golden_values` один раз, вставить напечатанные значения в `GOLDEN_*`.

- [ ] **Step 7: Прогнать тесты**

Run: `cargo test -p crypt-core`
Expected: все PASS.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(core): workspace, errors, BIP39 seed and HKDF key derivation"
```

---

### Task 2: Конверт шифрования

**Files:**
- Create: `crates/crypt-core/src/crypto/envelope.rs`
- Test: юнит-тесты в том же файле

**Interfaces:**
- Consumes: `SecretKey`, `CryptError` (Task 1)
- Produces:
  - `pub fn seal(key: &SecretKey, aad: &[u8], plaintext: &[u8]) -> Vec<u8>`
  - `pub fn open(key: &SecretKey, aad: &[u8], envelope: &[u8]) -> Result<Zeroizing<Vec<u8>>>`
  - Формат: `b"CRPT"` (4) ‖ версия `0x01` (1) ‖ nonce (24) ‖ ciphertext+tag.

- [ ] **Step 1: Написать падающие тесты**

```rust
#[test] fn roundtrip() { let k = key(1); let e = seal(&k, b"a", b"hello"); assert_eq!(&*open(&k, b"a", &e).unwrap(), b"hello"); }
#[test] fn nonce_is_random() { let k = key(1); assert_ne!(seal(&k, b"a", b"x"), seal(&k, b"a", b"x")); }
#[test] fn wrong_key_fails() { let e = seal(&key(1), b"a", b"x"); assert!(matches!(open(&key(2), b"a", &e), Err(CryptError::Decrypt))); }
#[test] fn wrong_aad_fails() { let k = key(1); let e = seal(&k, b"a", b"x"); assert!(matches!(open(&k, b"b", &e), Err(CryptError::Decrypt))); }
#[test] fn tamper_fails() { let k = key(1); let mut e = seal(&k, b"a", b"x"); let n = e.len()-1; e[n] ^= 1; assert!(matches!(open(&k, b"a", &e), Err(CryptError::Decrypt))); }
#[test] fn bad_magic_version_and_truncation_do_not_panic() {
    let k = key(1); let e = seal(&k, b"a", b"x");
    assert!(matches!(open(&k, b"a", b"NOPE"), Err(CryptError::BadMagic)));
    let mut v = e.clone(); v[4] = 9; assert!(matches!(open(&k, b"a", &v), Err(CryptError::UnsupportedEnvelope(9))));
    assert!(open(&k, b"a", &e[..20]).is_err());
}
```

`key(n)` — тестовый хелпер `SecretKey::from_bytes([n; 32])`.

- [ ] **Step 2: Run** `cargo test -p crypt-core envelope` → FAIL (не определено).
- [ ] **Step 3: Реализовать `seal`/`open`** на `chacha20poly1305::XChaCha20Poly1305` с `Payload { msg, aad }`; nonce из `rand::rngs::OsRng`. Вход короче 29 байт → `BadMagic`, если не начинается с `CRPT`, иначе `Decrypt`.
- [ ] **Step 4: Run** `cargo test -p crypt-core envelope` → PASS.
- [ ] **Step 5: Commit** `git commit -am "feat(core): XChaCha20-Poly1305 envelope"` (с новым файлом через `git add`).

---

### Task 3: Файлы хранилища, создание и открытие

**Files:**
- Create: `src/storage/{mod,fs,layout}.rs`, `src/vault.rs` (начальная версия)
- Test: юнит-тесты в `fs.rs`, `vault.rs`

**Interfaces:**
- Consumes: `seal`/`open` (Task 2), `SeedPhrase`, `VaultKeys`, `SecretKey` (Task 1)
- Produces:
  - `pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()>`; temp-файл `<name>.<random>.tmp` в той же папке, `sync_all`, `rename`, права `0o600`.
  - `pub struct VaultPaths { root: PathBuf }`: `header()` → `vault.json`, `device()` → `device.json`, `keycheck()` → `keycheck.bin`, `seed()` → `seed.bin`, `meta()` → `meta.bin`, `objects_dir()` → `objects/`, `object(&RecordId)` → `objects/<id>.bin`.
  - `pub enum Aad { KeyCheck, Seed, Meta, Object(RecordId) }`, `fn bytes(&self) -> Vec<u8>` → `crypt/v1/keycheck`, `crypt/v1/seed`, `crypt/v1/meta`, `crypt/v1/object/<id>`. (`RecordId` — определить здесь минимально в `model/ids.rs`, Task 4 его дополняет.)
  - `vault.json`: `{"format":1,"vault_id":"<hex>","created_at":<unix ms>}`; `device.json`: `{"peer_id":<u64>}` — создаётся при первом открытии на устройстве, если нет.
  - `keycheck.bin` = `seal(data_key, Aad::KeyCheck, b"crypt-keycheck-v1")`; `seed.bin` = `seal(data_key, Aad::Seed, seed_phrase_bytes)`.
  - `pub struct Vault` с методами:
    - `fn create(root: &Path, seed: &SeedPhrase) -> Result<Vault>` — `root` не существует или пуст, иначе `VaultExists`.
    - `fn open_with_seed(root: &Path, seed: &SeedPhrase) -> Result<Vault>`
    - `fn open_with_key(root: &Path, data_key: SecretKey) -> Result<Vault>` — расшифровывает `seed.bin` и заново выводит все ключи (путь для Keychain в плане 2).
    - `fn data_key(&self) -> &SecretKey`, `fn vault_id(&self) -> &VaultId`, `fn reveal_seed(&self) -> Result<SeedPhrase>`, `fn lock(self)` (drop; ключи обнуляются через `ZeroizeOnDrop`).
  - Ошибки открытия: нет `vault.json` → `NotAVault`; `format != 1` → `UnsupportedFormat(n)`; keycheck не расшифровывается → `WrongKey`.

- [ ] **Step 1: Падающие тесты `write_atomic`**

```rust
#[test] fn write_atomic_replaces_and_sets_0600() { /* пишет "a", затем "b"; читает "b"; mode & 0o777 == 0o600 */ }
#[test] fn write_atomic_leaves_no_tmp_files() { /* после записи в папке ровно один файл */ }
```

- [ ] **Step 2: Падающие тесты `Vault`**

```rust
#[test] fn create_then_open_with_seed() {
    let dir = tempdir(); let seed = SeedPhrase::generate();
    let id = Vault::create(dir.path(), &seed).unwrap().vault_id().clone();
    assert_eq!(Vault::open_with_seed(dir.path(), &seed).unwrap().vault_id(), &id);
}
#[test] fn open_with_wrong_seed_is_wrong_key() { /* другая валидная фраза → Err(WrongKey) */ }
#[test] fn open_with_key_restores_seed() { /* open_with_key(data_key.clone()) → reveal_seed() == исходная фраза */ }
#[test] fn create_refuses_non_empty_dir() { /* второй create в ту же папку → Err(VaultExists) */ }
#[test] fn open_non_vault_dir() { /* пустая папка → Err(NotAVault) */ }
#[test] fn unsupported_format() { /* переписать vault.json с "format":2 → Err(UnsupportedFormat(2)) */ }
#[test] fn stray_tmp_files_are_ignored() { /* положить objects/garbage.123.tmp → open_with_seed Ok */ }
#[test] fn nothing_on_disk_contains_seed_words() { /* прочитать все файлы папки байтами: ни одно слово фразы длиной ≥5 не встречается */ }
```

- [ ] **Step 3: Run** `cargo test -p crypt-core vault storage` → FAIL.
- [ ] **Step 4: Реализовать `fs.rs`, `layout.rs`, `Vault::{create, open_with_seed, open_with_key, data_key, vault_id, reveal_seed, lock}`**. На этом шаге `Vault` хранит `paths`, `keys`, `peer_id`; `meta` и записи появляются в Task 4–5.
- [ ] **Step 5: Run** `cargo test -p crypt-core` → PASS.
- [ ] **Step 6: Commit** `feat(core): vault layout, atomic writes, create/open/unlock`.

---

### Task 4: Типы, папки, виды — `MetaDoc`

**Files:**
- Create/Modify: `src/model/{mod,ids,types,meta}.rs`; Modify: `src/vault.rs`
- Test: юнит-тесты в `types.rs`, `meta.rs`

**Interfaces:**
- Consumes: `seal`/`open`, `write_atomic`, `VaultPaths`, `Aad::Meta`
- Produces:
  - Id-типы (newtype над `Ulid`, serde-строка, `new()`, `Display`, `FromStr`): `RecordId`, `TypeId`, `FieldId`, `FolderId`, `OptionId`, `ViewId`.
  - ```rust
    pub enum FieldKind { Text, Number, Date, Url, Checkbox,
        Select { options: Vec<SelectOption> }, MultiSelect { options: Vec<SelectOption> },
        RecordLink { target_type: Option<TypeId>, multiple: bool },
        SecretLink { multiple: bool } }
    pub struct SelectOption { pub id: OptionId, pub label: String, pub color: Option<String> }
    pub struct FieldDef { pub id: FieldId, pub name: String, pub kind: FieldKind }
    pub struct TypeDef { pub id: TypeId, pub name: String, pub icon: String, pub fields: Vec<FieldDef> }
    pub enum FieldValue { Text(String), Number(f64), Date(String), Url(String), Checkbox(bool),
        Select(OptionId), MultiSelect(Vec<OptionId>), Links(Vec<RecordId>) }
    impl FieldValue { pub fn fits(&self, kind: &FieldKind) -> bool }
    pub struct Folder { pub id: FolderId, pub name: String, pub parent: Option<FolderId> }
    pub struct ViewDef { pub id: ViewId, pub type_id: TypeId, pub name: String, pub config: serde_json::Value }
    pub fn starter_types() -> Vec<TypeDef>  // «Сервер»: Hostname(Text), IP(Text), ОС(Text), Провайдер(Text), SSH-порт(Number), SSH-пользователь(Text), Креды(SecretLink{multiple:true})
    ```
    Все — `Serialize/Deserialize/Clone/Debug/PartialEq`. Enum-ы: `#[serde(tag = "kind", content = "value")]`.
  - `fits`: `Date` — строго `YYYY-MM-DD`; `Select` — id есть среди options; `MultiSelect` — все id есть; `Links` подходит к `RecordLink` и `SecretLink`, при `multiple: false` — не больше одного id; `Number` — конечное число.
  - `pub struct MetaDoc` (обёртка над `LoroDoc`; LoroMap `types`, `folders`, `views` — значения JSON-строки; LoroMap `purged` — `record_id → unix ms`):
    `fn new(peer: u64) -> Self`, `fn export(&self) -> Vec<u8>`, `fn import(peer: u64, bytes: &[u8]) -> Result<Self>`,
    `list_types() -> Vec<TypeDef>` (по имени), `get_type(&TypeId) -> Option<TypeDef>`, `put_type(TypeDef) -> Result<()>`, `remove_type(&TypeId)`,
    `list_folders() -> Vec<Folder>`, `put_folder(Folder) -> Result<()>`, `remove_folder(&FolderId)`,
    `list_views(&TypeId) -> Vec<ViewDef>`, `put_view(ViewDef)`, `remove_view(&ViewId)`,
    `add_tombstone(&RecordId, at_ms: i64)`, `tombstones() -> Vec<(RecordId, i64)>`.
  - Методы `Vault` (каждый изменяющий метод сразу сохраняет `meta.bin`): `types()`, `put_type(TypeDef) -> Result<()>`, `delete_type(&TypeId) -> Result<()>`, `folders()`, `create_folder(name: &str, parent: Option<FolderId>) -> Result<FolderId>`, `rename_folder`, `move_folder(&FolderId, Option<FolderId>) -> Result<()>`, `delete_folder(&FolderId) -> Result<()>`, `views(&TypeId)`, `put_view(ViewDef) -> Result<()>`, `delete_view(&ViewId) -> Result<()>`.

- [ ] **Step 1: Падающие тесты `FieldValue::fits`** — по одному утверждению на каждое правило выше; среди них `Date("2026-13-40")` → false, `Date("14.11.2026")` → false, `Links(vec![a, b])` к `SecretLink{multiple:false}` → false, `Number(f64::NAN)` → false.
- [ ] **Step 2: Падающие тесты `MetaDoc`/`Vault`**

```rust
#[test] fn put_type_validates() { /* пустое имя → Validation; два поля с одинаковым FieldId → Validation; дубли OptionId в одном select → Validation */ }
#[test] fn types_persist_across_reopen() { /* put_type(starter «Сервер») → reopen → types() содержит его с 7 полями в том же порядке */ }
#[test] fn folders_reject_cycles() { /* A → B (B внутри A); move_folder(A, Some(B)) → Validation */ }
#[test] fn delete_folder_with_children_rejected() { /* у папки есть подпапка → Validation */ }
#[test] fn views_roundtrip_opaque_config() { /* put_view с config {"sort":[["price","asc"]]} → views(type) возвращает тот же JSON */ }
#[test] fn meta_file_is_encrypted() { /* meta.bin не содержит байтов строки «Сервер» */ }
```

- [ ] **Step 3: Run** `cargo test -p crypt-core meta types` → FAIL.
- [ ] **Step 4: Реализовать** модели, `MetaDoc`, методы `Vault`. `Vault::create` пишет пустой `meta.bin`; открытие его читает. Проверки «папка не пуста» по записям и «тип используется» добавляются в Task 5.
- [ ] **Step 5: Run** `cargo test -p crypt-core` → PASS.
- [ ] **Step 6: Commit** `feat(core): types, fields, folders, views in encrypted meta doc`.

---

### Task 5: Записи — `RecordDoc` и CRUD в `Vault`

**Files:**
- Create: `src/model/record.rs`, `src/model/secret.rs` (только enum); Modify: `src/vault.rs`
- Test: юнит-тесты в `record.rs`, `vault.rs`

**Interfaces:**
- Consumes: id-типы, `TypeDef`, `FieldValue::fits` (Task 4); `seal`/`open`, `Aad::Object`, `write_atomic`
- Produces:
  - `RecordDoc` (Loro): LoroMap `meta` {`title`, `type_id`?, `folder_id`?, `secret_kind`?, `created_at`, `updated_at`, `deleted_at`?, `agent_enabled`?}, LoroMap `tags` (`tag → true`), LoroText `body`, LoroMap `fields` (`FieldId → JSON FieldValue`), LoroMap `secret` (имя → строка), LoroMap `secret_history` (имя → LoroList JSON `{value, replaced_at}`).
    Методы: `new(peer, id, title, now)`, `export()`, `import(peer, bytes) -> Result<Self>`, геттеры/сеттеры `title`, `body` (`set_body` через `LoroText::update`, чтобы будущий синк сливал по символам), `type_id`, `folder`, `tags`/`add_tag`/`remove_tag`, `field(&FieldId)`, `fields() -> BTreeMap<FieldId, FieldValue>`, `set_field(FieldId, Option<FieldValue>)`, `deleted_at`/`set_deleted_at`. Каждый сеттер обновляет `updated_at` и вызывает `commit()`.
  - Теги нормализуются: trim, срезать ведущий `#`, lowercase; пустой → `Validation`.
  - `pub enum SecretKind { Login, Card, Token, SshKey, Note }` (serde lowercase) — создать здесь в `src/model/secret.rs`; Task 6 добавляет к нему поведение.
  - `pub struct NewRecord { pub title: String, pub type_id: Option<TypeId>, pub folder: Option<FolderId>, pub secret_kind: Option<SecretKind> }` (в тестах этой задачи `secret_kind` всегда `None`).
  - `pub struct RecordView { pub id, pub title, pub body: String, pub type_id, pub folder, pub tags: Vec<String>, pub fields: BTreeMap<FieldId, FieldValue>, pub hidden_fields: BTreeMap<FieldId, FieldValue>, pub secret_kind: Option<SecretKind>, pub created_at: i64, pub updated_at: i64, pub deleted_at: Option<i64> }` — значения поля попадают в `hidden_fields`, если у типа записи такого поля больше нет. Значений секрета в `RecordView` нет.
  - `pub struct RecordSummary { pub id, pub title, pub type_id, pub folder, pub tags, pub secret_kind, pub updated_at }`
  - `pub struct ListFilter { pub type_id: Option<TypeId>, pub folder: Option<FolderId>, pub tag: Option<String>, pub secrets: Option<bool>, pub trashed: bool }` (`Default`: всё `None`, `trashed: false`).
  - Методы `Vault` (каждый изменяющий метод сразу сохраняет `objects/<id>.bin`):
    `create_record(NewRecord) -> Result<RecordId>`, `get(&RecordId) -> Result<RecordView>`, `list(&ListFilter) -> Vec<RecordSummary>` (по `updated_at` desc),
    `set_title`, `set_body(&RecordId, &str)`, `set_type(&RecordId, Option<TypeId>)`, `set_folder`, `add_tag`, `remove_tag`,
    `set_field(&RecordId, &FieldId, Option<FieldValue>) -> Result<()>` — поле должно быть в текущем типе записи и `fits`, иначе `Validation`,
    `load_errors() -> &[CryptError]` — файлы, которые не удалось расшифровать или разобрать при открытии (`Corrupt { file }`).
  - В Task 4 дописать: `delete_folder` → `Validation`, если в папке есть записи; `delete_type` → `Validation`, если тип используется хоть одной записью (включая корзину).

- [ ] **Step 1: Падающие тесты**

```rust
#[test] fn record_roundtrip_across_reopen() { /* create «Яндекс 360» с типом «Сервис», set_body("Семейная подписка [[...]]"), add_tag("#Подписки"), set_field(Стоимость, Number(399.0)) → reopen → get() то же самое, tags == ["подписки"] */ }
#[test] fn set_field_validates_kind_and_membership() { /* Text в поле Number → Validation; FieldId не из типа → Validation; запись без типа → Validation */ }
#[test] fn removed_field_value_becomes_hidden() { /* значение есть → put_type без этого поля → get(): нет в fields, есть в hidden_fields; вернуть поле → снова в fields */ }
#[test] fn changed_field_kind_keeps_old_value_hidden() { /* Text → Number у того же FieldId: старое Text-значение уходит в hidden_fields, get() не падает */ }
#[test] fn list_filters_by_type_folder_tag() { /* 3 записи, проверка каждого фильтра */ }
#[test] fn corrupt_object_does_not_block_vault() { /* испортить один objects/*.bin → open Ok, другая запись читается, load_errors().len() == 1 */ }
#[test] fn object_files_are_encrypted() { /* в objects/*.bin нет байтов «Семейная подписка» */ }
#[test] fn delete_type_in_use_rejected() { /* Validation */ }
#[test] fn delete_non_empty_folder_rejected() { /* Validation */ }
```

- [ ] **Step 2: Run** `cargo test -p crypt-core record` → FAIL.
- [ ] **Step 3: Реализовать** `RecordDoc` и методы `Vault`. При открытии читаются все `objects/*.bin` (кроме `*.tmp`); у каждого `peer_id` устройства из `device.json`.
- [ ] **Step 4: Run** `cargo test -p crypt-core` → PASS.
- [ ] **Step 5: Commit** `feat(core): records with fields, tags, folders in per-record Loro docs`.

---

### Task 6: Секреты, корзина и полная вычистка

**Files:**
- Create: `src/model/secret.rs`; Modify: `src/model/record.rs`, `src/vault.rs`
- Test: юнит-тесты в `vault.rs`

**Interfaces:**
- Consumes: `RecordDoc`, `MetaDoc::add_tombstone`, `VaultPaths::object`
- Produces:
  - `impl SecretKind { pub fn value_names(&self) -> &'static [&'static str] }` (enum из Task 5):
    Login → `["username","password","url","totp_seed"]`; Card → `["number","holder","expiry","cvv","pin"]`; Token → `["token","url"]`; SshKey → `["private_key","public_key","passphrase","comment"]`; Note → `["note"]`.
  - `pub struct SecretRevision { pub value: Zeroizing<String>, pub replaced_at: i64 }`
  - Методы `Vault`:
    `secret_value(&RecordId, name: &str) -> Result<Option<Zeroizing<String>>>`,
    `set_secret_value(&RecordId, name: &str, value: Option<&str>) -> Result<()>` — имя должно быть из `value_names()` вида секрета, запись должна быть секретом, иначе `Validation`; прежнее непустое значение добавляется в `secret_history` с `replaced_at = now`,
    `secret_history(&RecordId, name: &str) -> Result<Vec<SecretRevision>>` (новые первыми),
    `set_agent_enabled(&RecordId, bool) -> Result<()>` (только `SshKey`), `agent_keys() -> Vec<(RecordId, String /*title*/)>` — ssh-ключи с включённым флагом (для плана 3),
    `delete(&RecordId) -> Result<()>` — секрет → `purge`; обычная запись → `deleted_at = now` (корзина),
    `restore(&RecordId) -> Result<()>`, `purge(&RecordId) -> Result<()>` — удалить файл, убрать из памяти и индекса, `add_tombstone`, сохранить `meta`,
    `empty_trash() -> Result<usize>`.
  - Ссылки `Links` на вычищенный id остаются в данных других записей, но `get()` отфильтровывает несуществующие id из `fields` (моё решение: без каскадной перезаписи чужих документов).

- [ ] **Step 1: Падающие тесты**

```rust
#[test] fn secret_values_roundtrip_and_history() { /* Login: set password "old1", затем "new2" → secret_value == "new2"; history == ["old1"] */ }
#[test] fn secret_value_name_validated() { /* "cvv" у Login → Validation; set_secret_value у обычной записи → Validation */ }
#[test] fn record_view_has_no_secret_values() { /* serde_json::to_string(&get(id)) не содержит "new2" */ }
#[test] fn delete_secret_purges_file_history_and_adds_tombstone() {
    /* delete(secret) → objects/<id>.bin не существует; get → NotFound; list(secrets: Some(true)) пуст;
       reopen → всё ещё NotFound; meta tombstones содержит id; ни один файл папки не содержит "old1"/"new2" */
}
#[test] fn delete_plain_record_goes_to_trash_and_restores() { /* delete → list() без него, list(trashed:true) с ним; restore → снова в list() */ }
#[test] fn empty_trash_purges() { /* 2 в корзине → empty_trash() == 2, файлов нет */ }
#[test] fn links_to_purged_secret_are_dropped_from_view() { /* сервер с SecretLink → delete(secret) → get(server).fields[Креды] == Links([]) */ }
#[test] fn agent_flag_only_for_ssh_keys() { /* Login → Validation; SshKey on → agent_keys() содержит его */ }
```

- [ ] **Step 2: Run** `cargo test -p crypt-core secret trash` → FAIL.
- [ ] **Step 3: Реализовать.**
- [ ] **Step 4: Run** `cargo test -p crypt-core` → PASS.
- [ ] **Step 5: Commit** `feat(core): secrets with value history, trash, purge with tombstones`.

---

### Task 7: Поиск, wiki-ссылки, бэклинки

**Files:**
- Create: `src/index/{mod,links}.rs`; Modify: `src/vault.rs`
- Test: юнит-тесты в `links.rs`, `index/mod.rs`

**Interfaces:**
- Consumes: `RecordDoc`, `RecordId`, `FieldValue::Links`
- Produces:
  - Формат ссылки в теле: `[[<RecordId>]]` или `[[<RecordId>|<алиас>]]`. Редактор (план 2) вставляет id, показывает алиас/текущий заголовок.
  - `pub struct WikiLink { pub target: RecordId, pub alias: Option<String> }`, `pub fn parse_wikilinks(body: &str) -> Vec<WikiLink>` — некорректный id внутри `[[...]]` игнорируется; дубликаты сохраняются по порядку.
  - `pub struct SearchHit { pub id: RecordId, pub title: String, pub snippet: String, pub in_title: bool }`
  - `pub struct Index` — строится из `title`, `body`, тегов и ссылок (`body` + все `FieldValue::Links`); **никогда** не читает LoroMap `secret`/`secret_history`. Методы: `upsert(&RecordDoc)`, `remove(&RecordId)`, `search(&str, limit: usize) -> Vec<SearchHit>`, `backlinks(&RecordId) -> Vec<RecordId>`.
  - Поиск: регистронезависимый для кириллицы и латиницы (`to_lowercase`), подстрока; попадания в заголовок раньше, потом по `updated_at` desc; записи в корзине не ищутся; `snippet` — до 80 символов вокруг первого вхождения в `body` (по границам символов, не байтов).
  - Методы `Vault`: `search(&str, limit) -> Vec<SearchHit>`, `backlinks(&RecordId) -> Vec<RecordSummary>`. Каждое изменение записи обновляет индекс.

- [ ] **Step 1: Падающие тесты**

```rust
#[test] fn parses_links_with_and_without_alias() { /* "[[01J…]] и [[01J…|Аня]] и [[не-id]]" → 2 ссылки, alias второй == Some("Аня") */ }
#[test] fn search_is_case_insensitive_cyrillic() { /* «Яндекс 360» находится по "яндекс" и "ЯНДЕКС" */ }
#[test] fn title_hits_rank_first() { /* «Бюджет» в заголовке одной записи и в теле другой → первая с in_title */ }
#[test] fn snippet_respects_char_boundaries() { /* длинное кириллическое тело → snippet валидный UTF-8, ≤ 80 символов */ }
#[test] fn search_never_matches_secret_values() { /* пароль "v7#Qm!r2Lx9" → search("v7#Qm") пуст */ }
#[test] fn backlinks_from_body_and_fields() { /* A ссылается в теле на C, B через SecretLink на C → backlinks(C) == {A, B}; убрать ссылку из A → {B} */ }
#[test] fn trashed_and_purged_excluded() { /* в корзине — не в поиске и не в бэклинках; restore — снова есть */ }
```

- [ ] **Step 2: Run** `cargo test -p crypt-core index links` → FAIL.
- [ ] **Step 3: Реализовать** (`regex`: `\[\[([0-9A-HJKMNP-TV-Z]{26})(?:\|([^\]\n]*))?\]\]`).
- [ ] **Step 4: Run** `cargo test -p crypt-core` → PASS.
- [ ] **Step 5: Commit** `feat(core): in-memory search, wikilinks and backlinks`.

---

### Task 8: Сквозной тест и проверка, что ядро собирается в WASM

**Files:**
- Create: `crates/crypt-core/tests/vault_e2e.rs`; Modify: `crates/crypt-core/src/lib.rs` (публичные реэкспорты)

**Interfaces:**
- Produces: публичный API крейта для плана 2 = `Vault` + типы из Task 1–7, реэкспортированные из `lib.rs`: `Vault, CryptError, Result, SeedPhrase, SecretKey, VaultId, RecordId, TypeId, FieldId, FolderId, OptionId, ViewId, TypeDef, FieldDef, FieldKind, FieldValue, SelectOption, Folder, ViewDef, NewRecord, RecordView, RecordSummary, ListFilter, SecretKind, SecretRevision, SearchHit, starter_types`.

- [ ] **Step 1: Написать сквозной тест `full_user_journey`**

Сценарий: создать хранилище → тип «Сервис» (Стоимость: Number, Продление: Date, Доступ: SecretLink) → секрет Login «Яндекс ID» с паролем → запись «Яндекс 360» с полями и `Доступ = Links([секрет])`, тело со ссылкой на заметку «Бюджет 2026» → `lock()` → `open_with_key(data_key)` → проверить: поля, `backlinks(секрет)` содержит «Яндекс 360», `search("бюджет")` находит заметку, `secret_value` == пароль → сменить пароль → история из 1 элемента → удалить секрет → поле «Доступ» пустое, секрета нет ни в поиске, ни на диске.

- [ ] **Step 2: Run** `cargo test -p crypt-core --test vault_e2e` → PASS (если упало — чинить в соответствующем модуле, не в тесте).
- [ ] **Step 3: Проверить сборку под WASM**

Run: `rustup target add wasm32-unknown-unknown && cargo check -p crypt-core --target wasm32-unknown-unknown --no-default-features --features wasm`
Expected: проходит. Чтобы это работало: модуль `storage::fs` и методы `Vault`, трогающие диск, — за feature `fs` (включён по умолчанию); feature `wasm` включает `getrandom/js`. Если `loro` или другая зависимость не собирается под WASM — не чинить, а записать в `docs/notes/wasm.md`, что именно мешает (требование мягкое, спец §10).
- [ ] **Step 4: Run** `cargo test -p crypt-core && cargo clippy -p crypt-core -- -D warnings` → PASS, без предупреждений.
- [ ] **Step 5: Commit** `test(core): end-to-end vault journey; wasm check`.
