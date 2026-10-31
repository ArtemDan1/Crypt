export interface ImportSource {
  id: string
  name: string
  group: 'Пароли из браузера' | 'Коды 2FA' | 'Другие менеджеры'
  format: string
  steps: string[]
}
const browser = (id: string, name: string, first: string, second: string): ImportSource => ({
  id,
  name,
  group: 'Пароли из браузера',
  format: 'CSV',
  steps: [first, second, 'Сохраните CSV на Mac и выберите его здесь. После импорта переместите файл в Корзину.'],
})
// Export paths checked against the vendors' help. Safari also supports exporting via macOS Passwords.
export const IMPORT_SOURCES: ImportSource[] = [
  browser(
    'chrome',
    'Chrome',
    'Откройте Chrome → Google Менеджер паролей → Настройки.',
    'В разделе «Экспорт паролей» скачайте файл и подтвердите действие паролем Mac.',
  ),
  browser(
    'safari',
    'Safari',
    'Откройте приложение «Пароли» на Mac и разблокируйте его.',
    'Выберите Файл → Экспортировать все пароли и сохраните CSV.',
  ),
  browser(
    'firefox',
    'Firefox',
    'Откройте Firefox → Пароли (about:logins).',
    'В меню ⋯ выберите «Экспорт логинов» и подтвердите действие.',
  ),
  browser(
    'edge',
    'Edge',
    'Откройте Edge → Настройки → Пароли.',
    'В меню сохранённых паролей выберите экспорт в CSV и подтвердите действие.',
  ),
  browser(
    'brave',
    'Brave',
    'Откройте Brave → Настройки → Автозаполнение и пароли → Менеджер паролей.',
    'В настройках менеджера выберите «Экспорт паролей» и скачайте CSV.',
  ),
  {
    id: 'ga',
    name: 'Google Authenticator',
    group: 'Коды 2FA',
    format: 'PNG / JPEG',
    steps: [
      'На телефоне откройте Google Authenticator → Перенести аккаунты → Экспортировать.',
      'Выберите аккаунты и сохраните скриншоты всех QR-кодов на Mac.',
      'Выберите все изображения сразу: если частей не хватает, Crypt покажет это в отчёте.',
    ],
  },
  {
    id: 'aegis',
    name: 'Aegis',
    group: 'Коды 2FA',
    format: 'JSON',
    steps: [
      'Откройте настройки Aegis → Импорт и экспорт → Экспорт.',
      'Сохраните JSON на Mac. Можно использовать зашифрованный экспорт: Crypt запросит пароль.',
      'Выберите файл и проверьте сопоставление кодов с логинами.',
    ],
  },
  {
    id: 'twofas',
    name: '2FAS',
    group: 'Коды 2FA',
    format: '.2fas',
    steps: [
      'Откройте настройки 2FAS → Резервная копия / Экспорт.',
      'Экспортируйте файл без пароля и сохраните его на Mac.',
      'Выберите .2fas здесь и проверьте список кодов.',
    ],
  },
  {
    id: 'keepass',
    name: 'KeePass',
    group: 'Другие менеджеры',
    format: '.kdbx / .kdb',
    steps: [
      'Выберите базу KeePass.',
      'Введите мастер-пароль и при необходимости выберите ключ-файл.',
      'Импорт создаст снимок для отката; вложения не переносятся.',
    ],
  },
  {
    id: 'markdown',
    name: 'Markdown / Obsidian',
    group: 'Другие менеджеры',
    format: 'Папка',
    steps: [
      'Выберите папку с Markdown или Obsidian.',
      'Подпапки станут папками, фронтматтер — полями и тегами.',
      'Результат и число пропущенных файлов появятся после импорта.',
    ],
  },
  {
    id: 'ssh',
    name: 'SSH-ключи',
    group: 'Другие менеджеры',
    format: '~/.ssh',
    steps: [
      'Crypt покажет ключи из ~/.ssh.',
      'Выберите ключи и введите пароль для защищённых ключей.',
      'Проверьте результат по каждому ключу.',
    ],
  },
]
export function defaultImportFolder(kind: 'browser' | 'otp', date: Date): string {
  const month = ['янв', 'фев', 'мар', 'апр', 'мая', 'июн', 'июл', 'авг', 'сен', 'окт', 'ноя', 'дек'][date.getMonth()]
  return `${kind === 'browser' ? 'Импорт из браузера' : 'Импорт 2FA'} · ${date.getDate()} ${month}`
}
