# Передача работы: редизайн «Печать» + бэкап/импорт

Ветка: `redesign-seal`. Не мержить в `main` и не пушить без явной просьбы владельца.

## Что читать сначала
1. Спецификации: `docs/superpowers/specs/2026-10-31-redesign-seal-design.md`, `docs/superpowers/specs/2026-10-31-backup-import-design.md`.
2. Планы: `docs/superpowers/plans/2026-10-31-redesign-seal.md` (выполняется сейчас), затем `docs/superpowers/plans/2026-10-31-backup-import.md`.
3. Журнал прогресса и принятых решений: `.superpowers/sdd/2026-10-31-redesign-seal/progress.md` (не в git). Строки `Ruling:` — решения, которые уже приняты; не переигрывать.
4. Макеты (HTML с данными, тексты и цвета): https://claude.ai/artifact/PCSwUujPE9RMxKYTJeKVua — если недоступны, ориентироваться на спецификацию и уже сделанные экраны.

## Состояние
План редизайна, задачи 1–13 готовы и закоммичены (тема, значок, примитивы, сайдбар, файлы, запись, таблица, секреты, генератор в Rust и экран генератора, конструктор типов, настройки, корзина).
Задача 14: `Unlock.vue` готов; **`Onboarding.vue` ещё не переделан**.
Осталось: задача 14 (онбординг), 15 (окно трея), 16 (итоговая проверка). Потом весь план бэкапа/импорта (8 задач).

## Как работать
- По шагам плана, TDD: сначала тест, увидеть падение, потом код, потом зелёный прогон.
- После каждой задачи: `cd apps/desktop && pnpm vitest run && pnpm build && pnpm prettier --check src`, в корне `cargo test`. Коммит на задачу, сообщение как в плане, в конце строка `Co-Authored-By: ...`.
- **Не запускать `cargo fmt` по всему workspace** — в проекте нет rustfmt.toml, он переформатирует чужие файлы. Форматировать только свои Rust‑файлы или не форматировать.
- Prettier для фронта: `pnpm prettier --write <свои файлы>`.
- Готовые общие компоненты: `src/components/crypt/{CryptMark,SegmentedControl,KindTile,SecretValuePill,AutolockRing}.vue`; утилиты анимаций в `src/styles.css` (`.anim-in`, `.anim-pop`, `.anim-shake`, `.anim-reveal`, `.anim-tick`, `.anim-fresh`); `src/lib/{theme,tones,fieldKinds,viewPref,strength,generatorProfiles}.ts`. SegmentedControl принимает проп `label` (не `aria-label`).
- Токены Tailwind: `bg-primary-soft text-primary-soft-fg`, `bg-secret-bg text-secret-fg text-secret`, `text-foreground-2`, `border-border-strong`. Изумрудный только для действий/активного; янтарь только для секретов.
- Анимации: 150–450 мс; бесконечные петли только для сканирования Touch ID и кольца TOTP.
- Проверка в браузере: `cd apps/desktop && pnpm dev` (фейковый бэкенд, http://localhost:1420). Вход: «Войти по сид‑фразе», слова — результат `window.__TAURI_INTERNALS__.invoke('generate_seed')`.
- Решения, отступающие от плана, дописывать в журнал строкой `Task N: Ruling: <что> — <почему> — <цена ошибки>`.

## Задача 14 (остаток): онбординг
`src/views/Onboarding.vue` по спецификации, раздел «Онбординг»: тёмная панель слева (CryptMark, слоган, вертикальные шаги с линией прогресса), справа шаг. Сид‑фраза скрыта до «Показать фразу», слова появляются по очереди (`.anim-reveal` с задержкой 25 мс), чекбокс «Я записал(а) фразу» разблокирует «Проверить»; «Скопировать» и «Распечатать» (существующий `.seed-sheet`) остаются; проверка слов с галочкой/красной рамкой по мере ввода; подсказка совпадения PIN. Логику (вызовы api, шаги, `?step=`) не менять. Варианта «Из бэкапа» не добавлять — его добавит план бэкапа. Тест `src/tests/onboarding-actions.test.ts` должен остаться зелёным.

## Задача 15 и далее
Строго по плану `2026-10-31-redesign-seal.md` (Task 15 — окно трея: недавние секреты только в памяти Rust, `recent.rs`; Task 16 — итоговая проверка), затем план `2026-10-31-backup-import.md` с Task 1.
В конце: обзор всей ветки (`git diff main...redesign-seal`) на баги и регрессии безопасности, список всех `Ruling:` из журнала — владельцу.
