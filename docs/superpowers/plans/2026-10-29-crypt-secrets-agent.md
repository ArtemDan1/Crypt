# Crypt: секреты и ssh-agent (план 3 из 3) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans (решение пользователя: без субагентов). Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** закрыть MVP. Нужны экраны секретов (логины, SSH-ключи, токены, карты, защищённые заметки) с показом и копированием значений, очистка буфера, генератор паролей, TOTP, генерация SSH-ключей, импорт из `~/.ssh`, встроенный ssh-agent с Touch ID и жизнь приложения в меню-баре.

**Architecture:** чистая логика без ОС (генератор, TOTP, разбор, генерация и подпись SSH-ключей) живёт в `crypt-core` и собирается в WASM. Платформенные части живут в `crypt-desktop` за трейтами, по образцу `KeyStore` и `Authenticator`: буфер обмена (`Clipboard`), часы для окна доверия агента (`Clock`), путь к `~/.ssh`. Агент — это синхронный сервер на `std::os::unix::net::UnixListener`, по потоку на соединение. Протокол разбирается чистыми функциями. Пока хранилище заблокировано, список публичных ключей берётся из кэша в Keychain. Фронтенд получает значение секрета только по явному «показать», а копирование целиком выполняется в Rust.

**Статус реализации:** код добавлен, Rust 102 теста (кроме UnixListener в песочнице), frontend 40 тестов, clippy/WASM/сборки прошли. Полная приёмка сокета и ручные проверки macOS отмечены отдельно в `2026-10-29-crypt-secrets-agent-execution.md`.

**Tech Stack:** Rust: `ssh-key` (0.6.x или новее; фичи `ed25519`, `rsa`, `p256`, `p384`, `encryption`, `getrandom`/`rand_core`), `rsa` + `sha2` (подписи `rsa-sha2-256/512`), `hmac` + `sha1` (TOTP), `data-encoding` (base32), `libc` (`getpeereid`, `LOCAL_PEERPID`, `proc_pidpath`), `objc2-app-kit` с фичей `NSPasteboard`, `tauri` с фичами `tray-icon` и `image-png`, `tauri-plugin-autostart`, `tauri-plugin-single-instance`. Фронтенд: существующие Vue 3, shadcn-vue, Pinia, Vitest.

**Spec:** `docs/specs/2026-10-28-crypt-design.md` (§4.3 Секреты, §6 ssh-agent, §8 Безопасность). **Макеты:** https://claude.ai/artifact/6WNytj3nPs5x1k1EcEEg8U (экраны «Секреты» и «Меню-бар»). **Прошлые планы:** `2026-10-28-crypt-core.md`, `2026-10-28-crypt-desktop.md`, `2026-10-29-navigation-settings.md`.

**Что уже есть в ядре** (не переделывать): `SecretKind { Login, Card, Token, SshKey, Note }` и `value_names()`; `Vault::{secret_value, set_secret_value, secret_history, set_agent_enabled, agent_keys, delete}`. Удаление секрета означает полную вычистку с надгробием, смена значения пишет старое в историю. `LinkPicker` уже умеет выбирать секрет для поля `SecretLink`.

## Global Constraints

- **Git не трогать** (указание пользователя): никаких `git init` и коммитов, шаги «Commit» опущены. Если пользователь до старта разрешит git, сделать первый коммит до Task 1 и коммитить после каждой задачи.
- **Без субагентов** (указание пользователя): исполнение и финальное ревью делаются в этой же сессии.
- Без платного Apple Developer аккаунта: никаких entitlements и data-protection keychain. Touch ID проверяется через существующий `Authenticator` (`LAPolicy::DeviceOwnerAuthentication`, с откатом на пароль Mac).
- Keychain: service `dev.crypt.app`. Новый account `agent-keys:<vault_id>` хранит JSON `Vec<AgentIdentity>` с публичными ключами для работы агента при заблокированном хранилище, без биометрии. Приватных данных в нём нет.
- Сокет агента: `<app_data_dir>/agent.sock` (`~/Library/Application Support/dev.crypt.app/agent.sock`). Права: каталог `0700`, сокет `0600`. Соединения от процессов с другим uid (`getpeereid`) закрываются сразу.
- Значения секретов не попадают в логи, `message` ошибок, `localStorage`, URL и Pinia-сторы. В DOM они появляются только после «показать» и автоматически скрываются через **30 с**.
- Буфер обмена: копирование делает Rust. Через **30 с** (настройка `clipboardClearSeconds`: 15/30/60/90) буфер очищается, **только если** `NSPasteboard.changeCount` с момента копирования не изменился. Вместе со строкой пишется тип `org.nspasteboard.ConcealedType`, чтобы менеджеры буфера не сохраняли значение в историю.
- «Публичные» значения, видимые без «показать»: `login.username`, `login.url`, `token.url`, `sshkey.public_key`, `sshkey.comment`, `card.holder`, `card.expiry`. Остальные скрыты: `password`, `totp_seed`, `token`, `private_key`, `passphrase`, `number`, `cvv`, `pin`, `note`.
- Агент подтверждает **каждую** подпись через Touch ID. Окно доверия «доверять N минут» (`agentTrustMinutes`: 0/1/5/15, по умолчанию **0** = выключено) действует на конкретный ключ. Блокировка хранилища сбрасывает все окна доверия.
- Одновременно показывается не больше одного диалога Touch ID от агента: подписи сериализуются мьютексом, и после разблокировки мьютекса окно доверия проверяется заново.
- Генератор паролей: длина 8–128, по умолчанию **24**. Классы: строчные, заглавные, цифры, символы `!#$%&*+-=?@^_~`. Все четыре класса включены по умолчанию, каждый выбранный класс встречается хотя бы раз. Опция «без похожих» исключает `0O1lI`. Источник случайности — `OsRng`.
- TOTP: RFC 6238. По умолчанию SHA1, 6 цифр, 30 с. Принимается base32 (пробелы и регистр игнорируются, паддинг необязателен) или `otpauth://totp/...?secret=&algorithm=&digits=&period=`. Значение `totp_seed` наружу не отдаётся, отдаётся только текущий код.
- SSH-ключи: поддерживаются ed25519, ecdsa (nistp256, nistp384) и rsa. Новый ключ по умолчанию ed25519. Импортированный ключ с паролем расшифровывается при импорте и хранится как незашифрованный OpenSSH-ключ внутри хранилища (оно само зашифровано). Пароль не сохраняется. Фингерпринт — `SHA256:<base64 без паддинга>`, как в `ssh-keygen -l`.
- Файлы в `~/.ssh` приложение никогда не меняет и не удаляет, кроме `~/.ssh/config`, и только по явной кнопке (Task 7).
- Меню-бар: закрытие окна (красная кнопка / ⌘W) прячет окно и убирает иконку из Dock (`ActivationPolicy::Accessory`), агент продолжает работать. ⌘Q и «Выйти» в меню-баре завершают приложение. Второй запуск фокусирует уже запущенный экземпляр.
- Язык интерфейса русский. Акцент `#fbbf24` (`text-secret`) — только на элементах секретов.

## Review Focus

