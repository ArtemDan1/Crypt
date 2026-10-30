# Crypt Desktop (план 2 из 3) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans (решение пользователя: без субагентов). Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** macOS-приложение Crypt на Tauri 2: онбординг с сид-фразой, разблокировка Touch ID / паролем Mac / PIN, автоблокировка, база знаний — записи с Markdown-редактором и wiki-ссылками, поля типов, конструктор типов, таблицы с сохранёнными видами, папки, теги, корзина, быстрый поиск.

**Architecture:** Крейт `crypt-desktop` (`apps/desktop/src-tauri`) держит `Vault` из `crypt-core` в `Mutex` внутри состояния Tauri и выставляет тонкие команды. Платформенная разблокировка — модуль `unlock` за двумя трейтами (`KeyStore`, `Authenticator`), чтобы логику PIN и Touch ID тестировать без Keychain. Фронтенд — Vue 3 + TS + Pinia + vue-router, компоненты shadcn-vue, CodeMirror 6 для заметок, TanStack Table для таблиц. Типы DTO во фронтенде пишутся вручную в `src/lib/types.ts` и зеркалят serde-формат ядра.

**Tech Stack:** Tauri 2 (`tauri` 2.x, `@tauri-apps/cli` и `@tauri-apps/api` 2.12), Vue 3.5, Vite, Tailwind CSS 4, shadcn-vue (reka-ui), Pinia, vue-router, lucide-vue-next, CodeMirror 6 (`codemirror`, `@codemirror/lang-markdown`), `@tanstack/vue-table`, Vitest; Rust: `security-framework` 3, `objc2-local-authentication` 0.3, `objc2`/`objc2-foundation`/`block2`, `argon2` 0.5 (стабильная), `crypt-core`. Пакетный менеджер — pnpm.

**Spec:** `docs/specs/2026-10-28-crypt-design.md`. **Макеты:** https://claude.ai/artifact/6WNytj3nPs5x1k1EcEEg8U. **Ядро:** `crates/crypt-core` (план 1, готов).

**Не входит (план 3):** UI секретов (создание и просмотр логинов, ключей, карт), копирование в буфер с очисткой, генератор паролей, TOTP, ssh-agent, импорт `~/.ssh`, меню-бар. В плане 2 поле-ссылка на секрет показывает только чип с названием секрета, а выбрать можно из уже существующих секретов.

## Global Constraints

- **Git не трогать** (указание пользователя): никаких `git init` и коммитов. Шаги «Commit» из шаблона опущены.
- Без платного Apple Developer аккаунта: ключ данных хранится в login keychain (обычный `SecItemAdd` без data-protection keychain и entitlements). Touch ID проверяется через `LAContext.evaluatePolicy(.deviceOwnerAuthentication)`: Touch ID с откатом на пароль Mac.
- Keychain: service `dev.crypt.app`, account `data-key:<vault_id>` (32 байта ключа данных), `pin:<vault_id>` (JSON обёртки PIN), `pin-attempts:<vault_id>` (счётчик).
- PIN — ровно 6–12 цифр, минимум 6. Обёртка: Argon2id (m = 64 MiB, t = 3, p = 1, соль 16 байт) → ключ 32 байта → `crypt_core::crypto::envelope::seal` ключа данных с AAD `crypt/v1/pin-wrap/<vault_id>`. После **5** неверных PIN подряд элемент `pin:` и счётчик удаляются, нужна сид-фраза или Touch ID.
- Автоблокировка: по умолчанию **15 мин** бездействия (настраивается: 1, 5, 15, 30, 60 мин), а также при блокировке экрана и при сне Mac. Блокировка = `Vault` выброшен из памяти (ключи обнуляются через `ZeroizeOnDrop`).
- Повторный показ сид-фразы — только после свежего подтверждения (Touch ID / пароль Mac / PIN), даже при открытом хранилище.
- Хранилище: `<app_data_dir>/vault` (`~/Library/Application Support/dev.crypt.app/vault`). Настройки (не секретные): `<app_data_dir>/settings.json`.
- Фронтенд никогда не получает значения секретов в этом плане. Команд, возвращающих `secret_value`, здесь нет.
- Строгий CSP: `default-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; img-src 'self' data:`. Шрифты Geist / Geist Mono ставятся npm-пакетом `geist`, не с Google Fonts. Никаких сетевых запросов.
- Сохранение тела заметки — с задержкой **600 мс** после последнего ввода и при уходе со страницы.
- Тема: тёмная по умолчанию, светлая по системе (`prefers-color-scheme`); палитра shadcn zinc; акцент `#fbbf24` только для элементов, связанных с секретами; шрифты Geist, Geist Mono для IP, ключей и кода.
- Язык интерфейса — русский.
- Горячие клавиши: ⌘K — поиск, ⌘N — новая запись, ⌘L — заблокировать.
- Ошибки ядра приходят во фронтенд как `{ code: string, message: string }`. `code` — имя варианта `CryptError` (`WrongKey`, `Validation`, …). Секретов в `message` нет.

## Review Focus

