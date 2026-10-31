# Бэкап и импорт — план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ручной зашифрованный бэкап всей базы с восстановлением при первом запуске, единый экран импорта с новыми источниками: CSV браузеров, Google Authenticator (QR), Aegis, 2FAS.

**Architecture:** Упаковка/распаковка и разбор форматов — модули в `crates/crypt-core` (рядом с `vault/import_keepass.rs`, за фичей импорта), каждый с тестами на образцах. Новые источники работают в два шага: `preview_*` (разбор без записи) → `import_*` (запись под `with_snapshot`). Команды Tauri — в `commands/transfer.rs` и новом `commands/backup.rs`. UI строится на компонентах и теме из плана редизайна.

**Tech Stack:** Rust (crypt-core, Tauri 2), Vue 3 + TS, vitest. Новые крейты: распознавание QR (`rqrr` + `image` с минимальными фичами `png`, `jpeg`), `scrypt`, `aes-gcm`; protobuf — ручной разбор (формат маленький).

**Spec:** `docs/superpowers/specs/2026-10-31-backup-import-design.md`
**Зависит от:** плана `2026-10-31-redesign-seal.md` (тема, `SegmentedControl`, `KindTile`, `SecretValuePill`, анимации, экран Настроек по разделам, онбординг в новом виде). Выполнять после него.
**Макеты:** `Import.dc.html`, `Restore.dc.html`, `Settings.dc.html` (раздел «Хранилище»), `Onboarding.dc.html` на холсте https://claude.ai/artifact/PCSwUujPE9RMxKYTJeKVua.

## Global Constraints

- Бэкап открывается только сид‑фразой хранилища; кроме служебного заголовка (магия `CRYPTBAK`, версия, дата, число объектов) ничего открытого.
- В бэкап не входят `device.json`, Связка ключей, настройки приложения.
- Восстановление доступно только при состоянии `noVault`.
- Импорт новых источников: сначала предпросмотр без записи, затем запись под `with_snapshot` (как KeePass).
- Открытые значения из CSV/JSON/QR — в `Zeroizing`, без временных файлов.
- Ограничение размера файла — существующий `MAX_IMPORT_FILE` (200 МБ); для бэкапа — 2 ГБ.
- Пропущенное и нераспознанное перечисляется в отчёте с причиной.
- `cargo test`, `pnpm test`, `pnpm build`, `pnpm format:check` зелёные после каждой задачи; новые команды имеют заглушки в `src/dev/mockBackend.ts`.

## Review Focus

1. CSV с BOM, `\r\n`, кавычками и переводами строк внутри поля (заметки Safari) — разбирается без смещения колонок.
2. Бэкап с повреждённым архивом или обрезанный файл — понятная ошибка, рабочая папка хранилища не создаётся/не остаётся наполовину.
3. Google Authenticator: QR из пакета, где отсканированы не все части (batch 1 из 2) — импортируется распознанное, в отчёте «не хватает QR 2 из 2».
4. Логин с тем же доменом в двух записях (личный и рабочий) — код 2FA не прикрепляется ни к одной, создаётся новый секрет.
5. Экспорт бэкапа во время правки записи — архив согласован (не смесь старых и новых объектов).

---

## Структура файлов

| Файл | Ответственность |
|---|---|
| `crates/crypt-core/src/vault/backup.rs` | формат `.cryptbackup`: запись, чтение заголовка, проверка фразы, распаковка |
| `crates/crypt-core/src/vault/import_csv.rs` | разбор CSV браузеров → `Vec<LoginDraft>` |
| `crates/crypt-core/src/vault/import_otp.rs` | `otpauth-migration` (protobuf), Aegis, 2FAS → `Vec<OtpDraft>`; QR → строки |
| `crates/crypt-core/src/vault/import_apply.rs` | дубли, сопоставление 2FA с логинами, запись секретов |
| `apps/desktop/src-tauri/src/commands/backup.rs` | `export_backup`, `inspect_backup`, `restore_backup` |
| `apps/desktop/src-tauri/src/commands/transfer.rs` | `preview_browser_csv`, `import_browser_csv`, `preview_otp`, `import_otp`, `trash_file` |
| `apps/desktop/src/views/Import.vue` | мастер импорта |
| `apps/desktop/src/views/Restore.vue` | восстановление при первом запуске |
| `apps/desktop/src/components/BackupCard.vue` | карточка бэкапа в Настройках |