1. **Пользователь скопировал что-то своё до истечения 30 с:** таймер очистки не должен стереть новое содержимое буфера. → тест `clear_skips_when_clipboard_changed` в Task 3.
2. **Два `ssh` одновременно** (`git fetch` в двух репозиториях, параллельный `ansible`): один диалог Touch ID за раз, второй запрос ждёт. При включённом окне доверия второй проходит без диалога, при выключенном — со своим диалогом. Ни один не падает. → тест `concurrent_signs_prompt_one_at_a_time` в Task 6.
3. **Хранилище заблокировано, запускаем `ssh`:** список ключей приходит из кэша Keychain. Подпись вызывает Touch ID, временно открывает хранилище, подписывает и снова закрывает (UI остаётся заблокированным). Если Touch ID для входа не настроен или отменён, ответ `SSH_AGENT_FAILURE` приходит сразу, без зависания, а окно Crypt выходит вперёд на экране разблокировки. → тесты `locked_sign_uses_biometric_and_relocks` и `locked_sign_without_biometric_fails_fast` в Task 6.
4. **Мусор в `~/.ssh`:** `config`, `known_hosts`, `*.pub`, `authorized_keys`, PuTTY `.ppk`, битые файлы, файлы больше 64 КБ, подкаталоги, симлинки на несуществующее. Скан молча пропускает всё, что не является приватным ключом OpenSSH/PEM. Неверный пароль у одного ключа не прерывает импорт остальных. Повторный импорт того же ключа помечается как «уже есть». → тест `scan_skips_junk_and_import_is_per_item` в Task 4.
5. **Повторная настройка `~/.ssh/config` и чужое содержимое:** второй запуск не дублирует блок Crypt, остальной текст пользователя сохраняется байт в байт, резервная копия создаётся один раз, права `0600` сохраняются, а «Убрать» возвращает файл к исходному тексту. → тест `ssh_config_block_is_idempotent_and_reversible` в Task 7.

---

## Структура файлов

```
crates/crypt-core/src/
  gen.rs                       # генератор паролей                         (Task 1)
  totp.rs                      # RFC 6238, разбор otpauth://               (Task 1)
  ssh.rs                       # разбор/генерация/фингерпринт/подпись      (Task 2)
  vault.rs                     # + set_ssh_private_key, agent_identities, totp_code (Task 2)
crates/crypt-core/tests/{gen_totp.rs, ssh.rs}
apps/desktop/src-tauri/src/
  clipboard.rs                 # трейт Clipboard, MacClipboard, MemoryClipboard, таймер очистки (Task 3)
  commands/secrets.rs          # команды значений, истории, генератора, TOTP (Task 3)
  commands/ssh_import.rs       # скан и импорт ~/.ssh                       (Task 4)
  agent/protocol.rs            # разбор/кодирование сообщений агента        (Task 5)
  agent/policy.rs              # подтверждение, окно доверия, режим «заблокировано» (Task 6)
  agent/server.rs              # UnixListener, проверка uid, жизненный цикл (Task 6)
  agent/cache.rs               # кэш публичных ключей в Keychain            (Task 6)
  ssh_config.rs                # блок IdentityAgent в ~/.ssh/config         (Task 7)
  tray.rs                      # меню-бар, скрытие окна, Dock, автозапуск  (Task 8)
  state.rs                     # + новые поля Settings, clipboard, agent handle
apps/desktop/src/
  lib/secretFields.ts          # метаданные полей по виду секрета           (Task 9)
  lib/types.ts, lib/api.ts, dev/mockBackend.ts                               (Task 9)
  views/Secrets.vue            # вместо SecretsSoon.vue                     (Task 9)
  components/secret/{SecretPanel,SecretValue,PasswordGenerator,TotpCode,SecretHistory,SshKeySection,SecretChip}.vue (Task 10)
  components/secret/SshImportDialog.vue                                      (Task 11)
  views/Settings.vue           # + раздел «Секреты и ssh-agent»            (Task 11)
```

---

### Task 1: Генератор паролей и TOTP (ядро)

**Files:** Create `crates/crypt-core/src/gen.rs`, `crates/crypt-core/src/totp.rs`, `crates/crypt-core/tests/gen_totp.rs`. Modify `crates/crypt-core/src/lib.rs` (`pub mod gen; pub mod totp;`), `crates/crypt-core/Cargo.toml` (`hmac`, `sha1`, `data-encoding`).

**Interfaces:**
- Produces:
  - `#[derive(Deserialize, Clone)] #[serde(rename_all = "camelCase", default)] pub struct PasswordOptions { pub length: usize, pub lower: bool, pub upper: bool, pub digits: bool, pub symbols: bool, pub avoid_ambiguous: bool }`, где `Default` = 24, все классы `true`, `avoid_ambiguous: false`.
  - `pub fn generate_password(o: &PasswordOptions) -> Result<Zeroizing<String>>`. Если длина вне 8..=128, ни один класс не выбран или длина меньше числа классов, возвращается `CryptError::Validation`.
  - `pub struct TotpParams { pub secret: Zeroizing<Vec<u8>>, pub algorithm: TotpAlg /* Sha1|Sha256|Sha512 */, pub digits: u32, pub period: u64 }`
  - `pub fn parse_totp(input: &str) -> Result<TotpParams>` принимает base32 или `otpauth://totp/`. Пустая строка, плохой base32, `otpauth://hotp`, digits вне 6..=8 или period = 0 дают `Validation`.
  - `#[derive(Serialize)] pub struct TotpCode { pub code: String, pub remaining: u64, pub period: u64 }`
  - `pub fn totp_at(p: &TotpParams, unix_secs: u64) -> TotpCode`

- [x] **Step 1: Падающие тесты** в `tests/gen_totp.rs`:

```rust
#[test] fn password_respects_length_and_classes() {
    // 200 прогонов по умолчанию: len == 24, в каждом есть строчная, заглавная, цифра и символ из "!#$%&*+-=?@^_~"
    // {length: 8, upper/digits/symbols: false} -> только a-z
    // avoid_ambiguous: true, 200 прогонов -> нет ни одного из "0O1lI"
}
#[test] fn password_rejects_bad_options() {
    // length 7 и 129 -> Validation; все классы false -> Validation
}
#[test] fn totp_rfc6238_vectors() {
    // секрет ASCII "12345678901234567890" (SHA1), digits 8, period 30:
    // t=59 -> "94287082", t=1111111109 -> "07081804", t=20000000000 -> "65353130"
    // SHA256, секрет "12345678901234567890123456789012", t=59 -> "46119246"
    // SHA512, секрет "1234567890123456789012345678901234567890123456789012345678901234", t=59 -> "90693936"
    // remaining при t=59, period 30 -> 1
}
#[test] fn totp_parses_base32_and_otpauth() {
    // "gezd gnbv gy3t qojq" == "GEZDGNBVGY3TQOJQ" (пробелы и регистр)
    // "otpauth://totp/X:me?secret=GEZDGNBVGY3TQOJQ&digits=8&period=60&algorithm=SHA256" -> digits 8, period 60, Sha256
    // "", "!!!", "otpauth://hotp/x?secret=GEZD", digits=9 -> Validation
}
```