1. **Неверный PIN 5 раз, потом верный:** верный PIN после пятой ошибки уже не принимается (`PinLocked`), экран предлагает Touch ID или сид-фразу; счётчик сбрасывается после успешного входа любым способом. → тест в Task 3.
2. **Touch ID отменён или недоступен** (нажали «Отмена», нет сенсора, крышка закрыта): приложение остаётся на экране разблокировки с понятным сообщением, без паники и без «вечной загрузки». → тест с фейковым `Authenticator` в Task 3.
3. **Быстрый ввод и сразу уход со страницы или блокировка:** последние символы, набранные меньше чем за 600 мс до ухода, не теряются: отложенное сохранение сбрасывается на `onBeforeUnmount` и перед `lock`. → тест в Task 8.
4. **Ссылка `[[id|алиас]]` на запись, которую удалили или переименовали:** переименованная показывает новый заголовок, удалённая — зачёркнутый алиас без перехода, без ошибки. → тест в Task 8.
5. **Тип изменили, пока открыта таблица** (удалили колонку из сохранённого вида, сменили вид поля): таблица и вид открываются, неизвестные колонки молча отбрасываются из конфига. → тест в Task 10.

---

## Структура файлов

```
Cargo.toml                                   # workspace: + "apps/desktop/src-tauri"
apps/desktop/
  package.json, pnpm-lock.yaml, vite.config.ts, tsconfig*.json, index.html, components.json
  src/
    main.ts, App.vue, router.ts
    styles/app.css                           # Tailwind 4 + токены темы
    lib/types.ts                             # DTO, зеркало serde ядра
    lib/api.ts                               # обёртки invoke + CryptApiError
    lib/fields.ts                            # формат и разбор значений полей
    lib/wikilinks.ts                         # разбор/вставка [[id|alias]]
    lib/table.ts                             # сортировка/фильтры/очистка ViewConfig
    lib/debounce.ts                          # debouncedSaver с flush()
    stores/session.ts                        # статус хранилища, lock/unlock, автоблокировка
    stores/library.ts                        # типы, папки, списки записей, кэш
    components/ui/*                          # shadcn-vue (генерируется CLI)
    components/AppShell.vue, AppSidebar.vue, CommandPalette.vue
    components/record/{RecordHeader,FieldsPanel,FieldEditor,BacklinksPanel,TagEditor}.vue
    components/editor/{MarkdownEditor.vue, livePreview.ts, wikilinkExtension.ts}
    components/table/{TypeTable.vue, ViewTabs.vue, FilterBar.vue}
    components/types/{TypeList.vue, TypeEditor.vue, FieldKindMenu.vue}
    views/{Onboarding,Unlock,Record,TypeTableView,Types,Trash,Settings,Home}.vue
    tests/*.test.ts                          # vitest для lib/*
  src-tauri/
    Cargo.toml, build.rs, tauri.conf.json, capabilities/default.json, icons/
    src/main.rs, src/lib.rs                  # сборка Builder, регистрация команд
    src/state.rs                             # AppState, Settings
    src/error.rs                             # ApiError {code,message} из CryptError
    src/commands/{mod,lifecycle,library,unlock}.rs
    src/unlock/{mod,keystore,authenticator,pin,macos}.rs
    src/autolock.rs                          # наблюдатели сна и блокировки экрана
README.md                                    # запуск, подпись самоподписанным сертификатом
```

---

### Task 1: Каркас Tauri + Vue + Tailwind + shadcn-vue с темой

**Files:** создать всё в `apps/desktop/` из структуры выше, кроме views/components логики; Modify: корневой `Cargo.toml`.

**Interfaces:**
- Produces: приложение `crypt-desktop` (identifier `dev.crypt.app`, productName `Crypt`, окно 1280×800, минимум 960×640, `titleBarStyle: "Overlay"`, `hiddenTitle: true`); алиас `@/` → `src/`; CSS-переменные shadcn (zinc) для тёмной и светлой темы; `--accent-secret: #fbbf24`.

- [ ] **Step 1:** `pnpm create vite apps/desktop --template vue-ts`; добавить `@tauri-apps/cli`, `@tauri-apps/api`, `tailwindcss @tailwindcss/vite`, `geist`, `pinia`, `vue-router`, `lucide-vue-next`, `vitest`; `pnpm tauri init` (src-tauri внутри `apps/desktop`); добавить `apps/desktop/src-tauri` в members workspace.
- [ ] **Step 2:** `pnpm dlx shadcn-vue@latest init` (стиль new-york, база zinc, CSS variables); добавить компоненты: `button input label dialog command dropdown-menu context-menu popover select switch tabs table tooltip badge separator scroll-area sonner checkbox`.
- [ ] **Step 3:** В `styles/app.css` подключить Geist / Geist Mono из пакета `geist`; `html` по умолчанию с классом `dark`, светлая тема через `@media (prefers-color-scheme: light)` снимает класс в `main.ts`. CSP из Global Constraints — в `tauri.conf.json`.
- [ ] **Step 4: Проверка.** Run: `cd apps/desktop && pnpm build && cargo check -p crypt-desktop`. Expected: оба без ошибок. Затем `pnpm tauri dev`: открывается тёмное окно с заглушкой «Crypt» (проверить запуск, окно закрыть).

---

### Task 2: Состояние, ошибки и жизненный цикл хранилища (Rust)

**Files:** Create: `src-tauri/src/{state,error}.rs`, `src-tauri/src/commands/{mod,lifecycle}.rs`; Modify: `src-tauri/src/lib.rs`
**Test:** `#[cfg(test)]` в `lifecycle.rs` (функции-ядра вызываются напрямую с `tempdir`)

