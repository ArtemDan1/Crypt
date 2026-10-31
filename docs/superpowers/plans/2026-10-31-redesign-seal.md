# Редизайн «Печать» — план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Перевести десктоп‑приложение Crypt на тему «Печать», значок «Семя», переработанные экраны, анимации, всплывающее окно трея и полноценный генератор паролей.

**Architecture:** Токены темы меняются в `styles.css` (имена shadcn сохраняются, добавляются недостающие), общие визуальные примитивы выносятся в `src/components/crypt/`, экраны переписываются по макетам поверх существующих сторов и `api`. Генератор расширяется в Rust (`crates/crypt-core/src/gen.rs`), трей становится отдельным окном Tauri с маршрутом `/tray` в том же фронтенде.

**Tech Stack:** Vue 3 + TypeScript, Tailwind 4, shadcn‑vue / reka‑ui, lucide, vitest (jsdom), Rust + Tauri 2.

**Spec:** `docs/superpowers/specs/2026-10-31-redesign-seal-design.md`
**Макеты:** https://claude.ai/artifact/PCSwUujPE9RMxKYTJeKVua (страница «Печать · экраны»). Локальные копии `.dc.html` можно получить через Artifact `read` с `path: "project/<Name>.dc.html"`; цвета, размеры и тексты брать оттуда.

## Global Constraints

- Цвета — только из таблицы токенов спецификации; hex там — источник истины.
- Изумрудный (`primary`) — только основные действия, активное состояние, фокус, включённые переключатели. Выделение строк — нейтральное (`accent`).
- Янтарь (`secret*`) — только секреты и предупреждение о сид‑фразе.
- Анимации 150–450 мс; появление `cubic-bezier(.2,.8,.2,1)`, пружина `cubic-bezier(.3,1.3,.5,1)`; петли допустимы только для сканирования Touch ID и TOTP; `prefers-reduced-motion: reduce` оставляет только смену прозрачности.
- Шрифт Geist / Geist Mono; без новых тяжёлых зависимостей.
- Ни одно открытое значение секрета не запрашивается без явного действия пользователя.
- Rust‑ядро и модель данных не меняются, кроме `gen.rs` (генератор).
- `pnpm test`, `pnpm build`, `pnpm format:check` (в `apps/desktop`) и `cargo test` (в корне) зелёные после каждой задачи.
- Режим `pnpm dev` с фейковым бэкендом (`src/dev/mockBackend.ts`) работает; новые команды получают в нём заглушки.
- Тексты интерфейса — русские, как в макетах.

## Review Focus

1. Системная тема меняется, пока приложение открыто, при выборе «Система» — интерфейс переключается сразу, а при явном «Светлая/Тёмная» системная смена игнорируется.
2. Автоблокировка «Никогда» (или 0 минут) — кольцо в сайдбаре не рисуется и не делит на ноль.
3. Окно трея при заблокированном хранилище не показывает список и не держит старые данные после блокировки из главного окна.
4. Генератор: все наборы символов выключены или пул пуст после «исключить символы» — понятное сообщение вместо паники/пустой строки.
5. Карта без `expiry`/`holder` в `public` — список и карточка не ломаются, показывают только то, что есть.

---

## Структура файлов

| Файл | Ответственность |
|---|---|
| `src/styles.css` | токены, утилиты анимаций, reduced‑motion |
| `src/lib/theme.ts` (новый) | предпочтение темы «system/light/dark», применение к `<html>` |
| `src/lib/strength.ts` (новый) | оценка надёжности пароля (эвристика) |
| `src-tauri/src/recent.rs` (новый) | недавно скопированные секреты, только в памяти |
| `src/lib/idle.ts` | + `remaining()` |
| `src/components/crypt/CryptMark.vue` | значок «Семя» (inline SVG, размеры) |
| `src/components/crypt/SegmentedControl.vue` | сегменты со скользящим индикатором |
| `src/components/crypt/KindTile.vue` | плитка‑иконка вида (тип/секрет/файл) |
| `src/components/crypt/SecretValuePill.vue` | янтарная плашка значения (маска ↔ раскрытие) |
| `src/components/crypt/AutolockRing.vue` | кольцо остатка автоблокировки |
| `crates/crypt-core/src/gen.rs` + `crates/crypt-core/src/eff_wordlist.txt` | генератор символов и фраз, энтропия |
| `src/views/Generator.vue` (новый) | экран генератора |
| `src/views/Tray.vue` (новый) | содержимое окна трея |
| `src-tauri/src/tray.rs` | окно трея вместо меню |

---

### Task 1: Токены темы и выбор темы