- [x] **Step 2: Run** `cargo test -p crypt-core --test gen_totp`. Ожидается FAIL (модулей нет).
- [x] **Step 3: Реализовать.** Пароль: сначала по одному символу из каждого выбранного класса, остальное из объединённого алфавита, затем перемешать через `SliceRandom::shuffle` с `OsRng`. TOTP: HMAC по `counter.to_be_bytes()`, динамическое усечение RFC 4226, `% 10^digits` с ведущими нулями. URI разбирается вручную (`split_once('?')`, параметры через `&`, percent-decode только `%xx`), без новых зависимостей.
- [x] **Step 4: Run** `cargo test -p crypt-core` → PASS и `cargo check -p crypt-core --target wasm32-unknown-unknown --no-default-features --features wasm` → собирается.

---

### Task 2: SSH-ключи в ядре

**Files:** Create `crates/crypt-core/src/ssh.rs`, `crates/crypt-core/tests/ssh.rs`. Modify `lib.rs`, `vault.rs`, `Cargo.toml` (`ssh-key`, `rsa`, `sha2` уже есть).

**Interfaces:**
- Consumes: `Vault::set_secret_value`, `Vault::secret_value`, `RecordDoc::agent_enabled`, `parse_totp` / `totp_at` (Task 1).
- Produces:
  - `#[derive(Serialize, Clone, Debug, PartialEq)] pub struct SshKeyInfo { pub algorithm: String /* "ssh-ed25519" | "ecdsa-sha2-nistp256" | ... | "ssh-rsa" */, pub bits: Option<u32>, pub public_key: String /* строка authorized_keys без комментария */, pub fingerprint: String, pub comment: String }`
  - `pub fn inspect_private_key(pem: &str, passphrase: Option<&str>) -> Result<(SshKeyInfo, Zeroizing<String>)>` возвращает информацию и **расшифрованный** OpenSSH PEM. Ошибки: `CryptError::Validation("not a private key")`. Для зашифрованного ключа без пароля — новый вариант `CryptError::PassphraseRequired`, при неверном пароле — `CryptError::WrongPassphrase`. Добавить оба варианта в `error.rs`, а их коды в desktop `ApiError` (сообщения «Ключ защищён паролем» / «Неверный пароль ключа»).
  - `pub fn is_encrypted_private_key(pem: &str) -> Option<bool>` (`None`, если это не приватный ключ). Нужен для скана без пароля.
  - `pub fn generate_ed25519(comment: &str) -> Result<(SshKeyInfo, Zeroizing<String>)>`
  - `pub fn public_key_blob(public_key: &str) -> Result<Vec<u8>>` (wire-формат для агента).
  - `pub fn sign(pem: &str, data: &[u8], flags: u32) -> Result<Vec<u8>>` возвращает SSH-подпись в wire-формате (`string algo, string sig`). Для RSA: `flags & 2` → `rsa-sha2-256`, `flags & 4` → `rsa-sha2-512`, иначе `ssh-rsa` (SHA-1, как у OpenSSH-агента). ecdsa/ed25519 игнорируют флаги.
  - `Vault::set_ssh_private_key(&mut self, id: &RecordId, pem: &str, passphrase: Option<&str>) -> Result<SshKeyInfo>` вызывает `inspect_private_key` и пишет `private_key` (расшифрованный), `public_key`, `comment` (если пуст — комментарий ключа). Запись должна быть `SecretKind::SshKey`, иначе `Validation`.
  - `Vault::ssh_key_info(&self, id: &RecordId) -> Result<Option<SshKeyInfo>>` (`None`, если ключа ещё нет).
  - `#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)] pub struct AgentIdentity { pub id: RecordId, pub title: String, pub public_key: String, pub fingerprint: String }`
  - `Vault::agent_identities(&self) -> Vec<AgentIdentity>`: SSH-секреты не в корзине, с `agent_enabled` и непустым `public_key`. Заменяет `agent_keys` (старый метод удалить, если больше нигде не используется).
  - `Vault::sign_with(&self, id: &RecordId, data: &[u8], flags: u32) -> Result<Vec<u8>>`
  - `Vault::totp_code(&self, id: &RecordId, unix_secs: u64) -> Result<Option<TotpCode>>` (`None`, если `totp_seed` пуст).

- [x] **Step 1: Падающие тесты** в `tests/ssh.rs`. Тестовые ключи сгенерировать один раз `ssh-keygen` и положить в `crates/crypt-core/tests/fixtures/ssh/`: `ed25519`, `ed25519_enc` (пароль `correct horse`), `ecdsa256`, `rsa3072`, плюс `.pub` для каждого и вывод `ssh-keygen -lf` в `fingerprints.txt`.

```rust
#[test] fn inspect_matches_ssh_keygen() {
    // для каждого fixture: info.public_key == первые два поля .pub; info.fingerprint == значение из fingerprints.txt
}
#[test] fn encrypted_key_needs_correct_passphrase() {
    // без пароля -> PassphraseRequired; "wrong" -> WrongPassphrase; "correct horse" -> Ok, а возвращённый PEM не зашифрован
    // is_encrypted_private_key: ed25519_enc -> Some(true), ed25519 -> Some(false), содержимое .pub -> None
}
#[test] fn junk_is_rejected() { /* "", "hello", содержимое known_hosts, PuTTY-заголовок "PuTTY-User-Key-File-3" -> Validation */ }
#[test] fn sign_verifies_with_public_key() {
    // generate_ed25519("t"), ecdsa256 и rsa3072: wire-подпись данных b"challenge" декодируется в ssh_key::Signature и проверяется публичным ключом фикстуры (signature::Verifier)
    // rsa: flags 2 -> имя алгоритма в подписи "rsa-sha2-256", flags 4 -> "rsa-sha2-512"
}
#[test] fn vault_ssh_roundtrip_and_agent_identities() {
    // создать SshKey-запись, set_ssh_private_key(ed25519) -> public_key и fingerprint сохранены
    // agent_identities() содержит запись; set_agent_enabled(false) -> не содержит; delete -> не содержит
    // set_ssh_private_key на Login-записи -> Validation
    // sign_with проверяется публичным ключом
}
#[test] fn vault_totp_code() { /* Login с totp_seed "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ" -> код из 6 цифр; без seed -> None */ }
```

- [x] **Step 2: Run** `cargo test -p crypt-core --test ssh`. Ожидается FAIL.
- [x] **Step 3: Реализовать.** Разбор и шифрование через `ssh_key::PrivateKey::{from_openssh, decrypt, to_openssh}`. Если `from_openssh` не справился, пробовать PEM PKCS#1/PKCS#8 (`-----BEGIN RSA PRIVATE KEY-----`, `-----BEGIN PRIVATE KEY-----`) через `rsa` / `p256` / `p384` и конвертировать в OpenSSH. Зашифрованный старый PEM (`Proc-Type: 4,ENCRYPTED`) отклонять с `Validation("старый формат с паролем: выполните ssh-keygen -p -f <file>")`. RSA-подпись: если `Signer` из `ssh-key` не даёт выбрать хэш, подписывать через `rsa::pkcs1v15::SigningKey::<Sha256 | Sha512 | Sha1>` из `RsaKeypair`. Комментарий из `generate_ed25519` — переданная строка.
- [x] **Step 4: Run** `cargo test -p crypt-core` → PASS; `cargo clippy -p crypt-core --all-targets -- -D warnings` → чисто; WASM-проверка из Task 1 собирается (если `rsa`/`ssh-key` требуют фичу getrandom под wasm — добавить её в feature `wasm`).

---

### Task 3: Команды значений секретов и буфер обмена