**Interfaces:**
- Consumes: `crypt_core::{Vault, SeedPhrase, CryptError, VaultId}`
- Produces:
  - `pub struct ApiError { pub code: String, pub message: String }` (Serialize); `impl From<CryptError>`: `code` = имя варианта, `message` — русский текст без данных пользователя (`WrongKey` → «Неверная сид-фраза или ключ»).
  - `pub struct AppState { pub vault: Mutex<Option<Vault>>, pub vault_dir: PathBuf, pub settings: Mutex<Settings>, pub settings_path: PathBuf }`
  - `pub struct Settings { pub autolock_minutes: u32 /* default 15 */ }` (serde, `load(path)`/`save(path)`).
  - `#[derive(Serialize)] pub enum VaultStatus { NoVault, Locked { vault_id: String }, Unlocked { vault_id: String } }` (`#[serde(tag = "state")]`).
  - Функции-ядра (тестируемые без Tauri) и команды-обёртки с теми же именами:
    `status(&AppState) -> VaultStatus`, `generate_seed() -> Vec<String>` (24 слова),
    `create_vault(&AppState, words: Vec<String>) -> Result<VaultStatus, ApiError>`,
    `unlock_with_seed(&AppState, words: Vec<String>) -> Result<VaultStatus, ApiError>`,
    `lock(&AppState) -> VaultStatus`.
  - `vault_id` для статуса `Locked` читается из открытого `vault.json` (поле `vault_id`) без ключа.

- [ ] **Step 1: Падающие тесты**

```rust
#[test] fn status_no_vault_then_unlocked_then_locked() { /* status == NoVault; create_vault(words) → Unlocked{vault_id}; lock → Locked{ тот же vault_id } */ }
#[test] fn unlock_with_wrong_seed_maps_to_wrong_key() { /* другая фраза → Err(ApiError{code:"WrongKey",..}); message не содержит ни одного слова фраз */ }
#[test] fn create_twice_is_vault_exists() { /* code == "VaultExists" */ }
#[test] fn settings_default_and_roundtrip() { /* нет файла → autolock_minutes == 15; save 5 → load == 5 */ }
```

- [ ] **Step 2: Run** `cargo test -p crypt-desktop lifecycle` → FAIL.
- [ ] **Step 3: Реализовать** и зарегистрировать команды в `lib.rs` (`tauri::generate_handler!`); `AppState` создаётся в `setup` из `app.path().app_data_dir()`.
- [ ] **Step 4: Run** `cargo test -p crypt-desktop` → PASS.

---

### Task 3: Разблокировка — Keychain, Touch ID, PIN (Rust)

**Files:** Create: `src-tauri/src/unlock/{mod,keystore,authenticator,pin,macos}.rs`, `src-tauri/src/commands/unlock.rs`
**Test:** юнит-тесты в `pin.rs` и `unlock/mod.rs` с `MemoryKeyStore` и `FakeAuthenticator`

**Interfaces:**
- Consumes: `AppState`, `ApiError`, `crypt_core::{SecretKey, Vault}`, `crypt_core::crypto::envelope::{seal, open}` (сделать модуль `crypto` публичным в ядре, если он ещё не `pub`)
- Produces:
  - `pub trait KeyStore: Send + Sync { fn get(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>, ApiError>; fn set(&self, account: &str, data: &[u8]) -> Result<(), ApiError>; fn delete(&self, account: &str) -> Result<(), ApiError>; }`
    Реализации: `MacKeychain` (`security_framework::passwords::{get_generic_password,set_generic_password,delete_generic_password}`, service `dev.crypt.app`), `MemoryKeyStore` (тесты).
  - `pub trait Authenticator: Send + Sync { fn authenticate(&self, reason: &str) -> Result<(), AuthError>; fn available(&self) -> bool; }`, `pub enum AuthError { Cancelled, Unavailable, Failed }`.
    `MacAuthenticator` в `macos.rs`: `LAContext::new()`, `canEvaluatePolicy`, `evaluatePolicy_localizedReason_reply(LAPolicy::DeviceOwnerAuthentication, reason, block)`; ответ блока через `std::sync::mpsc`, вызов из `tauri::async_runtime::spawn_blocking`.
  - `pin.rs`: `pub fn wrap(pin: &str, data_key: &SecretKey, vault_id: &str) -> Result<PinWrap, ApiError>`, `pub fn unwrap(pin: &str, w: &PinWrap, vault_id: &str) -> Result<SecretKey, ApiError>` (неверный PIN → `code: "WrongPin"`), `pub fn validate_pin(pin: &str) -> Result<(), ApiError>` (6–12 цифр, иначе `Validation`). `PinWrap { salt: [u8;16] (base64), m_kib: u32, t: u32, p: u32, envelope: Vec<u8> (base64) }`.
  - `pub struct Unlocker<'a> { keys: &'a dyn KeyStore, auth: &'a dyn Authenticator }` с методами:
    `methods(vault_id) -> UnlockMethods { biometric: bool, pin: bool, attempts_left: u8 }`,
    `enable_biometric(vault: &Vault) -> Result<()>` (сначала `authenticate`, потом `set("data-key:<id>")`), `disable_biometric(vault_id)`,
    `unlock_biometric(vault_dir, vault_id) -> Result<Vault>` (`authenticate` → `get` → `Vault::open_with_key`; ключа нет → `code: "BiometricNotSet"`; отмена → `code: "AuthCancelled"`),
    `set_pin(vault: &Vault, pin) -> Result<()>`, `remove_pin(vault_id)`,
    `unlock_pin(vault_dir, vault_id, pin) -> Result<Vault>` (неверный → счётчик+1, `WrongPin` с `attempts_left` в message; на 5-й ошибке удалить `pin:` и счётчик → `PinLocked`; успех → счётчик сброшен),
    `confirm_presence(vault_id, pin: Option<&str>) -> Result<()>` (для показа сид-фразы: PIN, если передан, иначе `authenticate`).
  - Команды: `unlock_methods`, `enable_biometric`, `disable_biometric`, `unlock_biometric`, `set_pin`, `remove_pin`, `unlock_pin`, `reveal_seed(pin: Option<String>) -> Result<Vec<String>, ApiError>`. Любой успешный вход (сид, Touch ID, PIN) сбрасывает `pin-attempts`.