**Files:**
- Modify: `src/styles.css` (блоки `:root`, `.dark`, `@theme inline`)
- Create: `src/lib/theme.ts`
- Modify: `src/main.ts:12-16`, `src/App.vue` (Toaster)
- Test: `src/tests/theme.test.ts`

**Interfaces:**
- Produces: `type ThemePref = 'system' | 'light' | 'dark'`; `getThemePref(): ThemePref`; `setThemePref(p: ThemePref): void`; `initTheme(): () => void` (вешает слушатель `matchMedia`, возвращает отписку); `resolvedTheme(): 'light' | 'dark'`. Ключ localStorage: `crypt.theme`.
- Produces (CSS): новые переменные `--secret-bg`, `--secret-fg`, `--primary-soft`, `--primary-soft-fg`, `--foreground-2`, `--border-strong` и утилиты Tailwind `bg-secret-bg`, `text-secret-fg`, `bg-primary-soft`, `text-primary-soft-fg`, `text-foreground-2`, `border-border-strong` через `@theme inline`.

- [ ] **Step 1: Тесты `theme.test.ts`**
  - `initTheme()` при `crypt.theme` отсутствующем и `matchMedia('(prefers-color-scheme: dark)')` = true ставит класс `dark` на `document.documentElement`.
  - `setThemePref('light')` снимает `dark`, сохраняет `'light'`; последующее изменение системной темы класс не меняет.
  - `setThemePref('system')` снова следует `matchMedia`.
  - Мусор в localStorage → `getThemePref() === 'system'`.
  (Мок `window.matchMedia` с ручным вызовом слушателей.)
- [ ] **Step 2:** `pnpm vitest run src/tests/theme.test.ts` → FAIL (модуля нет).
- [ ] **Step 3:** Реализовать `src/lib/theme.ts`; в `main.ts` заменить текущий блок темы на `initTheme()`; в `App.vue` `Toaster` получает `:theme="resolved"` (реактивно от смены темы — `theme.ts` диспатчит `window` событие `crypt-theme`).
- [ ] **Step 4:** Заменить значения токенов в `styles.css` на таблицу спецификации (светлая в `:root`, тёмная в `.dark`; `--primary`/`--ring`/`--sidebar-primary` — изумрудный; `--accent` — нейтральное выделение; `--sidebar-accent` светлой `#E6E4DE`; `--secret` = иконка секрета; `--radius: 0.625rem`). Добавить новые переменные и их `--color-*` в `@theme inline`. На `html` добавить `transition: background-color .35s, color .35s` и класс‑утилиту `.theme-fade` не нужна — переход на `body` и `bg-*` элементах через `@layer base`.
- [ ] **Step 5:** `pnpm vitest run` и `pnpm build` → PASS. `pnpm dev` → в браузере светлая и тёмная тема выглядят как `Directory.dc.html` (цвета фона, сайдбара, рамок).
- [ ] **Step 6: Commit** `feat(ui): тема «Печать» и выбор темы`

### Task 2: Значок «Семя» и иконки приложения

**Files:**
- Create: `src/components/crypt/CryptMark.vue`
- Modify: `assets/icon.svg`; Create: `assets/icon-small.svg` (вариант 32 px), `assets/tray.svg` (шаблон 24×24)
- Regenerate: `src-tauri/icons/*`, `src-tauri/icons/tray-template.png`, `tray-template@2x.png`

**Interfaces:**
- Produces: `<CryptMark :size="number" :animated?="boolean" />` — плитка `#1F1E1B`, семя `#12855F`, жилки `#0A3B2C` (пути — `Icons.dc.html`, вариант D). При `size < 40` рисует упрощённый вариант (только центральная жилка). `animated` включает «прорастание» (scale от низа, 650 мс) один раз при монтировании.

- [ ] **Step 1:** `CryptMark.vue` по SVG из макета; `role="img"` + `aria-label="Crypt"`, при декоративном использовании — проп `decorative` ставит `aria-hidden`.
- [ ] **Step 2:** `assets/icon.svg` — большой вариант на холсте 1024; `pnpm tauri icon assets/icon.svg` перегенерирует набор. Проверить `src-tauri/icons/32x32.png` глазами: если силуэт не читается, пересобрать 32/64 из `icon-small.svg` (`pnpm tauri icon assets/icon-small.svg -o /tmp/small -p 32,64` и скопировать).
- [ ] **Step 3:** Трей: `assets/tray.svg` — чёрный контур семени и жилка на прозрачном фоне (пути из блока «Меню‑бар» макета, `stroke-width 1.8`); `pnpm tauri icon assets/tray.svg -o /tmp/tray -p 22,44` → скопировать как `tray-template.png` и `tray-template@2x.png`.
- [ ] **Step 4:** `pnpm build` → PASS; `cargo build -p crypt-desktop` (или имя крейта из `src-tauri/Cargo.toml`) → PASS.
- [ ] **Step 5: Commit** `feat(brand): значок «Семя»`