---

### Task 1: Формат бэкапа в ядре

**Files:**
- Create: `crates/crypt-core/src/vault/backup.rs` (+ `mod` и реэкспорт в `lib.rs`)
- Test: в том же файле, `#[cfg(test)]`

**Interfaces:**
- Produces:
  ```rust
  pub struct BackupHeader { pub version: u16, pub created_at: i64, pub objects: u32 }
  pub fn write_backup(root: &Path, out: &mut impl Write, now: i64) -> Result<BackupHeader>;
  pub fn read_backup_header(input: &mut impl Read) -> Result<BackupHeader>;
  pub fn verify_backup_seed(bytes: &[u8], seed: &SeedPhrase) -> Result<()>;      // по keycheck из архива в памяти
  pub fn restore_backup(bytes: &[u8], seed: &SeedPhrase, root: &Path) -> Result<BackupHeader>;
  ```
  Ошибки: `CryptError::BackupFormat("Это не файл бэкапа Crypt")`, `BackupFormat("Файл от более новой версии Crypt")`, `BackupFormat("Файл повреждён")`, `BackupSeedMismatch`. Добавить варианты в `error.rs` и их маппинг в `ApiError` с кодами `BackupFormat`/`BackupSeedMismatch`.
  Формат: `b"CRYPTBAK"` + `u16` версия (1) + JSON‑заголовок с длиной `u32` + поток записей `{ путь (u16 длина + UTF‑8), размер u64, байты }` для `vault.json`, `keycheck.bin`, `seed.bin`, `meta.bin`, `objects/*.bin` (+ подпапки снимков, если есть — включить всё, кроме `device.json`). Пути только относительные, без `..` (проверять при чтении). Контрольная сумма SHA‑256 всего потока в конце.
  `restore_backup`: проверка фразы → распаковка во временную папку‑соседа `root` → `Vault::open_with_seed` на ней → `rename` в `root`. При любой ошибке временная папка удаляется, `root` не создаётся.

- [ ] **Step 1: Тесты**:
  - `roundtrip`: создать хранилище (`Vault::create`), добавить запись и секрет, `write_backup` → `restore_backup` в новый `root` → `open_with_seed` видит запись и значение секрета.
  - `device_json_not_included`: после восстановления `device.json` нет до первого открытия; `peer_id` после открытия отличается от исходного.
  - `wrong_seed`: другая фраза → `BackupSeedMismatch`, `root` не существует.
  - `not_a_backup`: случайные байты → `BackupFormat("Это не файл бэкапа Crypt")`.
  - `future_version`: версия 99 → `BackupFormat("Файл от более новой версии Crypt")`.
  - `truncated`: обрезать на середине → `BackupFormat("Файл повреждён")`, `root` не существует, временной папки нет.
  - `path_traversal`: запись с путём `../x` → `BackupFormat("Файл повреждён")`.
- [ ] **Step 2:** `cargo test -p crypt-core backup` → FAIL.
- [ ] **Step 3:** Реализовать.
- [ ] **Step 4:** `cargo test -p crypt-core` → PASS.
- [ ] **Step 5: Commit** `feat(core): формат бэкапа .cryptbackup`

### Task 2: Команды бэкапа и карточка в Настройках

**Files:**
- Create: `apps/desktop/src-tauri/src/commands/backup.rs`; Modify: `commands/mod.rs`, `lib.rs` (регистрация), `state.rs`/настройки (поле `lastBackupAt: Option<i64>` в `Settings`)
- Create: `apps/desktop/src/components/BackupCard.vue`; Modify: `src/views/Settings.vue` (раздел «Хранилище»), `src/lib/api.ts`, `src/lib/types.ts`, `src/dev/mockBackend.ts`
- Test: Rust‑тесты в `backup.rs` (как в `transfer.rs`: `AppState` во временной папке); `src/tests/backupReminder.test.ts`