**Files:** Create `apps/desktop/src-tauri/src/clipboard.rs`, `apps/desktop/src-tauri/src/commands/secrets.rs`. Modify `state.rs` (поля `Settings`, `clipboard: Box<dyn Clipboard>`), `commands/mod.rs`, `lib.rs` (регистрация), `error.rs` (новые коды), `Cargo.toml` (`objc2-app-kit` + фича `NSPasteboard`).

**Interfaces:**
- Consumes: `with_vault`, `crypt_core::{gen, totp}`, `Vault::{secret_value, set_secret_value, secret_history, ssh_key_info, totp_code, set_ssh_private_key}`, `generate_ed25519`.
- Produces:
  - `Settings` получает поля (serde `default`, старые файлы читаются): `clipboard_clear_seconds: u32 = 30`, `agent_enabled: bool = true`, `agent_trust_minutes: u32 = 0`, `launch_at_login: bool = false`. Значения вне `{15,30,60,90}` и `{0,1,5,15}` дают `Validation` в `set_settings`.
  - `pub trait Clipboard: Send + Sync { fn write_concealed(&self, text: &str) -> u64 /* changeCount после записи */; fn change_count(&self) -> u64; fn clear(&self); }`, реализации `MacClipboard` (`NSPasteboard::generalPasteboard`, `clearContents`, `setString:forType:` для `NSPasteboardTypeString` и `org.nspasteboard.ConcealedType`) и `MemoryClipboard` (тесты).
  - `pub fn copy_with_clear(cb: Arc<dyn Clipboard>, text: &str, after: Duration)`: пишет значение, запускает поток `sleep(after)`, затем `if cb.change_count() == token { cb.clear() }`.
  - `#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct SecretOverview { pub kind: SecretKind, pub public: BTreeMap<String, String> /* только «публичные» имена из Global Constraints */, pub filled: Vec<String> /* какие скрытые значения заданы */, pub history_counts: BTreeMap<String, usize>, pub ssh: Option<SshKeyInfo>, pub agent_enabled: bool, pub has_totp: bool }`
  - Команды (все async, через `with_vault`):
    `secret_overview(id) -> SecretOverview`,
    `reveal_secret(id, name) -> String`,
    `copy_secret(id, name) -> ()`,
    `set_secret(id, name, value: Option<String>) -> ()`,
    `secret_history(id, name) -> Vec<{ index: usize, replacedAt: i64 }>`,
    `reveal_secret_revision(id, name, index) -> String`, `copy_secret_revision(id, name, index) -> ()`,
    `generate_password(options: PasswordOptions) -> String`,
    `totp_now(id) -> Option<TotpCode>`, `copy_totp(id) -> ()`,
    `set_ssh_key(id, pem, passphrase: Option<String>) -> SshKeyInfo`, `generate_ssh_key(id) -> SshKeyInfo` (комментарий `<title>@crypt`),
    `set_agent_enabled(id, enabled) -> ()`.
    Для `set_secret` с именем `private_key` возвращается `Validation` («используйте set_ssh_key»), чтобы `public_key` и фингерпринт не расходились с ключом. `set_secret` с `public_key` для SshKey тоже запрещён.

- [x] **Step 1: Падающие тесты** (юнит-тесты в `commands/secrets.rs` и `clipboard.rs`, `MemoryKeyStore` + `FakeAuthenticator` + `MemoryClipboard`):

```rust
#[test] fn overview_hides_sensitive_values() {
    // Login: username "anton", password "p@ss", url "https://x", totp_seed "GEZDGNBVGY3TQOJQ"
    // serde_json::to_string(&secret_overview) содержит "anton" и "https://x", НЕ содержит "p@ss" и "GEZD"
    // filled == ["password", "totp_seed"], has_totp == true
}
#[test] fn copy_writes_and_clears_after_timeout() { /* copy_with_clear(…, 50ms) -> буфер == значение; через 100 мс пусто */ }
#[test] fn clear_skips_when_clipboard_changed() { /* copy, затем пользователь пишет "mine" (change_count+1); через 100 мс буфер == "mine" */ }
#[test] fn private_key_only_via_set_ssh_key() { /* set_secret(…, "private_key") -> Validation; set_ssh_key(fixture) -> info.fingerprint совпадает */ }
#[test] fn locked_vault_rejects_secret_commands() { /* после lock: reveal/copy/overview -> code "Locked" */ }
#[test] fn settings_validate_new_fields() { /* старый settings.json {"autolockMinutes":5} читается, clipboardClearSeconds == 30; 20 -> Validation; agentTrustMinutes 7 -> Validation */ }
#[test] fn api_errors_never_contain_values() { /* reveal несуществующего имени: message не содержит заданное значение; WrongPassphrase-message не содержит пароль */ }
```

- [x] **Step 2: Run** `cargo test -p crypt-desktop secrets clipboard settings`. Ожидается FAIL.
- [x] **Step 3: Реализовать** модуль, команды и регистрацию в `lib.rs`. Копирование идёт через `copy_with_clear` с `Duration::from_secs(settings.clipboard_clear_seconds)`. `Clipboard` хранится в `AppState` как `Arc<dyn Clipboard>`.
- [x] **Step 4: Run** `cargo test -p crypt-desktop` → PASS; `cargo clippy --workspace --all-targets -- -D warnings` → чисто.

---

### Task 4: Скан и импорт `~/.ssh`

**Files:** Create `apps/desktop/src-tauri/src/commands/ssh_import.rs`. Modify `lib.rs`, `state.rs` (поле `ssh_dir: PathBuf`, по умолчанию `$HOME/.ssh`, в тестах — tempdir).

**Interfaces:**
- Consumes: `crypt_core::ssh::{inspect_private_key, is_encrypted_private_key}`, `Vault::{create_record, set_ssh_private_key, set_agent_enabled, agent_identities}`; кэш агента `agent::cache::refresh` (Task 6; до неё вызов — no-op-заглушка с тем же именем).
- Produces:
  - `#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct SshCandidate { pub path: String, pub file_name: String, pub encrypted: bool, pub info: Option<SshKeyInfo> /* None для зашифрованных: публичную часть берём из соседнего .pub, если есть */, pub already_imported: bool }`
  - `#[derive(Deserialize)] pub struct SshImportItem { pub path: String, pub title: String, pub passphrase: Option<String> }`
  - `#[derive(Serialize)] pub struct SshImportResult { pub path: String, pub record_id: Option<RecordId>, pub error: Option<ApiError> }`
  - Команды: `scan_ssh_dir() -> Vec<SshCandidate>` (только верхний уровень каталога; пропускаются подкаталоги, `*.pub`, файлы больше 64 КБ, нечитаемые файлы и всё, где `is_encrypted_private_key` даёт `None`; сортировка по имени), `import_ssh_keys(items) -> Vec<SshImportResult>`. Путь каждого item обязан лежать прямо в `ssh_dir` (после `canonicalize`), иначе `Validation`. Новые записи — `SecretKind::SshKey` в корне, `agent_enabled = true`. При ошибке ключа запись не создаётся (или удаляется при неудаче `set_ssh_private_key`).
  - `already_imported` = фингерпринт (из ключа или `.pub`) совпадает с одним из существующих SSH-секретов.