### Task 3: Общие примитивы и движение

**Files:**
- Modify: `src/styles.css` (утилиты анимаций)
- Create: `src/components/crypt/SegmentedControl.vue`, `KindTile.vue`, `SecretValuePill.vue`
- Modify: `src/components/ui/button/*` (нажатие `active:scale-[.97]`), `src/components/ui/switch/*` (пружинящий бегунок), `src/components/ui/popover/*`, `dropdown-menu/*`, `dialog/*` (появление `scale .97 → 1`)

**Interfaces:**
- Produces CSS‑классы: `.anim-in` (подъём 8 px + проявление, 400 мс), `.anim-pop`, `.anim-shake`, `.anim-reveal` (из размытия), `.anim-tick` (пружина 0→1); все отключаются в `@media (prefers-reduced-motion: reduce)` (остаётся только `opacity`).
- Produces: `<SegmentedControl v-model="string" :options="{ value: string; label: string }[]" :aria-label="string" size?="sm|md" />` — индикатор позиционируется по `offsetLeft/offsetWidth` активной кнопки; кнопки — `button` с `aria-pressed`.
- Produces: `<KindTile kind="note|server|book|card|secret|folder|..." :icon?="Component" :tone?="'neutral'|'primary'|'secret'|'sand'|'slate'" size?="sm|md|lg" />`.
- Produces: `<SecretValuePill :value="string | null" :mono?="boolean" />` — `null` → маска `••••••••••••`, строка → значение с `.anim-reveal`; фон `bg-secret-bg`, текст `text-secret-fg`.

- [ ] **Step 1:** Утилиты в `styles.css` + блок reduced‑motion.
- [ ] **Step 2:** Тест `src/tests/segmented.test.ts` (`@vue/test-utils` уже доступен через vitest? — если нет в `devDependencies`, тестировать через `createApp` + jsdom): клик по второй опции эмитит `update:modelValue` со значением второй опции; активная кнопка имеет `aria-pressed="true"`.
- [ ] **Step 3:** Реализовать три компонента; доработать shadcn‑примитивы.
- [ ] **Step 4:** `pnpm vitest run` + `pnpm build` → PASS.
- [ ] **Step 5: Commit** `feat(ui): примитивы и анимации`

### Task 4: Сайдбар и кольцо автоблокировки

**Files:**
- Modify: `src/lib/idle.ts`, `src/stores/session.ts`, `src/components/AppSidebar.vue`, `src/components/FolderTree.vue`
- Create: `src/components/crypt/AutolockRing.vue`
- Test: `src/tests/idle.test.ts`

**Interfaces:**
- Produces: `idleTimer(...).remaining(): number | null` — мс до срабатывания; `null`, если таймер остановлен. `useSession().autolockLeft: Ref<number | null>` (мс) и `autolockTotal: Ref<number | null>` (мс; `null` при «Никогда»/0), обновляются раз в секунду, пока открыто.
- Consumes: `CryptMark`, `KindTile`.

- [ ] **Step 1: Тесты в `idle.test.ts`**: после `start()` с 60 000 и `advanceTimersByTime(20_000)` → `remaining() === 40_000`; `activity()` возвращает к 60 000; после `stop()` → `null`.
- [ ] **Step 2:** FAIL → реализовать (хранить момент срабатывания) → PASS.
- [ ] **Step 3:** В `session.ts` — `autolockLeft/Total` (интервал 1 с только в `unlocked`; при `autolockMinutes <= 0` — оба `null`).
- [ ] **Step 4:** `AutolockRing.vue` (`size` 28, `stroke` 2, доля = left/total; не рендерит кольцо при `total === null`).
- [ ] **Step 5:** `AppSidebar.vue` по `Sidebar.dc.html`: шапка (drag‑region, кнопка блокировки с кольцом), блок хранилища (`CryptMark` 30, «Личное», `N записей`, изумрудная «+»), поиск ⌘K, Недавние/Секреты (янтарная иконка), «Типы» с `KindTile`, «Настроить типы», «Файлы» (дерево; выделение нейтральное, иконка активной папки — `primary`), теги чипами в строку, низ: Корзина со счётчиком + иконка Настроек. Пункт «Недавние» ведёт на `/`.
- [ ] **Step 6:** `pnpm test`, `pnpm build` → PASS; визуально сверить с макетом в `pnpm dev`.
- [ ] **Step 7: Commit** `feat(ui): сайдбар «Печать», кольцо автоблокировки`