**Interfaces:**
- Produces (Tauri): `export_backup(path: String, pin: Option<String>) -> { path, bytes, header }` — подтверждение присутствия как в `reveal_seed`, упаковка под блокировкой хранилища (держать `vault_guard` на время записи), запись в `path.tmp` → `rename`; обновляет `settings.lastBackupAt`.
- Produces (TS): `api.exportBackup(path, pin?)`, `Settings.lastBackupAt: number | null`; `backupReminder(lastAt: number | null, now: number): 'none' | 'never' | 'stale'` — `'never'` при `null`, `'stale'` старше 30 дней, иначе `'none'`.
- Consumes: диалог сохранения `@tauri-apps/plugin-dialog` `save` (добавить разрешение `dialog:allow-save` в capabilities).

- [ ] **Step 1: Тесты**: Rust — `export_backup_writes_file_and_updates_last_backup` (файл существует, `read_backup_header` читается, `lastBackupAt` выставлен); `export_requires_presence` (`FakeAuthenticator` с отказом → ошибка, файла нет). TS — `backupReminder` для `null`, 10 и 31 дня.
- [ ] **Step 2:** FAIL → реализовать → PASS.
- [ ] **Step 3:** `BackupCard.vue` по `Settings.dc.html`: «Экспорт бэкапа…» → подтверждение (Touch ID / поле PIN, как показ сид‑фразы) → диалог сохранения (`Crypt-ГГГГ-ММ-ДД.cryptbackup`) → полоса прогресса → итог с размером и «Показать в Finder»; строка «Последний бэкап: <дата>» или напоминание. Под карточкой — строка «Экспорт в Markdown» (существующая функция из `TransferSection`).
- [ ] **Step 4:** `cargo test`, `pnpm test`, `pnpm build` → PASS.
- [ ] **Step 5: Commit** `feat: экспорт бэкапа`

### Task 3: Восстановление при первом запуске

**Files:**
- Modify: `commands/backup.rs` (`inspect_backup`, `restore_backup`), `src/router.ts` (маршрут `/restore`, разрешён в `noVault`), `src/views/Onboarding.vue` (третий вариант «Из бэкапа»), `src/stores/session.ts`
- Create: `src/views/Restore.vue`

**Interfaces:**
- Produces (Tauri): `inspect_backup(path) -> BackupHeader` (только заголовок); `restore_backup(path, words: Vec<String>) -> VaultStatus` — доступна только при `noVault`, иначе ошибка `VaultExists`; после успеха хранилище открыто (`install`), как после `unlock_with_seed`.
- Produces (TS): `api.inspectBackup`, `api.restoreBackup`; `session.restoreBackup(path, words)`.

- [ ] **Step 1: Rust‑тесты**: `restore_when_vault_exists_fails`; `restore_opens_vault` (после — статус `unlocked`, записи на месте); `restore_wrong_seed_keeps_novault`.
- [ ] **Step 2:** FAIL → реализовать → PASS.
- [ ] **Step 3:** `Restore.vue` по `Restore.dc.html` (тёмная панель шагов как в онбординге): файл (диалог/перетаскивание, `inspect_backup` для проверки и размера) → сид‑фраза со счётчиком → «Проверяем…» → прогресс распаковки → переход на шаг «Быстрый вход» онбординга (`/onboarding?step=4`). Ошибки — тексты из ядра, неверная фраза — тряска.
- [ ] **Step 4:** Онбординг: третья карточка «Из бэкапа» → `/restore`.
- [ ] **Step 5:** `cargo test`, `pnpm test`, `pnpm build` → PASS; `pnpm tauri dev` на чистой папке данных: экспорт на одном хранилище, удаление хранилища в Настройках, восстановление из файла.
- [ ] **Step 6: Commit** `feat: восстановление из бэкапа`

### Task 4: CSV браузеров в ядре