- [x] **Step 1: Падающий тест** `scan_skips_junk_and_import_is_per_item` (tempdir как `ssh_dir`): в каталог скопировать фикстуры `id_ed25519` (+ `.pub`), `id_enc` (зашифрованный, + `.pub`), `id_rsa`, плюс `config`, `known_hosts`, `authorized_keys`, `putty.ppk`, `broken` (обрезанный ключ), `big` (70 КБ), подкаталог `sub/` с ключом внутри, битый симлинк `dangling`.
  - скан возвращает ровно `id_ed25519`, `id_enc` (`encrypted: true`, `info` из `.pub`), `id_rsa`;
  - импорт `[id_ed25519, id_enc с паролем "wrong", id_rsa]` даёт ошибку `WrongPassphrase` только для `id_enc`, для двух других есть `record_id`, а секретов в хранилище ровно 2;
  - повторный скан: `already_imported` = true у `id_ed25519` и `id_rsa`;
  - содержимое и mtime всех файлов в каталоге не изменились;
  - item с путём `../etc/passwd` и путём `sub/key` → `Validation`.
- [x] **Step 2: Run** `cargo test -p crypt-desktop ssh_import`. Ожидается FAIL.
- [x] **Step 3: Реализовать.**
- [x] **Step 4: Run** `cargo test -p crypt-desktop` → PASS.

---

### Task 5: Протокол ssh-agent

**Files:** Create `apps/desktop/src-tauri/src/agent/mod.rs`, `apps/desktop/src-tauri/src/agent/protocol.rs`.

**Interfaces:**
- Produces (чистые функции, без I/O):
  - `pub enum Request { Identities /* 11 */, Sign { key_blob: Vec<u8>, data: Vec<u8>, flags: u32 } /* 13 */, Other(u8) }`
  - `pub enum Response { Identities(Vec<(Vec<u8> /*blob*/, String /*comment*/)>) /* 12 */, Signature(Vec<u8>) /* 14 */, Failure /* 5 */ }`
  - `pub fn decode_request(body: &[u8]) -> Result<Request, ProtocolError>` (body без 4-байтной длины).
  - `pub fn encode_response(r: &Response) -> Vec<u8>` (с 4-байтной длиной).
  - `pub fn read_frame(r: &mut impl Read) -> io::Result<Option<Vec<u8>>>`: `None` на EOF. Кадр длиннее **256 КБ** даёт ошибку, соединение закрывается.
  - Комментарий идентичности = заголовок записи (как увидит `ssh-add -l`).

- [x] **Step 1: Падающие тесты** в `protocol.rs`:

```rust
#[test] fn decodes_identities_and_sign() { /* [11] -> Identities; кадр 13 со string blob, string data, u32 flags=2 -> Sign{…, flags: 2} */ }
#[test] fn unknown_and_extension_messages_are_other() { /* [27 /*extension*/, …] -> Other(27); [17 /*add identity*/] -> Other(17) */ }
#[test] fn truncated_frames_are_errors() { /* Sign с обрезанной строкой -> Err; пустое тело -> Err */ }
#[test] fn encodes_responses() { /* Failure -> [0,0,0,1,5]; Identities с одним ключом -> [len][12][u32 1][string blob][string comment] */ }
#[test] fn oversized_frame_rejected() { /* длина 0x0010_0000 -> read_frame Err */ }
```

- [x] **Step 2: Run** `cargo test -p crypt-desktop agent::protocol`. Ожидается FAIL.
- [x] **Step 3: Реализовать** (big-endian `u32`, `string` = `u32 len + bytes`).
- [x] **Step 4: Run** → PASS.

---

### Task 6: Сервер агента, подтверждение и работа при заблокированном хранилище

**Files:** Create `agent/policy.rs`, `agent/server.rs`, `agent/cache.rs`. Modify `state.rs` (поле `agent: AgentHandle`), `commands/lifecycle.rs` (`lock`, `delete_vault`, все пути разблокировки), `commands/secrets.rs` и `commands/ssh_import.rs` (обновление кэша), `lib.rs` (запуск при старте, если `agent_enabled`), `Cargo.toml` (`libc`).

**Interfaces:**
- Consumes: `protocol::*` (Task 5), `Vault::{agent_identities, sign_with}`, `ssh::public_key_blob` (Task 2), `Authenticator`, `KeyStore`, `Unlocker::unlock_biometric`-логика (чтение `data-key:<id>` после `authenticate`).
- Produces:
  - `cache.rs`: `pub fn refresh(st: &AppState)`: если хранилище открыто, записывает `agent-keys:<vault_id>` = JSON `vault.agent_identities()`. Вызывается после каждой разблокировки и после команд `set_ssh_key`, `generate_ssh_key`, `set_agent_enabled`, `delete_record`/`purge_record` SSH-секрета, `set_title` SSH-секрета, `import_ssh_keys`. `pub fn load(keys: &dyn KeyStore, vault_id: &str) -> Vec<AgentIdentity>`; `delete_vault` удаляет элемент.
  - `pub trait Clock: Send + Sync { fn now(&self) -> Instant; }` (`SystemClock`, `FakeClock` для тестов).
  - `pub struct AgentPolicy { trust: Mutex<HashMap<RecordId, Instant>>, prompt: Mutex<()>, clock: Arc<dyn Clock> }` с методами:
    `pub fn handle(&self, st: &AppState, req: Request, peer: &PeerInfo) -> Response`,
    `pub fn clear_trust(&self)` (вызывается в `lock` и при автоблокировке).
  - Поведение `handle`:
    - `Identities`: если хранилище открыто, `vault.agent_identities()`, иначе `cache::load`. Ключи с невалидным `public_key` пропускаются.
    - `Sign`: найти идентичность по blob (нет → `Failure`). Если окно доверия для ключа ещё действует, подписать без диалога. Иначе взять `prompt`, перепроверить окно доверия и вызвать `auth.authenticate("ssh: подписать ключом «<title>» для <процесс>")`; при `Err` вернуть `Failure`. Если хранилище открыто, `vault.sign_with`. Если закрыто: нет `data-key:<id>` в Keychain → `Failure` и событие `agent://unlock-needed` (Task 8 показывает окно); иначе одно подтверждение покрывает и разблокировку, и подпись: `Vault::open_with_key` во временную переменную, подпись, `drop` (в `AppState.vault` не кладётся). При `agent_trust_minutes > 0` записать `trust[id] = now + N мин`.
    - `Other(_)`: `Failure`.
  - `pub struct PeerInfo { pub uid: u32, pub pid: Option<i32>, pub process: Option<String> }`; процесс берётся через `LOCAL_PEERPID` + `proc_pidpath` (basename), при ошибке `None` и в тексте «неизвестного процесса».
  - `server.rs`: `pub struct AgentHandle { … }` с `pub fn start(st: Arc<…>, socket: PathBuf) -> io::Result<()>`, `pub fn stop(&self)`, `pub fn running(&self) -> bool`. При старте: создать каталог `0700`. Если файл сокета существует, попробовать `UnixStream::connect`: успех → `AddrInUse` (второй экземпляр), отказ → удалить устаревший сокет. Затем `bind`, `chmod 0600`, поток `accept`. На каждое соединение отдельный поток: `getpeereid`, если uid не наш — закрыть; цикл `read_frame → decode → handle → write`. `stop` закрывает листенер и удаляет файл сокета.
  - Команды: `agent_status() -> { running: bool, socketPath: String, identities: usize }`. `set_settings` с изменённым `agent_enabled` запускает или останавливает агент.
  - Чтобы политика работала с `AppState` из потоков агента, `AppState` хранится в `Arc` (`app.manage(Arc<AppState>)` или `AppHandle::state` из потока — выбрать одно и применить везде).