### Task 5: Файлы и недавние, палитра ⌘K

**Files:**
- Modify: `src/views/Directory.vue`, `src/views/Home.vue`, `src/components/RecordList.vue`, `src/components/CommandPalette.vue`, `src/components/PageHeader.vue`
- Create: `src/lib/viewPref.ts`
- Test: `src/tests/viewPref.test.ts`

**Interfaces:**
- Produces: `useViewPref(key: string, initial: 'list' | 'grid'): Ref<'list' | 'grid'>` (localStorage `crypt.view.<key>`, мусор → `initial`).

- [ ] **Step 1:** Тест `viewPref`: значение сохраняется и читается заново; мусор → `initial`.
- [ ] **Step 2:** FAIL → реализовать → PASS.
- [ ] **Step 3:** `Directory.vue` в корне (`folderId` пуст): секция «Недавние» (4 карточки из того же источника, что `Home.vue`), «Папки» плитками, «Записи» корня; в папке — крошки, подпапки, записи. Переключатель «списком/карточками» (`SegmentedControl` с иконками) через `useViewPref('directory', 'list')`. Содержимое при смене папки — `<Transition>` c `.anim-in`. `Home.vue` (маршрут `/recent`) — тот же вид карточек.
- [ ] **Step 4:** `CommandPalette.vue`: группы «Записи» и «Команды», хоткеи справа (`font-mono`), `.anim-pop` у панели, затемнение с проявлением.
- [ ] **Step 5:** `pnpm test`, `pnpm build` → PASS; сверить с `Directory.dc.html`.
- [ ] **Step 6: Commit** `feat(ui): файлы и недавние, палитра`

### Task 6: Запись

**Files:**
- Modify: `src/views/Record.vue`, `src/components/record/FieldEditor.vue`, `src/components/record/TagEditor.vue`, `src/components/record/RecordHistory.vue` (только стиль), `src/components/editor/livePreview.ts` (стиль wiki‑ссылок и чекбоксов)

**Interfaces:**
- Consumes: `KindTile`, утилиты анимаций.
- Produces: `localStorage['crypt.record.inspector']` (`'1' | '0'`).

- [ ] **Step 1:** Шапка: крошки, статус «Сохраняется…» (спиннер) → «Сохранено · ЧЧ:ММ» (галочка `primary`, `.anim-in`), кнопка панели связей (сохраняет состояние), меню «⋯».
- [ ] **Step 2:** Чипы типа (`bg-primary-soft`) и тегов, «+ тег» пунктиром; заголовок 34 px.
- [ ] **Step 3:** Поля: строка с глифом вида (`Aa`, `#`, `◉`, `▦`, `↗`, `☑`, `◎`, `↔`, `⚿` — как в `Types.dc.html`), подсветка при наведении, кнопка копирования при наведении для `Text`/`Url`; `SecretLink` — янтарный чип; `RecordLink` — чипы с иконкой файла.
- [ ] **Step 4:** Правая панель: ширина 280 ↔ 0 с переходом `width .4s`, бэклинки карточками, мини‑история (3 последние версии точками), даты.
- [ ] **Step 5:** В редакторе: wiki‑ссылки — `bg-primary-soft text-primary-soft-fg rounded px-1.5`; чекбоксы задач — галочка `.anim-tick`, зачёркивание с переходом.
- [ ] **Step 6:** `pnpm test`, `pnpm build` → PASS; сверить с `Record.dc.html`.
- [ ] **Step 7: Commit** `feat(ui): экран записи`

### Task 7: Таблица типа

**Files:**
- Modify: `src/views/TypeTableView.vue`, `src/views/TagView.vue`, `src/views/FolderView.vue` (стиль списков)

- [ ] **Step 1:** Шапка с `KindTile` типа, кнопки «Поля типа» и основная «Новая …».
- [ ] **Step 2:** Виды (`ViewDef`) — вкладки с бегущим подчёркиванием (позиция по активной кнопке); активные фильтры — чипы `bg-primary-soft` с крестиком; поиск справа.
- [ ] **Step 3:** Таблица: заголовок со стрелкой сортировки (поворот 180° при смене направления), строка при наведении показывает стрелку «открыть», статусы `Select` — чипы цвета опции с точкой; новая строка — подсветка `primary` 500 мс. Числа — числом (звёзд нет).
- [ ] **Step 4:** `pnpm test`, `pnpm build` → PASS; сверить с `Table.dc.html`.
- [ ] **Step 5: Commit** `feat(ui): таблица типа`