- [ ] **Step 1: Падающие тесты**

```rust
#[test] fn pin_wrap_roundtrip_and_wrong_pin() { /* wrap("482913") → unwrap("482913") == key; unwrap("000000") → code "WrongPin" */ }
#[test] fn pin_validation() { /* "12345" и "12ab56" → Validation; "123456" и "123456789012" → Ok; "1234567890123" → Validation */ }
#[test] fn five_wrong_pins_lock_out_even_correct_one() { /* 5× неверный → 5-й даёт PinLocked; верный → PinLocked; methods().pin == false */ }
#[test] fn successful_unlock_resets_attempts() { /* 3× неверный, верный → Ok; methods().attempts_left == 5 */ }
#[test] fn biometric_cancel_and_unavailable() { /* FakeAuthenticator(Cancelled) → AuthCancelled; (Unavailable) → code "AuthUnavailable"; хранилище не открыто */ }
#[test] fn biometric_without_enrolled_key() { /* code "BiometricNotSet" */ }
#[test] fn enable_biometric_requires_auth_first() { /* Failed → ключ в KeyStore не записан */ }
#[test] fn reveal_seed_requires_fresh_auth() { /* FakeAuthenticator(Cancelled) → Err; Ok → 24 слова */ }
```

Тестовые Argon2-параметры: `cfg(test)` → m = 1 MiB, t = 1, чтобы тесты шли быстро; продакшн-значения — из Global Constraints.

- [ ] **Step 2: Run** `cargo test -p crypt-desktop unlock pin` → FAIL.
- [ ] **Step 3: Реализовать** логику, `MacKeychain`, `MacAuthenticator`, команды.
- [ ] **Step 4: Run** `cargo test -p crypt-desktop` → PASS; `cargo clippy -p crypt-desktop -- -D warnings` → чисто.

---

### Task 4: Онбординг и экран разблокировки (Vue)

**Files:** Create: `src/lib/{types,api}.ts`, `src/stores/session.ts`, `src/router.ts`, `src/views/{Onboarding,Unlock}.vue`, `src/tests/api.test.ts`

**Interfaces:**
- Consumes: команды Task 2–3.
- Produces:
  - `lib/api.ts`: `class CryptApiError extends Error { code: string }`; `call<T>(cmd: string, args?: object): Promise<T>` переводит отказ invoke в `CryptApiError`; именованные обёртки для каждой команды (`api.status()`, `api.createVault(words)`, … — camelCase от snake_case команд).
  - `stores/session.ts` (Pinia `useSession`): `status: VaultStatus`, `methods: UnlockMethods | null`, действия `refresh()`, `create(words)`, `unlockSeed(words)`, `unlockBiometric()`, `unlockPin(pin)`, `lock()`, `beforeLock(fn: () => Promise<void>)` — регистрирует функцию, которую `lock()` дожидается до вызова команды (для сброса отложенных сохранений).
  - Роутер-гард: `NoVault` → `/onboarding`; `Locked` → `/unlock`; `Unlocked` и путь онбординга/разблокировки → `/`.
  - Онбординг (как в макете «Сид-фраза»): 4 шага — «Хранилище» (кнопки «Создать новое» / «Восстановить по сид-фразе»), «Сид-фраза» (24 слова сеткой 4×6, моноширинный шрифт, предупреждение, «Распечатать» через `window.print()`), «Проверка» (ввести 3 случайных слова по номерам), «Touch ID и PIN» (переключатель Touch ID, поле PIN, «Пропустить»). Восстановление при отсутствии хранилища: textarea для 24 слов → `create_vault` с введёнными словами (новое пустое хранилище с теми же ключами; данные вернутся после синхронизации в будущих фазах).
  - Разблокировка (как в макете): большая кнопка Touch ID (если `methods.biometric`), 6 полей PIN (если `methods.pin`, автоподтверждение по 6-й цифре, плюс Enter для более длинных), «Осталось попыток: N», ссылка «Восстановить по сид-фразе» (24 поля или textarea). При открытии экрана Touch ID запускается автоматически один раз.

- [ ] **Step 1: Падающий тест `api.test.ts`** (мок `@tauri-apps/api/core`): отказ `{code:"WrongPin",message:"…"}` → `CryptApiError` с `code === "WrongPin"`; строковый отказ → `code === "Unknown"`.
- [ ] **Step 2: Run** `pnpm vitest run` → FAIL. **Step 3:** реализовать. **Step 4:** `pnpm vitest run` → PASS; `pnpm build` → без ошибок `vue-tsc`.
- [ ] **Step 5: Ручная проверка** `pnpm tauri dev`: создать хранилище, пройти проверку слов, задать PIN 6 цифр → главная; ⌘L (временно кнопкой) → экран разблокировки → PIN открывает. Пять неверных PIN → поле PIN исчезает, остаётся Touch ID / сид-фраза.

---