- [x] **Step 1: Падающие тесты** (`policy.rs`, `MemoryKeyStore`, `FakeAuthenticator` со счётчиком вызовов и задержкой, `FakeClock`, фикстура ed25519):

```rust
#[test] fn identities_unlocked_and_from_cache_when_locked() { /* открыто -> 1 ключ; lock -> всё ещё 1 ключ (из кэша); agent_enabled=false + refresh -> 0 */ }
#[test] fn each_sign_prompts_without_trust() { /* 2 подписи -> authenticate вызван 2 раза; подписи проверяются публичным ключом */ }
#[test] fn trust_window_skips_prompt_until_expiry_and_lock() { /* trust 5 мин: 2 подписи -> 1 вызов; clock +6 мин -> снова вызов; clear_trust -> снова вызов */ }
#[test] fn cancelled_touch_id_fails() { /* FakeAuthenticator(Cancelled) -> Response::Failure */ }
#[test] fn unknown_key_fails_without_prompt() { /* blob чужого ключа -> Failure, authenticate не вызван */ }
#[test] fn concurrent_signs_prompt_one_at_a_time() {
    // authenticate спит 100 мс и отмечает максимум одновременных вызовов; 2 потока подписывают одновременно
    // trust=0 -> 2 вызова, максимум одновременных == 1; trust=5 -> 1 вызов, обе подписи успешны
}
#[test] fn locked_sign_uses_biometric_and_relocks() { /* Touch ID включён, lock; Sign -> подпись валидна, authenticate 1 раз, st.vault остаётся None */ }
#[test] fn locked_sign_without_biometric_fails_fast() { /* Touch ID не включён, lock; Sign -> Failure за < 50 мс, authenticate не вызван */ }
```

Плюс интеграционный тест `server_roundtrip_over_socket`: `AgentHandle::start` на `tempdir/agent.sock`, клиент `UnixStream` шлёт `Identities` и получает 1 ключ, затем `Sign` и получает валидную подпись. Права файла сокета `0o600`. Второй `start` на том же пути при живом первом даёт `AddrInUse`. После `stop` файла нет.

- [x] **Step 2: Run** `cargo test -p crypt-desktop agent`. Ожидается FAIL.
- [x] **Step 3: Реализовать.**
- [x] **Step 4: Run** `cargo test -p crypt-desktop` → PASS; clippy чистый.
- [ ] **Step 5: Ручная проверка с настоящим OpenSSH** (`pnpm tauri dev`, хранилище с импортированным ключом):
  `SSH_AUTH_SOCK="$HOME/Library/Application Support/dev.crypt.app/agent.sock" ssh-add -l` выводит фингерпринт и заголовок записи;
  `SSH_AUTH_SOCK=… ssh-keygen -Y sign -f <(ssh-add -L | head -1) -n test <<< hi` (или `ssh -T git@github.com`, если ключ там зарегистрирован) показывает Touch ID, подпись проходит. Если доступа к экрану нет, записать в журнал как ручную проверку для пользователя.

---

### Task 7: Настройка `~/.ssh/config`

**Files:** Create `apps/desktop/src-tauri/src/ssh_config.rs`. Modify `lib.rs` (команды).

**Interfaces:**
- Produces:
  - Блок (точный текст):
    ```
    # >>> Crypt ssh-agent >>>
    Host *
      IdentityAgent "~/Library/Application Support/dev.crypt.app/agent.sock"
    # <<< Crypt ssh-agent <<<
    ```
    Блок ставится **в конец** файла: в ssh_config побеждает первое значение, поэтому явные `IdentityAgent` пользователя в его `Host`-блоках сохраняют силу.
  - `pub fn with_block(original: &str) -> String`: удаляет существующий блок (между маркерами включительно) и добавляет новый в конец, отделив одной пустой строкой. Если файл не оканчивается `\n`, перевод строки добавляется.
  - `pub fn without_block(original: &str) -> String`: убирает блок и пустую строку, добавленную вместе с ним.
  - `pub fn has_block(original: &str) -> bool`.
  - Команды: `ssh_config_status() -> { configured: bool, path: String, block: String, exportLine: String /* export SSH_AUTH_SOCK="…" */ }`, `configure_ssh_config()`, `unconfigure_ssh_config()`. Запись атомарная (temp + rename) с правами `0600`, если файла не было. Существующие права сохраняются. Перед первой записью создаётся `~/.ssh/config.crypt-backup`, если его ещё нет. Каталог `~/.ssh` создаётся с `0700`, если его нет. Путь к каталогу берётся из `AppState.ssh_dir`.

- [x] **Step 1: Падающий тест** `ssh_config_block_is_idempotent_and_reversible`:
  - `orig = "Host gh\n  HostName github.com\n  IdentityAgent none\n"`; `a = with_block(orig)`; `with_block(&a) == a`; `a` содержит маркер ровно один раз; `a.starts_with(orig)`; `without_block(&a) == orig`;
  - файл без завершающего `\n`: `without_block(with_block("Host x"))` == `"Host x\n"` (допустимая нормализация, зафиксировать в тесте);
  - на tempdir: `configure` дважды → бэкап создан один раз и равен оригиналу, режим файла `0o600` (исходно выставить `0o600`), `unconfigure` → содержимое == оригинал;
  - пустой каталог: `configure` создаёт `config` с `0o600` и каталог с `0o700`.
- [x] **Step 2: Run** `cargo test -p crypt-desktop ssh_config`. Ожидается FAIL.
- [x] **Step 3: Реализовать.**
- [x] **Step 4: Run** → PASS.

---

### Task 8: Меню-бар, скрытие окна, автозапуск, один экземпляр

**Files:** Create `apps/desktop/src-tauri/src/tray.rs`. Modify `lib.rs`, `Cargo.toml` (`tauri` с фичами `tray-icon`, `image-png`; `tauri-plugin-autostart`, `tauri-plugin-single-instance`), `capabilities/default.json` (разрешения autostart), `icons/` (шаблонная иконка меню-бара `tray-template.png` 18×18 и `@2x` 36×36, монохромная, из `assets/icon.svg`: замок/скважина без плитки).

**Interfaces:**
- Consumes: `lifecycle::lock`, `AgentHandle::running`, событие `agent://unlock-needed` (Task 6).
- Produces:
  - Меню-бара (иконка `icon_as_template(true)`): строка статуса «Crypt открыт» / «Crypt заблокирован» (неактивная), «ssh-agent: работает (N ключей)» / «ssh-agent: выключен» (неактивная), разделитель, «Открыть Crypt», «Заблокировать» (⌘L, неактивно при заблокированном), разделитель, «Выйти Crypt» (⌘Q). Пункты статуса обновляются функцией `pub fn refresh_tray(app: &AppHandle)`, которую вызывают `lock`, все пути разблокировки, `cache::refresh` и старт/стоп агента.
  - `WindowEvent::CloseRequested` → `api.prevent_close()`, `window.hide()`, `app.set_activation_policy(ActivationPolicy::Accessory)`. «Открыть Crypt», клик по Dock и второй запуск (single-instance) → `set_activation_policy(Regular)`, `show()`, `set_focus()`.
  - `agent://unlock-needed` → показать окно (оно и так на экране разблокировки).
  - Автозапуск: `tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"]))`. `set_settings` с изменённым `launch_at_login` вызывает `enable()`/`disable()`. При запуске с `--hidden` окно не показывается, приложение стартует в режиме `Accessory`.
  - Автоблокировка (существующий `autolock`) и `lock` вызывают `agent.policy.clear_trust()`.