### Task 8: Секреты

**Files:**
- Modify: `src/views/Secrets.vue`, `src/components/secret/{SecretPanel,SecretValue,SecretChip,TotpCode,SshKeySection,SecretHistory}.vue`
- Create: `src/lib/strength.ts`, `src/components/secret/CardFace.vue`
- Test: `src/tests/strength.test.ts`

**Interfaces:**
- Produces: `passwordStrength(value: string): { score: 0 | 1 | 2 | 3; label: 'Слабый' | 'Средний' | 'Надёжный' | 'Очень надёжный' }` — длина и число классов (строчные, заглавные, цифры, прочие): `score 3` при длине ≥ 20 и ≥ 3 классах или длине ≥ 28; `2` при ≥ 14 и ≥ 3 классах; `1` при ≥ 10; иначе `0`.
- Produces: `<CardFace :number="string | null" :holder="string | null" :expiry="string | null" />` — `number === null` → `•••• •••• •••• ••••`.
- Consumes: `SecretValuePill`, `KindTile`, `SegmentedControl`.

- [ ] **Step 1: Тесты `strength.test.ts`**: `''` → 0; `'abcdefghij'` → 1; `'Hollow-Brick-Tundra-42'` → 3; `'Abcdefgh1234!x'` → 2; `'aaaaaaaaaaaaaaaaaaaaaaaaaaaa'` (28) → 3.
- [ ] **Step 2:** FAIL → реализовать → PASS.
- [ ] **Step 3:** `Secrets.vue` по `Secrets.dc.html`: шапка (заголовок, кнопка «Импорт» — пока ведёт на Настройки → Перенос, кнопка «Генератор» → `/generator`, «+»), поиск, фильтры видов «таблетками» со счётчиками, список с янтарной `KindTile`.
- [ ] **Step 4:** `SecretValue.vue`: значение в `SecretValuePill`; индикатор надёжности **только** когда значение пароля уже раскрыто (или пришло из генератора); копирование — тост «<Поле> скопирован · очистится через N с» (N = `settings.clipboardClearSeconds`).
- [ ] **Step 5:** `TotpCode.vue`: кольцо отсчёта (SVG, `stroke-dashoffset` с переходом 1 с, цвет `secret` при ≤ 5 с), код меняется с `.anim-in`.
- [ ] **Step 6:** Карта: `CardFace` над полями; номер — `null`, пока не раскрыт через существующий `revealSecret`; подзаголовок в списке — `до ММ/ГГ` из `public.expiry`, иначе пусто.
- [ ] **Step 7:** `pnpm test`, `pnpm build` → PASS.
- [ ] **Step 8: Commit** `feat(ui): секреты`

### Task 9: Генератор в ядре

**Files:**
- Modify: `crates/crypt-core/src/gen.rs`
- Create: `crates/crypt-core/src/eff_wordlist.txt` (EFF large wordlist, 7 776 строк, только слова), подключение `include_str!`
- Modify: `src-tauri/src/commands/secrets.rs` (`generate_password` принимает новые опции; новая команда `generate_passwords`), `src/lib/types.ts` (`PasswordOptions`), `src/lib/api.ts`, `src/dev/mockBackend.ts`

**Interfaces:**
- Produces (Rust, serde camelCase, все поля с `default`):
  ```rust
  pub enum GenMode { Chars, Words }
  pub struct PasswordOptions {
      pub mode: GenMode, pub length: usize,            // 4..=64
      pub lower: bool, pub upper: bool, pub digits: bool,
      pub minus: bool, pub underline: bool, pub space: bool,
      pub symbols: bool, pub brackets: bool, pub latin1: bool,
      pub avoid_ambiguous: bool, pub each_class: bool,
      pub include: String, pub exclude: String,
      pub words: usize,                                // 3..=12
      pub separator: String, pub capitalize: bool, pub add_number: bool,
  }
  pub struct Generated { pub value: Zeroizing<String>, pub bits: u32 }
  pub fn generate(o: &PasswordOptions) -> Result<Generated>;
  pub fn generate_password(o: &PasswordOptions) -> Result<Zeroizing<String>>; // = generate(o)?.value, для совместимости
  ```
  Наборы: `symbols` = ``!"#$%&'*+,./:;=?@\^`|~``, `brackets` = `[]{}()<>`, `latin1` = `äöüßéèçñåø`; `avoid_ambiguous` убирает `O0Il1|`. Дефолты — как сейчас (24, lower/upper/digits/symbols).
  `bits`: Chars — `floor(length × log2(|пул|))`; Words — `floor(words × log2(7776) + (add_number ? log2(100) : 0))`.