**Files:**
- Create: `crates/crypt-core/src/vault/import_csv.rs`, `crates/crypt-core/src/vault/import_apply.rs`
- Test: в файлах, образцы в `crates/crypt-core/tests/fixtures/csv/{chrome,safari,firefox}.csv` (синтетические данные)

**Interfaces:**
- Produces:
  ```rust
  pub enum CsvFormat { Chromium, Safari, Firefox }
  pub struct LoginDraft { pub title: String, pub url: Option<String>, pub username: String,
                          pub password: Zeroizing<String>, pub note: Option<String>, pub totp: Option<Zeroizing<String>> }
  pub struct CsvParse { pub format: CsvFormat, pub logins: Vec<LoginDraft>, pub skipped: Vec<(usize, SkipReason)> }
  pub enum SkipReason { NoPassword, BadRow, BadTotp }
  pub fn parse_browser_csv(bytes: &[u8]) -> Result<CsvParse>;
  // import_apply.rs
  pub struct LoginPreview { pub new: usize, pub duplicates: usize, pub skipped: usize, pub sample: Vec<(String, String)> /* сайт, логин; ≤ 5 */ }
  impl Vault {
      pub fn preview_logins(&self, drafts: &[LoginDraft]) -> LoginPreview;
      pub fn import_logins(&mut self, drafts: Vec<LoginDraft>, folder_name: &str) -> Result<LoginImportReport>;
  }
  ```
  Формат по заголовкам: Chromium `name,url,username,password,note`; Safari `Title,URL,Username,Password,Notes,OTPAuth`; Firefox `url,username,password,…`. Иначе `ImportFormat("Не похоже на выгрузку паролей браузера. Ожидаются колонки: …")`. Название: `name`/`Title`, иначе домен из URL. Дубль: тот же домен (нормализация: нижний регистр, без `www.`) + логин + пароль — с существующим логином или с более ранней строкой файла. Папка создаётся, если её нет. `totp` → `parse_totp`; ошибка → `SkipReason::BadTotp` для TOTP, сам логин импортируется.
  CSV разбирается своим небольшим парсером RFC 4180 (BOM, `\r\n`, кавычки, переводы строк в полях) — или крейтом `csv`, на усмотрение исполнителя.

- [ ] **Step 1: Тесты**: каждый образец определяет формат и даёт ожидаемое число логинов; Safari с многострочной заметкой и `OTPAuth`; BOM + `\r\n`; строка без пароля → `NoPassword`; дубль внутри файла и дубль с существующим → `duplicates`; тот же домен и логин с другим паролем → новая запись; неизвестные заголовки → `ImportFormat`.
- [ ] **Step 2:** FAIL → реализовать → PASS (`cargo test -p crypt-core import_csv import_apply`).
- [ ] **Step 3: Commit** `feat(core): импорт CSV браузеров`

### Task 5: Коды 2FA в ядре

**Files:**
- Create: `crates/crypt-core/src/vault/import_otp.rs`; Modify: `crates/crypt-core/Cargo.toml` (`rqrr`, `image` с `default-features = false, features = ["png", "jpeg"]`, `scrypt`, `aes-gcm` — под фичей импорта), `import_apply.rs`
- Test: образцы `tests/fixtures/otp/{ga_batch1.png, ga_batch2.png, otpauth.png, aegis_plain.json, aegis_encrypted.json, twofas.2fas}` (сгенерированы для тестов, синтетические ключи; пароль зашифрованного Aegis — в тесте)