### Task 5: Команды библиотеки (Rust)

**Files:** Create: `src-tauri/src/commands/library.rs` (+ тесты)

**Interfaces:**
- Consumes: методы `Vault` из плана 1.
- Produces: хелпер `with_vault<T>(state, f: impl FnOnce(&mut Vault) -> crypt_core::Result<T>) -> Result<T, ApiError>` (заблокировано → `code: "Locked"`). Команды (аргументы — camelCase во фронтенде):
  `list_types`, `put_type(t: TypeDef)`, `delete_type(id)`, `starter_types() -> Vec<TypeDef>`,
  `list_folders`, `create_folder(name, parent)`, `rename_folder(id, name)`, `move_folder(id, parent)`, `delete_folder(id)`,
  `list_views(type_id)`, `put_view(v)`, `delete_view(id)`,
  `list_records(filter: ListFilterDto) -> Vec<RecordSummary>`, `get_record(id) -> RecordView`, `get_records(ids) -> Vec<RecordView>` (для таблицы; отсутствующие пропускаются),
  `create_record(new: NewRecord) -> RecordId`, `set_title`, `set_body`, `set_type`, `set_folder`, `add_tag`, `remove_tag`, `set_field(id, field, value: Option<FieldValue>)`,
  `delete_record(id)`, `restore_record(id)`, `purge_record(id)`, `empty_trash() -> usize`,
  `search(query, limit) -> Vec<SearchHit>`, `backlinks(id) -> Vec<RecordSummary>`, `all_tags() -> Vec<(String, usize)>` (по всем незаудалённым записям, по убыванию частоты).
  `ListFilterDto` = поля `ListFilter` (serde, camelCase); если в ядре у `ListFilter` нет `Deserialize`, добавить его там.
  `create_record` для записи с `secret_kind` в этом плане не вызывается фронтендом (план 3), но команда не запрещает.

- [ ] **Step 1: Падающие тесты** (через `with_vault` на `AppState` с tempdir):
```rust
#[test] fn locked_vault_returns_locked_code() { /* lock → list_types → code "Locked" */ }
#[test] fn record_crud_through_commands() { /* create → set_title/body/tag → get_record отражает всё; delete → list_records(trashed) содержит; restore */ }
#[test] fn all_tags_counts_and_excludes_trash() { /* теги a(2), b(1); запись с b в корзину → [("a",2)] */ }
#[test] fn get_records_skips_missing() { /* [существующий, случайный] → 1 элемент */ }
```
- [ ] **Step 2: Run** → FAIL. **Step 3:** реализовать (`all_tags` — в ядре как `Vault::all_tags()`, с тестом там же). **Step 4:** `cargo test --workspace` → PASS.

---

### Task 6: Оболочка приложения — сайдбар, навигация, поиск, горячие клавиши

**Files:** Create: `src/stores/library.ts`, `src/components/{AppShell,AppSidebar,CommandPalette}.vue`, `src/views/Home.vue`; Modify: `router.ts`, `App.vue`

**Interfaces:**
- Consumes: команды Task 5, `useSession`.
- Produces:
  - `useLibrary`: `types`, `folders`, `tags`, `counts` (по типам и папкам, из `list_records`), `refreshAll()`, `createRecord(opts) → id` (навигация на `/r/:id`), событие-шина `changed(recordId)` для обновления списков.
  - Маршруты: `/` (Home — недавние 20 записей), `/r/:id`, `/t/:typeId` (таблица), `/types/:typeId?`, `/folder/:folderId`, `/tag/:tag`, `/trash`, `/settings`.
  - Сайдбар как в макете: строка с отступом под светофор (52 px, `data-tauri-drag-region`), кнопка блокировки, название «Личное» + число записей, кнопка «+» (новая запись), кнопка поиска «⌘K». Группы: **Типы** (иконка, имя, счётчик; последним «Новый тип…»), **Хранилище → Секреты** (ведёт на заглушку «Появится в следующей версии», акцентный цвет), **Папки** (дерево, раскрытие, контекстное меню: «Новая подпапка», «Переименовать», «Удалить»; drag-and-drop не нужен — «Переместить в…» через меню), **Теги** (топ-10 + «все»). Низ: «Корзина · N», «Настройки».
  - `CommandPalette` (shadcn Command в Dialog): поиск через `search` (debounce 150 мс, limit 20), показ заголовка, типа и сниппета; Enter открывает. Пустой запрос — недавние записи. Пункты-команды: «Новая запись», «Новый тип», «Заблокировать».
  - Глобальные клавиши в `AppShell`: ⌘K, ⌘N, ⌘L (через `keydown` на `window`, только когда разблокировано).
- [ ] **Step 1:** Реализовать. **Step 2:** `pnpm build` → без ошибок. **Step 3: Ручная проверка** в `pnpm tauri dev`: создать папку и подпапку, переименовать, удалить пустую; ⌘N создаёт запись и открывает её; ⌘K находит её по заголовку на кириллице; ⌘L блокирует.

---

### Task 7: Карточка записи — заголовок, теги, поля, бэклинки

**Files:** Create: `src/lib/{fields,debounce}.ts`, `src/components/record/*.vue`, `src/views/Record.vue`, `src/tests/{fields,debounce}.test.ts`