- Produces (TS): `api.generatePassword(o: PasswordOptions): Promise<string>` (как сейчас); `api.generatePasswords(o: PasswordOptions, count: number): Promise<{ value: string; bits: number }[]>` — команда `generate_passwords`, `count` 1..=10.

- [ ] **Step 1: Rust‑тесты в `gen.rs`**:
  - `chars_respect_length_and_pool`: length 40 только digits → 40 цифр.
  - `each_class_guarantees_every_set`: 100 прогонов length 4, lower+upper+digits+symbols, `each_class` → в каждом есть символ каждого набора.
  - `avoid_ambiguous_and_exclude`: в 1 000 символах нет `O0Il1|` и символов из `exclude`.
  - `include_adds_chars`: только `include = "€"`, length 8 → `"€€€€€€€€"`.
  - `empty_pool_is_error`: все наборы выключены, `include` пуст → `Err(CryptError::..)` с понятным текстом «Выберите хотя бы один набор символов».
  - `words_mode`: words 5, separator `-`, capitalize → 5 частей, каждая с заглавной и есть в словаре; `add_number` → 6‑я часть — число 0..99.
  - `bits_values`: digits only length 6 → 19; words 5 → 64; words 5 + number → 70.
  - `wordlist_has_7776_unique_words`.
- [ ] **Step 2:** `cargo test -p crypt-core gen` → FAIL.
- [ ] **Step 3:** Реализовать (OsRng, `each_class` — по символу из каждого включённого набора + перемешивание).
- [ ] **Step 4:** `cargo test -p crypt-core` → PASS.
- [ ] **Step 5:** Команда `generate_passwords`, расширенный `PasswordOptions` в TS, заглушка в `mockBackend` (простая генерация на `crypto.getRandomValues`), `PasswordGenerator.vue` читает старые ключи `crypt.pwgen` без поломки.
- [ ] **Step 6:** `cargo test`, `pnpm test`, `pnpm build` → PASS.
- [ ] **Step 7: Commit** `feat(core): генератор паролей и фраз`

### Task 10: Экран генератора

**Files:**
- Create: `src/views/Generator.vue`
- Modify: `src/router.ts` (маршрут `generator`), `src/components/secret/PasswordGenerator.vue` (те же опции и профили, компактно)
- Test: `src/tests/generatorProfiles.test.ts`, Create: `src/lib/generatorProfiles.ts`

**Interfaces:**
- Consumes: `api.generatePasswords`, `SegmentedControl`, `SecretValuePill`.
- Produces: `GENERATOR_PROFILES: { id: 'standard' | 'pin6' | 'wifi' | 'phrase5'; label: string; options: Partial<PasswordOptions> }[]` — «Стандартный» (chars 20, lower/upper/digits/symbols, avoidAmbiguous), «PIN 6 цифр» (chars 6, только digits), «Wi‑Fi» (chars 16, lower/upper/digits, avoidAmbiguous), «Фраза 5 слов» (words 5, `-`, capitalize); `activeProfile(o: PasswordOptions): id | null`; `loadGeneratorOptions(): PasswordOptions` / `saveGeneratorOptions(o)` (localStorage `crypt.pwgen`).

- [ ] **Step 1: Тесты**: `activeProfile` для опций «Wi‑Fi» → `'wifi'`; после изменения длины → `null`; `loadGeneratorOptions` с мусором → дефолты.
- [ ] **Step 2:** FAIL → реализовать `generatorProfiles.ts` → PASS.
- [ ] **Step 3:** `Generator.vue` по `Generator.dc.html`: крошки «Секреты › Генератор паролей», сегменты «Символы/Фраза», большое значение (цифры — `text-primary-soft-fg`, буквы — `text-secret-fg` на `bg-secret-bg`), кнопка «заново» (поворот 180°), «Скопировать» (через существующую команду копирования значения с очисткой буфера — если копирования произвольной строки нет, добавить команду `copy_text` в `src-tauri/src/clipboard.rs` с той же очисткой), полоса стойкости по `bits` (пороги 50/75/100), настройки режимов, профили, 5 вариантов (`generatePasswords(o, 5)`), «Сохранить как секрет» (создаёт секрет‑логин с этим паролем через существующий `api` создания секрета и открывает его).
- [ ] **Step 4:** Ошибка пустого пула показывается под настройками, кнопки копирования неактивны.
- [ ] **Step 5:** `pnpm test`, `pnpm build`, `cargo test` → PASS.
- [ ] **Step 6: Commit** `feat(ui): генератор паролей`

