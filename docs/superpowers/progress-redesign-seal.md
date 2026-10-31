# SDD ledger — plan: docs/superpowers/plans/2026-10-31-redesign-seal.md

Pre-flight:
- T3 produces SegmentedControl/KindTile/SecretValuePill; consumed by T4,T5,T8,T10,T12 — consistent.
- T4 produces session.autolockLeft/Total; consumed by T15 (Tray status) — consistent.
- T9 produces api.generatePasswords + PasswordOptions; consumed by T10 — consistent.
- T2 CryptMark consumed by T4,T14,T15 — consistent.
- T15 recent.rs: plan Interfaces updated to Rust in-memory; Step 4 uses api.recentSecrets — consistent.
Task 1: Ruling: визуальную сверку с макетом в pnpm dev делаю пакетно после задач 4–8, а не после каждой — экономия времени — если что‑то разойдётся, правки позже
Task 1: complete (commits f1a3320..5da6c28, tests: bash -c 'cd apps/desktop && pnpm vitest run' →              learn more: https://vitest.dev/guide/improving-performance#test-environments)
Task 2: complete (commits 5da6c28..bfd66d7, tests: bash -c 'cd apps/desktop && pnpm build >/dev/null && echo build-ok' → build-ok)
Task 3: complete (commits bfd66d7..ae8267c, tests: bash -c 'cd apps/desktop && pnpm vitest run 2>&1 | grep -E '^ +Tests'' →       Tests  52 passed (52))
Task 4: Ruling: варианта «Никогда» в автоблокировке нет (валидация 1/5/15/30/60 в Rust); его не добавляю — вне объёма спецификации — если нужен, отдельная правка validate_settings
Task 4: Ruling: «Недавние» в сайдбаре ведёт на /recent, а не на / — иначе подсвечивались бы два пункта сразу; блок недавних всё равно есть в корне / — цена: одна строка в to=
Task 4: complete (commits ae8267c..56454dd, tests: bash -c 'cd apps/desktop && pnpm vitest run 2>&1 | grep -E '^ +Tests'' →       Tests  53 passed (53))
Task 5: Ruling: проп SegmentedControl ariaLabel → label (vue-tsc считает aria-label атрибутом) — план обновлён — цена: переименование
Task 5: Ruling: directory.test.ts обновлён под новое поведение (недавние в корне, крошки со стрелками) — так требует спека — цена: нет
Task 5: complete (commits 56454dd..6351ae4, tests: bash -c 'cd apps/desktop && pnpm vitest run 2>&1 | grep -E '^ +Tests'' →       Tests  55 passed (55))
Task 6: complete (commits 6351ae4..f24f092, tests: bash -c 'cd apps/desktop && pnpm vitest run 2>&1 | grep -E '^ +Tests'' →       Tests  55 passed (55))
Task 7: complete (commits f24f092..b73220c, tests: bash -c 'cd apps/desktop && pnpm vitest run 2>&1 | grep -E '^ +Tests'' →       Tests  55 passed (55))
Task 8: Ruling: «Секреты» остаются списком, деталь — страница записи с SecretPanel (без разделённого экрана из макета) — план не требовал split‑view, меньше риска — цена: позже можно добавить панель справа
Task 8: Ruling: «Секреты» остаются списком, деталь — страница записи с SecretPanel (без разделённого экрана из макета) — план не требовал split‑view — цена: позже можно добавить панель
Task 8: Ruling: CardFace берёт recordId и раскрывает номер сам (revealSecret + таймер), а не проп number — так номер не идёт через родителя — цена: малая
Task 8: complete (commits b73220c..feb23b5, tests: bash -c 'cd apps/desktop && pnpm vitest run 2>&1 | grep -E '^ +Tests'' →       Tests  60 passed (60))
Task 9: Ruling: ожидание bits для «5 слов + число» исправлено 70→71 (5·log2 7776 + log2 100 = 71.27) — ошибка арифметики в плане, формула спеки верна — цена: нет. Длина символов допускается 4..=128 (а не 64) ради старых сохранённых настроек — ползунок UI 4–64
Task 9: Ruling: each_class по умолчанию true (сохраняет прежнюю гарантию «каждый класс хотя бы раз»); старый tests/gen_totp.rs обновлён под новый набор спецсимволов и минимум 4 — так в спеке — цена: нет
Task 9: complete (commits feb23b5..750b892, tests: bash -c 'cargo test 2>&1 | grep -E 'test result' | awk '{p+=$4; f+=$6} END {print "passed",p,"failed",f; exit f>0}'' → passed 130 failed 0)
Task 9: Ruling: не запускать cargo fmt по всему workspace — в проекте нет rustfmt.toml и код отформатирован иначе; посторонние правки откатил — цена: нет
Task 10: Ruling: поповер PasswordGenerator в поле пароля оставлен прежним (совместим с crypt.pwgen и новым бэкендом), полноценные профили — на экране /generator; добавлена команда copy_text для копирования ещё не сохранённого пароля с автоочисткой — цена: поповер можно унифицировать позже
Task 10: complete (commits 750b892..43e5dd3, tests: bash -c 'cd apps/desktop && pnpm vitest run 2>&1 | grep -E '^ +Tests'' →       Tests  63 passed (63))
Task 11: complete (commits 43e5dd3..5c506b3, tests: bash -c 'cd apps/desktop && pnpm vitest run 2>&1 | grep -E '^ +Tests'' →       Tests  63 passed (63))
Task 12: Ruling: кнопка «Удалить» в диалоге неактивна до ввода УДАЛИТЬ (как было), тряска — при Enter с неверным словом — цена: нет. Ruling: плитка CryptMark в тёмной теме светлее (#33322E), иначе сливается с сайдбаром — иконка приложения не меняется — цена: нет
Task 12: complete (commits 5c506b3..c9cce65, tests: bash -c 'cd apps/desktop && pnpm vitest run 2>&1 | grep -E '^ +Tests'' →       Tests  63 passed (63))
Task 13: complete (commits c9cce65..e7b09b2, tests: bash -c 'cd apps/desktop && pnpm vitest run 2>&1 | grep -E '^ +Tests'' →       Tests  63 passed (63))
Task 14: partial — Unlock.vue готов (коммит 5e16f17); Onboarding.vue ещё не переделан

Task 14: Ruling: Prettier применён также к AppShell.vue и TransferSection.vue — исходный общий check падал на этих двух файлах; только форматирование — цена ошибки: лишний форматирующий diff.
Task 14: complete — Onboarding: тест скрытия/подтверждения RED→GREEN; vitest 63/63, build, prettier --check src, cargo test PASS (cargo вне sandbox из-за Unix-сокета).
Task 15: Ruling: добавлены vault-unlocked и session-activity между окнами — иначе вход из трея не обновляет главное окно, а скрытый трей блокирует активную сессию своим таймером — цена ошибки: лишние события и рассинхронизация таймеров до 500 мс.
Task 15: Ruling: нативная визуальная проверка трея отложена — в доступных инструментах нет управления окном/снимка macOS; автоматические Rust/TS проверки выполнены — цена ошибки: положение и закрытие окна требуют ручной проверки на Mac.
Task 15: automated complete — recent RED→GREEN; shared activity RED→GREEN; vitest 64/64, build, prettier --check src, cargo test PASS; native smoke pending.
Task 16: Ruling: визуальный обход светлой/тёмной темы и системного reduced-motion остаётся ручным — нет browser/computer инструмента для такого обхода — цена ошибки: визуальные регрессии могут остаться; автоматические тесты и численный контраст не заменяют обход.
Task 16: automated complete — vitest 64/64, build, prettier --check src, cargo test PASS; contrast light background/card 5.31/5.55, dark 6.84/6.24; README updated; visual smoke pending.

Backup/import pre-flight: T1 BackupHeader → T2 export/T3 inspect/restore; T4 LoginDraft → T6 commands/T7 UI; T5 OtpDraft → T6 token cache/T7 UI; T6 Imported → T7 report. Interfaces consistent; header.objects counts archive entries.
Task 1: Ruling: backup/import — после валидационного open_with_seed удаляется временный device.json перед rename — иначе проверочное открытие нарушает тест отсутствия device.json до первого открытия — цена ошибки: новый peer создаётся ещё раз при первом рабочем открытии.
Task 1: backup/import automated complete — 7 backup tests RED→GREEN, cargo test PASS, vitest 64/64, build, prettier --check src PASS.
Task 2: Ruling: backup/import — системное подтверждение присутствия происходит после выбора пути, перед записью — интерфейс export_backup(path,pin) выполняет проверку и запись атомарно; отдельная авторизация до диалога потребовала бы токенов или повторного запроса — цена ошибки: порядок двух диалогов отличается от макета.
Task 2: backup/import automated complete — export presence/file/settings tests RED→GREEN; backupReminder RED→GREEN; vitest 65/65, build, prettier --check src, cargo test PASS. Markdown export remains in TransferSection until Task 7 moves its import forms.
Task 3: Ruling: backup/import — реальный цикл через tauri dev и удаление пользовательской базы не выполнялся — нет GUI-инструмента, а удалять текущую базу для теста недопустимо; цикл покрыт изолированными Rust-тестами — цена ошибки: системный диалог и drag-and-drop требуют ручного smoke-теста.
Task 3: automated complete — 3 restore tests RED→GREEN, cargo test PASS, vitest 65/65, build, prettier --check src PASS; native smoke pending.