**Interfaces:**
- Consumes: `get_record`, `set_*`, `backlinks`, `list_records({secrets:true})`, `list_records({typeId})`.
- Produces:
  - `lib/debounce.ts`: `debouncedSaver<T>(save: (v: T) => Promise<void>, ms: number): { push(v: T): void; flush(): Promise<void>; pending(): boolean }`.
  - `lib/fields.ts`: `formatValue(v: FieldValue | undefined, kind: FieldKind, ctx: { titleOf(id): string | undefined }): string`; `parseInput(raw: string, kind: FieldKind): FieldValue | null | { error: string }` — Number принимает `399`, `399,5`, `1 690` (пробелы и запятая), Date — `YYYY-MM-DD` из `<input type=date>`; Url без схемы дополняется `https://`; пустая строка → `null` (очистить поле).
  - Экран (как в макете «Карточка записи»): хлебные крошки (папка), справа «Сохранено · HH:MM» / «Сохраняется…», кнопка «⋯» (Переместить в папку, Сменить тип, Удалить). Бейдж типа и теги (TagEditor: добавить по Enter, удалить по ×, автодополнение из `all_tags`). Заголовок — большой `input` без рамки, сохраняется по blur/Enter. FieldsPanel — сетка «название | значение», редактор по виду поля (Text/Url/Number — input; Date — input type=date; Checkbox — Switch; Select/MultiSelect — Select / Popover с чекбоксами и цветными бейджами; RecordLink — Popover с поиском по записям (тип ограничен `target_type`), чипы со ссылками; SecretLink — Popover со списком секретов, чип «✱ имя» акцентным цветом). Скрытые значения (`hidden_fields`) — свёрнутый блок «Значения удалённых полей · N». Ниже — MarkdownEditor (Task 8). Справа панель 300 px: «Бэклинки · N» (заголовок, тип) и «Информация» (создано, изменено).
  - Запись без типа показывает «+ Назначить тип».
- [ ] **Step 1: Падающие тесты**
```ts
test('parseInput number accepts ru formats', () => { expect(parseInput('1 690', num)).toEqual({kind:'Number',value:1690}); expect(parseInput('399,5', num)).toEqual({kind:'Number',value:399.5}); expect(parseInput('abc', num)).toHaveProperty('error') })
test('parseInput empty clears', () => { expect(parseInput('  ', text)).toBeNull() })
test('url gets scheme', () => { expect(parseInput('360.yandex.ru', url)).toEqual({kind:'Url',value:'https://360.yandex.ru'}) })
test('formatValue links use current titles and mark missing', () => { /* titleOf(a)='Аня', titleOf(b)=undefined → 'Аня, (удалено)' */ })
test('debouncedSaver coalesces and flush saves last value', async () => { /* push(1), push(2) → save вызван 0 раз до 600мс; flush() → save(2) один раз; pending() false */ })
```
- [ ] **Step 2: Run** `pnpm vitest run` → FAIL. **Step 3:** реализовать. **Step 4:** `pnpm vitest run && pnpm build` → PASS.
- [ ] **Step 5: Ручная проверка:** поставить starter-тип «Сервер», создать запись, заполнить IP, порт `2 222`, дату, теги `#Prod` → после перезапуска приложения значения на месте, тег `prod`.

---

### Task 8: Markdown-редактор с live preview и wiki-ссылками

**Files:** Create: `src/lib/wikilinks.ts`, `src/components/editor/{MarkdownEditor.vue,livePreview.ts,wikilinkExtension.ts}`, `src/tests/wikilinks.test.ts`; Modify: `views/Record.vue`