### Task 11: Конструктор типов

**Files:**
- Modify: `src/views/Types.vue`

- [ ] **Step 1:** По `Types.dc.html`: список типов слева, шапка с `KindTile` и именем, поля — карточки с ручкой, глифом вида, вариантами `Select` чипами; «Добавить поле» — поповер‑сетка видов (`.anim-pop`), новое поле с подсветкой; справа предпросмотр записи, обновляется сразу; «Удалить тип» внизу. Выбора цвета типа нет.
- [ ] **Step 2:** `pnpm test`, `pnpm build` → PASS.
- [ ] **Step 3: Commit** `feat(ui): конструктор типов`

### Task 12: Настройки

**Files:**
- Modify: `src/views/Settings.vue`, `src/components/SnapshotsSection.vue`, `src/components/TransferSection.vue` (только стиль)

- [ ] **Step 1:** Разделы слева (Безопасность, Секреты и SSH, Хранилище, Опасная зона) с бегущей меткой; активный раздел в `?section=` и `localStorage['crypt.settings.section']`.
- [ ] **Step 2:** Безопасность: `SegmentedControl` автоблокировки (значения из текущей логики), Touch ID, PIN, **«Тема»: `SegmentedControl` Система/Светлая/Тёмная → `setThemePref`**, сид‑фраза: подтверждение → слова с задержкой 25 мс по очереди → полоса 60 с.
- [ ] **Step 3:** Секреты и SSH: сегменты очистки буфера, ssh‑agent с пульсирующей точкой, доверие ключу, автозапуск.
- [ ] **Step 4:** Хранилище: путь, снимки, перенос (KeePass/Markdown, включая «Экспорт в Markdown» — существующий `TransferSection`; план бэкапа потом переносит импорт на отдельный экран, экспорт остаётся здесь). Опасная зона: кнопка неактивна до «УДАЛИТЬ», при попытке — `.anim-shake`.
- [ ] **Step 5:** `pnpm test`, `pnpm build` → PASS.
- [ ] **Step 6: Commit** `feat(ui): настройки по разделам, выбор темы`

### Task 13: Корзина

**Files:**
- Modify: `src/views/Trash.vue`

- [ ] **Step 1:** По `Trash.dc.html`: действия строки при наведении, «Удалить навсегда» → подтверждение в строке; восстановление — строка уезжает вправо, удаление — сжимается (`<TransitionGroup>`), тост с папкой назначения; «Очистить корзину» — плашка подтверждения; пустое состояние с иллюстрацией.
- [ ] **Step 2:** `pnpm test`, `pnpm build` → PASS.
- [ ] **Step 3: Commit** `feat(ui): корзина`

### Task 14: Разблокировка и онбординг

**Files:**
- Modify: `src/views/Unlock.vue`, `src/views/Onboarding.vue`
- Test: существующие `src/tests/unlock.test.ts`, `onboarding-actions.test.ts` остаются зелёными

- [ ] **Step 1:** Unlock по `Unlock.dc.html`: `CryptMark` 76 с «парением», Touch ID с ореолом, сканирующей полосой и кольцами во время `busy`; PIN‑ячейки с подсветкой текущей, точки `.anim-tick`; ошибка — `.anim-shake` + красные ячейки; успех — `CryptMark animated`, затем переход; сид‑режим со счётчиком слов.
- [ ] **Step 2:** Onboarding по `Onboarding.dc.html`: тёмная панель слева со значком и шагами (линия прогресса), сид‑фраза скрыта до «Показать фразу», слова по очереди, чекбокс «Я записал(а)…» разблокирует «Проверить», «Скопировать» и «Распечатать» остаются (печать — существующий `.seed-sheet`), проверка слов с галочкой/красной рамкой по мере ввода, подсказка совпадения PIN. Третий вариант «Из бэкапа» **не добавлять** — его добавит план бэкапа.
- [ ] **Step 3:** `pnpm test`, `pnpm build` → PASS.
- [ ] **Step 4: Commit** `feat(ui): разблокировка и онбординг`

### Task 15: Окно трея

**Files:**
- Modify: `src-tauri/src/tray.rs`, `src-tauri/tauri.conf.json` (окно `tray`), `src-tauri/capabilities/default.json` (`"windows": ["main", "tray"]`), `src/router.ts`, `src/main.ts`
- Create: `src/views/Tray.vue`, `src-tauri/src/recent.rs`
- Modify: `src-tauri/src/state.rs` (поле `recent`), `src-tauri/src/commands/secrets.rs` (`copy_secret`, `copy_totp` отмечают использование; новая команда `recent_secrets`), место очистки ключа при блокировке (`commands/lifecycle.rs` / `autolock.rs`) — `recent.clear()`
- Test: Rust‑тесты в `recent.rs`; Rust‑тест `tray_labels_match_state` убрать вместе с меню