**Interfaces:**
- Produces:
  ```rust
  pub struct OtpDraft { pub issuer: String, pub account: String, pub secret: Zeroizing<Vec<u8>>,
                        pub algorithm: TotpAlgorithm, pub digits: u32, pub period: u32 }
  pub struct OtpParse { pub drafts: Vec<OtpDraft>, pub skipped: Vec<(String, OtpSkip)>, pub missing_batches: Vec<(u32, u32)> }
  pub enum OtpSkip { Hotp, BadSecret, Unsupported }
  pub fn decode_qr_images(images: &[Vec<u8>]) -> Result<Vec<String>>;        // строки из всех QR на всех картинках
  pub fn parse_otp_uris(uris: &[String]) -> OtpParse;                        // otpauth:// и otpauth-migration://
  pub fn parse_aegis(bytes: &[u8], password: Option<&str>) -> Result<OtpParse>;   // NeedsPassword, если зашифрован и пароля нет
  pub fn parse_2fas(bytes: &[u8]) -> Result<OtpParse>;                       // зашифрованный → ImportFormat("Экспортируйте 2FAS без пароля")
  pub enum OtpTarget { Attach { record: RecordId, title: String }, New }
  pub struct OtpPreviewRow { pub issuer: String, pub account: String, pub target: OtpTarget }
  impl Vault {
      pub fn preview_otp(&self, drafts: &[OtpDraft]) -> Vec<OtpPreviewRow>;
      pub fn import_otp(&mut self, drafts: Vec<OtpDraft>, folder_name: &str) -> Result<OtpImportReport>;
  }
  ```
  `otpauth-migration`: base64 из `data` → protobuf `MigrationPayload { repeated OtpParameters otp_parameters = 1; int32 version = 2; int32 batch_size = 3; int32 batch_index = 4; int32 batch_id = 5; }`, `OtpParameters { bytes secret = 1; string name = 2; string issuer = 3; Algorithm algorithm = 4; DigitCount digits = 5; OtpType type = 6; int64 counter = 7; }` (Algorithm: 1 SHA1, 2 SHA256, 3 SHA512, 4 MD5→`Unsupported`; DigitCount: 1→6, 2→8; OtpType: 1 HOTP→`Hotp`, 2 TOTP). `missing_batches` — пары `(index+1, size)` для партий одного `batch_id`, которых нет. Имя `issuer:account` в `name` — разделить, если `issuer` пуст.
  Aegis: `db` открытый или зашифрованный (`header.slots` с `type: 1` — scrypt `n,r,p,salt` → ключ слота → AES‑256‑GCM расшифровывает мастер‑ключ → AES‑256‑GCM расшифровывает `db` с `header.params.nonce/tag`), по документации формата Aegis.
  Сопоставление (`preview_otp`): логины, у которых домен `url` совпадает с `issuer` (нижний регистр; `issuer` «GitHub» ↔ `github.com`, сравнение по первой метке домена и по полному домену) и, если у кандидатов есть `username`, `username == account`. Ровно один кандидат без `totp_seed` → `Attach`, иначе `New`. Новый секрет: вид «Логин», название `issuer` (или `account`), `username = account`, `totp_seed` = `otpauth://totp/...` из параметров.

- [ ] **Step 1: Тесты**: два PNG пакета → 2 строки миграции → все аккаунты, `missing_batches` пуст; только первый → `missing_batches == [(2, 2)]`; HOTP → `Hotp`; Aegis открытый и зашифрованный (верный пароль; неверный → ошибка `ImportPassword`); 2FAS; сопоставление: один подходящий логин → `Attach`, два (личный и рабочий) → `New`, логин уже с `totp_seed` → `New`; `import_otp` прикрепляет код к найденному логину и создаёт остальные в папке.
- [ ] **Step 2:** FAIL → реализовать → PASS.
- [ ] **Step 3: Commit** `feat(core): импорт кодов 2FA`