**Interfaces:**
- Consumes: `set_body` (через `debouncedSaver`, 600 мс), `search`, `get_record`/кэш заголовков `useLibrary`.
- Produces:
  - `lib/wikilinks.ts`: `findLinks(text: string): { from: number; to: number; id: string; alias?: string }[]` (тот же regex, что в ядре: `\[\[([0-7][0-9A-HJKMNP-TV-Z]{25})(?:\|([^\]\n]*))?\]\]`), `makeLink(id: string, title: string): string` → `[[id|title]]` (символы `]` и `|` в title заменяются на пробел).
  - `MarkdownEditor.vue`: props `modelValue: string`, `recordId: string`; emits `update:modelValue`; `expose({ flush })`. CodeMirror 6: `markdown()` + live preview в стиле Obsidian — на строках без курсора скрываются маркеры `#`, `**`, `*`, `` ` ``, `~~`, `>` и показывается оформление (заголовки H1–H3 размером, жирный, курсив, зачёркивание, инлайн-код Geist Mono, цитата с левой полосой, списки, `- [ ]` как чекбокс, кликабельный и переключающий `[ ]`/`[x]`); на строке с курсором видна исходная разметка. Блоки кода — моноширинный фон без подсветки синтаксиса.
  - Wiki-ссылки: на строках без курсора `[[id|alias]]` заменяется виджетом-ссылкой с **текущим** заголовком записи (из кэша, если есть, иначе alias); если записи нет — зачёркнутый alias серым, без перехода. Клик (⌘+клик на строке с курсором) → `/r/:id`. Автодополнение: набор `[[` открывает список из `search(query, 10)` (пустой запрос → недавние); выбор вставляет `makeLink(id, title)`; последний пункт «Создать «<запрос>»» создаёт запись и вставляет ссылку.
  - `Record.vue`: при уходе со страницы (`onBeforeRouteLeave`, `onBeforeUnmount`) и в `session.beforeLock` вызывается `flush()`.
- [ ] **Step 1: Падающие тесты**
```ts
test('findLinks parses id and alias', () => { /* два линка + мусорный [[не-id]] → 2 */ })
test('makeLink sanitizes title', () => { expect(makeLink(ID, 'a]|b')).toBe(`[[${ID}|a  b]]`) })
test('missing link target renders as broken', () => { /* хелпер linkLabel(id, alias, titleOf) → {text: alias, broken: true} */ })
test('flush before leave saves the last keystrokes', async () => { /* debouncedSaver с fake timers: push('abc') и сразу flush() → save('abc') */ })
```
- [ ] **Step 2:** `pnpm vitest run` → FAIL. **Step 3:** реализовать. **Step 4:** `pnpm vitest run && pnpm build` → PASS.
- [ ] **Step 5: Ручная проверка:** набрать заголовок, список, чекбокс, ссылку на другую запись через `[[`; переименовать целевую запись → ссылка показывает новое имя; удалить её в корзину → ссылка зачёркнута; набрать текст и сразу ⌘L → после разблокировки текст на месте.

---

### Task 9: Конструктор типов

**Files:** Create: `src/components/types/*.vue`, `src/views/Types.vue`

**Interfaces:**
- Consumes: `list_types`, `put_type`, `delete_type`, `starter_types`, `list_records({typeId})`.
- Produces (как в макете «Конструктор типов»): слева список типов (иконка, имя, счётчик) и «+ Тип» (пустой тип «Новый тип»). Справа: кнопка-иконка (выбор из ~24 иконок lucide: file-text, server, book, user, globe, credit-card, chef-hat, folder, star, heart, home, briefcase, graduation-cap, car, film, music, gamepad-2, plane, shopping-cart, wrench, code, database, key-round, link), название типа (input без рамки), ссылка «Открыть таблицу · N». Список полей: перетаскивание для порядка (HTML5 drag-and-drop по ручке ⋮⋮), имя (inline-input), вид, доп. настройки по виду (Select/MultiSelect — редактор вариантов: метка + цвет из 8; RecordLink — целевой тип и «несколько значений»; SecretLink — «несколько значений»), удаление поля с подтверждением «Значения сохранятся скрытыми». «+ Добавить поле» открывает FieldKindMenu (9 видов как в макете, «Секрет» — акцентным цветом). Справа превью карточки с примерными значениями. Сохранение — по каждому изменению с debounce 400 мс через `put_type`. «Удалить тип» доступен, только если записей 0 (иначе отключён с подсказкой).
  - Пустая библиотека: в Home и в «Типы» — кнопка «Добавить стартовый тип «Сервер»».
- [ ] **Step 1:** Реализовать. **Step 2:** `pnpm build` → без ошибок. **Step 3: Ручная проверка:** создать тип «Книга» из макета (Автор → ссылка на «Человек», Статус — выбор из 3, Оценка, Дата, Ссылка, Где читаю → секрет), переставить поля, удалить «Ссылка» → в записи значение ушло в «Значения удалённых полей».

---

### Task 10: Таблица типа с видами

**Files:** Create: `src/lib/table.ts`, `src/components/table/*.vue`, `src/views/TypeTableView.vue`, `src/tests/table.test.ts`

**Interfaces:**
- Consumes: `list_records({typeId})`, `get_records(ids)`, `set_field`, `set_title`, `create_record`, `list_views`/`put_view`/`delete_view`, `fields.ts`.
- Produces:
  - `lib/table.ts`:
    ```ts
    type ViewConfig = { columns: string[]; hidden: string[]; sort: { col: string; dir: 'asc'|'desc' }[]; filters: Filter[] }
    type Filter = { col: string; op: 'eq'|'contains'|'empty'|'notEmpty'; value?: string }
    // col: 'title' | 'tags' | 'folder' | <FieldId>
    function sanitizeConfig(cfg: unknown, t: TypeDef): ViewConfig   // отбрасывает неизвестные колонки и фильтры; некорректный JSON → defaultConfig
    function defaultConfig(t: TypeDef): ViewConfig                  // title + все поля по порядку, без сортировки
    function applyView(rows: RecordView[], cfg: ViewConfig, t: TypeDef, titleOf: (id: string) => string | undefined): RecordView[]
    ```
    Сравнение: числа — численно, даты — строкой ISO, текст — `localeCompare(…, 'ru')`; пустые значения всегда в конце; `contains` без учёта регистра; для Select — по метке варианта.
  - Экран (как в макете «Таблица типа»): заголовок с иконкой, «N записей», кнопки «Поля типа» (→ `/types/:id`) и «Новая запись» (создаёт с типом, фокус на ячейке названия). Вкладки видов: «Все» (конфиг не сохраняется, только в памяти) + сохранённые виды + «+» (создать из текущего). Панель: чипы фильтров (добавить через Popover: колонка → оператор → значение), сортировка (клик по заголовку колонки: asc → desc → нет; Shift+клик добавляет вторичную), поиск по таблице (фильтр по заголовку на клиенте). Изменения сохранённого вида — автосохранение `put_view` (debounce 400 мс). Ячейки: клик по названию → открыть запись; двойной клик (или Enter) по ячейке поля → редактор поля (тот же `FieldEditor`), Esc отменяет, Enter/blur сохраняет через `set_field`. Колонка «Доступ» (SecretLink) — чипы с ✱. Последняя строка «+ Новая строка». Горизонтальная прокрутка при ширине больше окна.
- [ ] **Step 1: Падающие тесты**
```ts
test('sanitizeConfig drops unknown columns and filters', () => { /* cfg с колонкой удалённого поля и фильтром по ней → обе убраны */ })
test('sanitizeConfig on garbage returns default', () => { expect(sanitizeConfig('x', t)).toEqual(defaultConfig(t)) })
test('sort numbers numerically, empties last', () => { /* 399, 1690, пусто, 12.4 → 12.4, 399, 1690, пусто */ })
test('sort text ru locale', () => { /* «Яндекс», «Авито», «ёлка» → Авито, ёлка, Яндекс */ })
test('filter contains is case-insensitive; select compares by label', () => { /* … */ })
test('kind change makes old values hidden, not crash', () => { /* RecordView с полем в hidden_fields → applyView не падает, ячейка пустая */ })
```
- [ ] **Step 2:** `pnpm vitest run` → FAIL. **Step 3:** реализовать. **Step 4:** `pnpm vitest run && pnpm build` → PASS.
- [ ] **Step 5: Ручная проверка:** тип «Сервис» из макета, 5 записей, вид «Платные» (фильтр Стоимость не пусто, сортировка по продлению) → перезапуск → вид на месте; удалить поле «Стоимость» из типа → вид открывается без этой колонки и фильтра.

---

### Task 11: Корзина, папки и теги как списки, настройки

**Files:** Create: `src/views/{Trash,Settings}.vue`, компонент списка записей `src/components/RecordList.vue`; Modify: `router.ts`

**Interfaces:**
- Consumes: `list_records`, `restore_record`, `purge_record`, `empty_trash`, `unlock_methods`, `enable/disable_biometric`, `set_pin`/`remove_pin`, `reveal_seed`, настройки (`get_settings`/`set_settings` — добавить команды в `lifecycle.rs` с тестом `settings_default_and_roundtrip` из Task 2).
- Produces:
  - `RecordList`: строки (иконка типа, заголовок, теги, дата изменения), используется для `/folder/:id`, `/tag/:tag`, Home.
  - Корзина: список с «Восстановить» и «Удалить навсегда» (подтверждение), «Очистить корзину (N)».
  - Настройки: «Автоблокировка» (Select 1/5/15/30/60 мин), «Touch ID» (Switch → enable/disable), «PIN-код» (задать / сменить / удалить), «Сид-фраза» → «Показать» (подтверждение Touch ID или PIN → 24 слова на 60 секунд, потом скрываются), «Расположение хранилища» (путь, кнопка «Показать в Finder» через `tauri-plugin-opener` `revealItemInDir`).
- [ ] **Step 1:** Реализовать. **Step 2:** `cargo test --workspace && pnpm vitest run && pnpm build` → PASS. **Step 3: Ручная проверка:** удалить запись → в корзине → восстановить; «Показать сид-фразу» без подтверждения не открывается; смена автоблокировки сохраняется после перезапуска.

---

### Task 12: Автоблокировка, сборка и README

**Files:** Create: `src-tauri/src/autolock.rs`, `README.md`; Modify: `src-tauri/src/lib.rs`, `src/stores/session.ts`, `tauri.conf.json`

**Interfaces:**
- Produces:
  - Бездействие: фронтенд (`useSession`) отслеживает `pointerdown`, `keydown`, `wheel`; таймер на `settings.autolock_minutes`; по истечении — `lock()` (с `beforeLock`).
  - Сон и блокировка экрана: `autolock.rs` подписывается на `NSWorkspaceWillSleepNotification` (через `NSWorkspace.sharedWorkspace.notificationCenter`) и на распределённое уведомление `com.apple.screenIsLocked` (`NSDistributedNotificationCenter`); по событию Rust шлёт `vault-will-lock`, фронтенд сбрасывает отложенные сохранения и вызывает команду `flush_done`; Rust ждёт её до 500 мс, затем блокирует (`lock(&AppState)`) и шлёт `vault-locked`, фронтенд уходит на `/unlock`.
  - Иконка приложения (замок + «C», тёмная) в `src-tauri/icons` через `pnpm tauri icon`.
  - `README.md`: требования (Rust 1.94, Node 26, pnpm 11, Xcode), `pnpm install`, `pnpm tauri dev`, тесты; раздел «Подпись без платного аккаунта»: создать в «Связке ключей» → «Ассистент сертификации» → «Создать сертификат» самоподписанный сертификат типа «Подписание кода» с именем `Crypt Local Signing`, указать его в `tauri.conf.json` → `bundle.macOS.signingIdentity`, `pnpm tauri build`; почему в dev-режиме macOS может спрашивать доступ к связке ключей после пересборки.
- [ ] **Step 1: Падающий тест** (Rust, без AppKit): `lock_sequence_waits_for_flush_with_timeout` — функция `graceful_lock(state, notify: impl Fn(), wait: Receiver<()>, timeout)`: при ответе — lock сразу после ответа; без ответа — lock через `timeout`. Run → FAIL.
- [ ] **Step 2:** Реализовать `graceful_lock`, наблюдатели, фронтенд-часть. Run `cargo test --workspace` → PASS.
- [ ] **Step 3: Сборка.** Run: `cd apps/desktop && pnpm tauri build --debug`. Expected: собирается `Crypt.app`. Запустить его; проверить: Touch ID включается и открывает хранилище; заблокировать экран Mac (⌃⌘Q) → после возврата приложение на экране разблокировки; набранный перед блокировкой текст сохранён.
- [ ] **Step 4: Финальная проверка:** `cargo test --workspace && cargo clippy --workspace -- -D warnings && cd apps/desktop && pnpm vitest run && pnpm build` → всё зелёное.