**Interfaces:**
- Produces (Rust): `pub struct RecentSecrets(Mutex<VecDeque<RecordId>>)` с `touch(&self, id)`, `list(&self, limit: usize) -> Vec<RecordId>`, `clear(&self)`; максимум 20, без дублей, новые впереди; только в памяти. `AppState.recent: RecentSecrets`. Команда `recent_secrets(limit: usize) -> Vec<Id>`; TS: `api.recentSecrets(limit = 6): Promise<Id[]>` (+ заглушка в `mockBackend`).
- Produces (Rust): окно `tray` — `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `skipTaskbar: true`, `visible: false`, `resizable: false`, 340×420, URL `index.html#/tray`. Клик по значку: показать окно под значком (позиция из `TrayIconEvent::Click { rect, .. }`, правый край окна = правый край значка, отступ 6 px вниз), повторный клик или потеря фокуса (`WindowEvent::Focused(false)`) — скрыть. Правый клик — то же. Меню у значка больше не задаётся.
- Produces (событие): `tray://shown` — окно трея обновляет данные при показе.
- Consumes: `api.secretOverview`, `api.copySecret`, `api.copyTotp`, `api.unlockBiometric`, `useSession`, `CryptMark`, `AutolockRing`.

- [ ] **Step 1: Rust‑тесты `recent.rs`**: три `touch` → `list` в порядке «последний первым»; повторный `touch` переносит вперёд без дубля; `list(2)` даёт 2; после 25 разных — хранится 20; `clear()` → пусто.
- [ ] **Step 2:** `cargo test -p <крейт src-tauri> recent` → FAIL → реализовать, подключить к `copy_secret`/`copy_totp` и к блокировке → PASS.
- [ ] **Step 3:** Роутер: маршрут `/tray` вне `AppShell`, гард пропускает его в любом состоянии; `main.ts` для `#/tray` монтирует тот же `App` (фон окна прозрачный, панель со скруглением 14 и тенью).
- [ ] **Step 4:** `Tray.vue` по `Tray.dc.html`: открыто — поиск (по всем секретам, `lib`), «Быстро скопировать» (`api.recentSecrets(6)`, при пустом списке — первые 6 секретов по дате изменения), строка: логин → пароль, у логина с TOTP — отдельная строка «· код» с кольцом и текущим кодом (`TotpCode` в компактном режиме), токен → токен, SSH → публичный ключ; клик копирует и показывает «Скопировано» 1,5 с; ⌘1…⌘9 — локальный `keydown` на окне трея. Статус: ssh‑agent и `autolockLeft`. Действия: «Открыть Crypt» (⌘O; вызывает существующий `show`), «Заблокировать» (⌘L), «Выйти» (⌘Q). Заблокировано — кнопка Touch ID, после успеха — список. При событии `vault-locked` список очищается сразу. Esc скрывает окно.
- [ ] **Step 5:** `tray.rs`: убрать `Menu`/`TrayState`, сохранить `show`/`hide`, слушатели `agent://*` теперь шлют `emit_to("tray", "tray://refresh")`; команды `tray_hide` и `app_quit` (если нужны фронту) в `commands/lifecycle.rs`.
- [ ] **Step 6:** `cargo test`, `pnpm test`, `pnpm build` → PASS; `pnpm tauri dev` → значок трея открывает окно под собой, клик вне закрывает, копирование работает, блокировка из главного окна очищает список.
- [ ] **Step 7: Commit** `feat(tray): всплывающее окно трея`

### Task 16: Итоговая проверка

- [ ] **Step 1:** `cargo test` (корень), `pnpm test`, `pnpm build`, `pnpm format:check` → всё PASS.
- [ ] **Step 2:** `pnpm dev`: пройти все экраны в светлой и тёмной теме, сверить с макетами; включить «Уменьшить движение» в macOS — анимации сводятся к прозрачности.
- [ ] **Step 3:** Контраст: `muted-foreground` на `background` и `card` в обеих темах ≥ 4.5:1 (проверить любым калькулятором; значения из спецификации уже подобраны так).
- [ ] **Step 4:** Обновить `README.md`: ссылка «Макеты» → новый холст.
- [ ] **Step 5: Commit** `docs: README — новые макеты`