- [x] **Step 1: Тест** на чистую часть: `fn tray_labels(locked: bool, agent_running: bool, keys: usize) -> TrayLabels` (вынести тексты в чистую функцию). Тест `tray_labels_match_state`: (true, true, 2) → «Crypt заблокирован», «ssh-agent: работает (2 ключа)» с правильным склонением для 1/2/5/21 ключей; (false, false, 0) → «ssh-agent: выключен», пункт «Заблокировать» активен.
- [x] **Step 2: Run** → FAIL. **Step 3:** реализовать меню, окна, плагины. **Step 4:** `cargo test -p crypt-desktop` → PASS; `pnpm tauri build --debug` собирается.
- [ ] **Step 5: Ручная проверка** (или пункт в журнале для пользователя): закрыть окно → иконки в Dock нет, агент отвечает на `ssh-add -l`; «Открыть Crypt» возвращает окно; повторный запуск `Crypt.app` фокусирует первый экземпляр, второй процесс завершается.

---

### Task 9: Фронтенд — типы, API, мок и список секретов

**Files:** Create `src/lib/secretFields.ts`, `src/views/Secrets.vue`, `src/tests/secretFields.test.ts`. Modify `src/lib/types.ts`, `src/lib/api.ts`, `src/dev/mockBackend.ts`, `src/router.ts` (`/secrets` → `Secrets.vue`), `src/components/AppSidebar.vue` (если нужно). Delete `src/views/SecretsSoon.vue`.

**Interfaces:**
- Consumes: команды Tasks 3, 4, 6, 7 (имена — как там; аргументы в camelCase, как в существующем `api.ts`).
- Produces:
  - `types.ts`: `SecretOverview`, `SshKeyInfo`, `TotpCode`, `PasswordOptions`, `SshCandidate`, `SshImportItem`, `SshImportResult`, `AgentStatus`, `SshConfigStatus`, расширенный `Settings` (`clipboardClearSeconds`, `agentEnabled`, `agentTrustMinutes`, `launchAtLogin`).
  - `api.ts`: `secretOverview, revealSecret, copySecret, setSecret, secretHistory, revealSecretRevision, copySecretRevision, generatePassword, totpNow, copyTotp, setSshKey, generateSshKey, setAgentEnabled, scanSshDir, importSshKeys, agentStatus, sshConfigStatus, configureSshConfig, unconfigureSshConfig`.
  - `secretFields.ts`: `export interface SecretFieldMeta { name: string; label: string; sensitive: boolean; multiline?: boolean; mono?: boolean; generator?: boolean }` и `export const SECRET_FIELDS: Record<SecretKind, SecretFieldMeta[]>`. Порядок и имена совпадают с `value_names()` в ядре, а `sensitive` — со списком из Global Constraints. Подписи: Логин — «Логин», «Пароль» (generator), «Адрес», «Ключ TOTP»; Карта — «Номер», «Владелец», «Срок», «CVV», «PIN»; Токен — «Токен», «Адрес»; SSH-ключ — «Приватный ключ», «Публичный ключ», «Пароль ключа», «Комментарий»; Заметка — «Текст» (multiline). Также `export const SECRET_KIND_LABEL: Record<SecretKind, string>`: «Логин», «Карта», «Токен», «SSH-ключ», «Защищённая заметка». И `export function maskPreview(kind, name): string` (`••••••••`).
  - Мок-бэкенд: все новые команды в памяти. `copy*` пишет значение в `navigator.clipboard`, если оно есть. Агент в моке всегда «работает» с числом ключей из стора.
  - `Secrets.vue` по макету: заголовок «Секреты», вкладки «Все / Логины / SSH-ключи / Токены / Карты / Заметки» со счётчиками, поиск по названию (клиентский фильтр уже загруженных `RecordSummary`), кнопка «Новый секрет» (выпадающее меню по видам → `createRecord({ title: 'Новый <вид>', secretKind })` → переход на `/r/:id`), кнопка «Импорт из ~/.ssh» (открывает диалог из Task 11; до неё скрыта). Строка списка: иконка вида, название, папка, дата изменения. Значения (даже username) в списке не показываются и не запрашиваются.

- [x] **Step 1: Падающий тест** `secretFields.test.ts`: имена для каждого вида равны массивам из `value_names()` (скопировать литералами); набор `sensitive` равен списку из Global Constraints; у каждого вида есть подпись.
- [x] **Step 2: Run** `pnpm vitest run secretFields`. Ожидается FAIL.
- [x] **Step 3: Реализовать** типы, API, мок, `Secrets.vue` и маршрут.
- [ ] **Step 4: Run** `pnpm test` → PASS, `pnpm build` → ок; в Chrome (`pnpm dev`) раздел «Секреты» показывает вкладки и создаёт новый логин.

---

### Task 10: Карточка секрета и чип в полях

**Files:** Create `src/components/secret/{SecretPanel,SecretValue,PasswordGenerator,TotpCode,SecretHistory,SshKeySection,SecretChip}.vue`, `src/lib/reveal.ts`, `src/tests/reveal.test.ts`. Modify `src/views/Record.vue` (при `rec.secret_kind` показывать `SecretPanel` над редактором тела), `src/components/record/LinkPicker.vue` (для `secrets` рендерить `SecretChip`; в пустом списке «Секретов пока нет» → кнопка «Создать секрет» с выбором вида).

**Interfaces:**
- Consumes: `api.*` и `SECRET_FIELDS` (Task 9).
- Produces:
  - `reveal.ts`: `export function useRevealTimer(ms = 30_000): { revealed: Ref<string | null>; show(v: string): void; hide(): void }` скрывает значение через `ms`, при `onBeforeUnmount` и при событии блокировки (`session.beforeLock`). Значение хранится только в локальном `ref` компонента.
  - `SecretValue.vue` (props: `recordId`, `meta: SecretFieldMeta`, `filled: boolean`, `publicValue?: string`):
    - публичное поле показывается как есть, правится инлайн, сохраняется через `setSecret` на blur или Enter;
    - скрытое поле показывает `••••••••` (или «не задано»), а кнопки «Показать» (глаз; `revealSecret` → `useRevealTimer`), «Копировать» (`copySecret`; тост «Скопировано, буфер очистится через N с») и «Изменить». «Изменить» открывает пустое поле ввода (`type=password` с глазом; многострочное для `note`), Enter или «Сохранить» вызывает `setSecret`, Esc отменяет. Если значение было показано, поле предзаполняется им;
    - для `meta.generator` рядом с «Изменить» есть кнопка генератора;
    - если в истории по этому имени ≥ 1 версии, есть ссылка «История (N)» → `SecretHistory`.
  - `PasswordGenerator.vue`: поповер со слайдером длины (8–128), четырьмя переключателями классов, «без похожих», превью (моноширинный, янтарный), кнопками «Обновить» и «Использовать». Последние опции хранятся в `localStorage['crypt.pwgen']` (в try/catch), сам пароль туда не попадает.
  - `TotpCode.vue`: если `has_totp`, показывает текущий код (группы по 3 цифры) и кольцо обратного отсчёта, раз в секунду уменьшает `remaining`, при 0 перезапрашивает `totpNow`; клик копирует (`copyTotp`).
  - `SecretHistory.vue`: диалог со списком «заменено <дата>» и кнопками «Показать» и «Копировать» по каждой версии.
  - `SshKeySection.vue` (для `sshkey`): если ключа нет, кнопки «Сгенерировать ed25519» (`generateSshKey`) и «Вставить приватный ключ» (textarea + необязательный пароль → `setSshKey`; ошибки `PassphraseRequired` и `WrongPassphrase` показываются под полем). Если ключ есть: алгоритм, биты, фингерпринт (моно), публичный ключ (моно, обрезан, кнопка «Копировать» — копирование через `copySecret(id, 'public_key')`), переключатель «Доступен ssh-agent» (`setAgentEnabled`) и строка-подсказка, если агент выключен в настройках.
  - `SecretChip.vue` (props: `id`): название секрета, иконка вида, кнопки копирования по виду (логин: «логин», «пароль»; SSH: «публичный ключ»; токен: «токен»). Если секрет удалён, чип зачёркнут и неактивен. Значения в чип не загружаются, копирование идёт напрямую через `copySecret`.