### Task 6: Команды импорта

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/transfer.rs`, `lib.rs` (регистрация), `src/lib/api.ts`, `src/lib/types.ts`, `src/dev/mockBackend.ts`

**Interfaces:**
- Produces (Tauri, все `async`):
  - `preview_browser_csv(path) -> { format: 'chromium'|'safari'|'firefox', new, duplicates, skipped, sample: [site, login][] }`
  - `import_browser_csv(path, folder: String) -> Imported<LoginImportReport>` (повторный разбор файла внутри, под `with_snapshot`)
  - `preview_otp(source: { kind: 'images', paths: string[] } | { kind: 'clipboardImage' } | { kind: 'aegis' | 'twofas', path: string, password?: string }) -> { token: string, rows: { issuer, account, target: { attach: string } | 'new' }[], skipped, missingBatches }` — разобранные черновики держатся в `AppState` (`Mutex<Option<(token, Vec<OtpDraft>)>>`, `Zeroizing`), очищаются при импорте, отмене и блокировке.
  - `import_otp(token, folder) -> Imported<OtpImportReport>`; `cancel_otp_import()`.
  - `trash_file(path)` — перемещение в системную Корзину (`NSFileManager trashItemAtURL` через `objc2`/`trash`‑крейт на усмотрение исполнителя).
  - Картинка из буфера: читать изображение буфера в Rust (`arboard` `get_image` или нативно; если тяжело — только выбор файлов, а кнопку «Вставить» скрыть — решение исполнителя с отметкой в отчёте задачи).
- Produces (TS): одноимённые методы `api.*`.

- [ ] **Step 1: Rust‑тесты** (по образцу `transfer.rs`): `import_browser_csv` создаёт логины и снимок; `preview_otp` → `import_otp` по токену; неверный/просроченный токен → ошибка; `lock` очищает черновики.
- [ ] **Step 2:** FAIL → реализовать → PASS.
- [ ] **Step 3: Commit** `feat: команды импорта браузеров и 2FA`

### Task 7: Экран импорта

**Files:**
- Create: `apps/desktop/src/views/Import.vue`, `src/lib/importSources.ts`
- Modify: `src/router.ts` (`/import`), `src/views/Secrets.vue` (кнопка «Импорт» → `/import`), `src/views/Settings.vue` (строка «Импорт» в разделе «Хранилище» → `/import`; импорт KeePass/Markdown убрать из `TransferSection`, экспорт в Markdown оставить), `src/components/secret/SshImportDialog.vue` (вызывается с экрана импорта)
- Test: `src/tests/importSources.test.ts`

**Interfaces:**
- Produces: `IMPORT_SOURCES` — группы «Пароли из браузера» (`chrome`, `safari`, `firefox`, `edge`, `brave`), «Коды 2FA» (`ga`, `aegis`, `twofas`), «Другие менеджеры» (`keepass`, `markdown`, `ssh`), у каждого `{ id, name, format, steps: string[] }` — тексты инструкций из `Import.dc.html`; `defaultImportFolder(kind: 'browser' | 'otp', date: Date): string` → `«Импорт из браузера · 31 окт»` / `«Импорт 2FA · 31 окт»`.

- [ ] **Step 1: Тесты**: у каждого браузера ровно 3 шага; `defaultImportFolder('browser', new Date(2026, 9, 31)) === 'Импорт из браузера · 31 окт'`.
- [ ] **Step 2:** FAIL → реализовать → PASS.
- [ ] **Step 3:** `Import.vue` по `Import.dc.html`: источник → инструкция (вкладки браузеров) и выбор файла (диалог + перетаскивание) → предпросмотр (счётчики, первые строки с замаскированными паролями, «к логину X»/«новый секрет», папка) → «Импортируем…» → итог с отчётом пропущенного. После CSV — янтарная плашка «Файл хранит пароли открытым текстом» + «Переместить в корзину» (`trash_file`); после QR из файлов — то же для скриншотов. KeePass, Markdown, SSH — прежние формы/диалог внутри мастера, без предпросмотра, с итоговым отчётом.
- [ ] **Step 4:** `pnpm test`, `pnpm build`, `cargo test` → PASS; в `pnpm dev` пройти все ветки на заглушках.
- [ ] **Step 5: Commit** `feat(ui): экран импорта`

### Task 8: Итоговая проверка

- [ ] **Step 1:** `cargo test`, `pnpm test`, `pnpm build`, `pnpm format:check` → PASS.
- [ ] **Step 2:** Ручная проверка в `pnpm tauri dev`: реальная выгрузка CSV из Safari и Chrome; экспорт Google Authenticator (2 QR); экспорт Aegis зашифрованный; бэкап → удаление хранилища → восстановление.
- [ ] **Step 3:** Обновить `README.md` (раздел о бэкапе и импорте, 3–5 строк).
- [ ] **Step 4: Commit** `docs: бэкап и импорт в README`