- [x] **Step 1: Падающий тест** `reveal.test.ts` (fake timers): `show('x')` → `revealed == 'x'`; через 30 000 мс → `null`; `hide()` → `null`; повторный `show` перезапускает таймер.
- [x] **Step 2: Run** `pnpm vitest run reveal` → FAIL.
- [x] **Step 3: Реализовать** компоненты по макету «Секреты».
- [x] **Step 4: Run** `pnpm test` → PASS, `pnpm build` → ок.
- [ ] **Step 5: Проверка в Chrome (мок):** создать логин → сгенерировать пароль → показать и через 30 с он скрыт → копирование показывает тост → смена пароля → «История (1)» показывает старый. В карточке типа «Сервер» поле «Креды» показывает чип с кнопками копирования. Удалённый секрет даёт зачёркнутый чип. SSH-ключ: «Сгенерировать» показывает фингерпринт, переключатель агента работает.

---

### Task 11: Диалог импорта `~/.ssh` и настройки

**Files:** Create `src/components/secret/SshImportDialog.vue`, `src/lib/sshImport.ts`, `src/tests/sshImport.test.ts`. Modify `src/views/Secrets.vue` (кнопка импорта), `src/views/Settings.vue` (раздел «Секреты и ssh-agent»), `src/views/Onboarding.vue` (последний шаг-подсказка про агент ведёт в настройки, если он уже есть, — без новых шагов).

**Interfaces:**
- Consumes: `scanSshDir`, `importSshKeys`, `agentStatus`, `sshConfigStatus`, `configureSshConfig`, `unconfigureSshConfig`, `getSettings` / `setSettings`.
- Produces:
  - `sshImport.ts`: `export function defaultTitle(fileName: string, comment?: string): string` (комментарий, если есть и не пуст, иначе имя файла без префикса `id_`, например `id_ed25519_work` → `ed25519_work`), `export function selectable(c: SshCandidate): boolean` (не `alreadyImported`).
  - Диалог: список кандидатов с чекбоксами (по умолчанию отмечены все, кроме уже импортированных; те серые с меткой «уже в Crypt»), редактируемое название, поле пароля у зашифрованных, кнопка «Импортировать (N)». Результат построчно: галочка или текст ошибки; при ошибке пароля поле остаётся для повтора. Внизу текст: «Файлы в ~/.ssh не изменяются. Удалить их можно вручную, когда убедитесь, что агент Crypt работает.»
  - Настройки, раздел «Секреты и ssh-agent»: «Очищать буфер через» (Select 15/30/60/90 с); «ssh-agent» (Switch) + статус (`работает · N ключей · путь к сокету` моноширинным); «Доверять ключу после подтверждения» (Select: «каждый раз», 1, 5, 15 мин); блок «Подключить к ssh» со статусом `configured`, текстом блока (моно), кнопками «Добавить в ~/.ssh/config» / «Убрать» и копированием `exportLine`; «Запускать при входе в систему» (Switch). Всё на существующих компонентах shadcn `Select` и `Switch`.

- [x] **Step 1: Падающий тест** `sshImport.test.ts`: `defaultTitle('id_ed25519', 'anton@mac') == 'anton@mac'`; `defaultTitle('id_ed25519_work', '') == 'ed25519_work'`; `defaultTitle('deploy', undefined) == 'deploy'`; `selectable` ложно для `alreadyImported`.
- [x] **Step 2: Run** → FAIL. **Step 3:** реализовать. **Step 4:** `pnpm test` → PASS, `pnpm build` → ок.
- [ ] **Step 5: Проверка в Chrome (мок):** диалог импорта с тремя кандидатами (один зашифрован, один уже импортирован); неверный пароль показывает ошибку только у этой строки; настройки сохраняются и переживают перезагрузку страницы.

---

### Task 12: README, полная проверка, журнал ручных проверок

**Files:** Modify `README.md`, `docs/specs/2026-10-28-crypt-design.md` (раздел 6: зафиксировать решения плана — блок в конце `~/.ssh/config`, временное открытие хранилища для подписи при блокировке, окно доверия на ключ).

- [x] **Step 1: README.** Добавить раздел «ssh-agent»: как включить (кнопка в настройках или `export SSH_AUTH_SOCK=…`), что Touch ID спрашивается на каждую подпись, что делает «доверять N минут», что при закрытом окне агент работает из меню-бара, как проверить (`ssh-add -l`). Раздел «Секреты»: буфер очищается через N с, история значений, удаление секрета необратимо.
- [ ] **Step 2: Полная проверка:**
  `cargo test --workspace` → всё зелёное; `cargo clippy --workspace --all-targets -- -D warnings` → чисто; `cargo check -p crypt-core --target wasm32-unknown-unknown --no-default-features --features wasm` → собирается; `cd apps/desktop && pnpm test && pnpm build` → ок; `pnpm tauri build --debug` → `Crypt.app` собран.
- [x] **Step 3: Поиск утечек:** `grep -rn "println!\|dbg!\|eprintln!\|console.log" crates/crypt-core/src apps/desktop/src-tauri/src apps/desktop/src --include=*.rs --include=*.ts --include=*.vue`. Совпадений, печатающих значения секретов, быть не должно; отладочный вывод удалить.
- [x] **Step 4: Список ручных проверок для пользователя** (в итоговом сообщении и в журнале): Touch ID на подпись в `ssh` и `git`; подпись при заблокированном хранилище; отмена Touch ID → ssh переходит к следующему методу; закрытие окна → агент жив; автозапуск при входе; копирование пароля → через 30 с буфер пуст, а если скопировать своё — не трогается (проверить в другом приложении); кнопка `~/.ssh/config` → `ssh -G host | grep identityagent`.
- [x] **Step 5: Саморевью всего плана** по `superpowers:requesting-code-review` (в этой сессии, без субагентов), исправления — с тестом, если поведение наблюдаемо.
