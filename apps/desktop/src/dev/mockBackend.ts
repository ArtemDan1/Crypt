// Dev-only in-memory backend so the UI can be previewed in a plain browser.
// Mirrors command names and payload shapes of src-tauri; not loaded in Tauri or in builds.
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import type {
  FieldValue,
  Folder,
  RecordView,
  TypeDef,
  ViewDef,
  Settings,
  SshKeyInfo,
  SecretOverview,
  PasswordOptions,
  SshCandidate,
  SshImportItem,
} from '@/lib/types'

import { SECRET_FIELDS } from '@/lib/secretFields'

const WORDS =
  'orbit lumber velvet canal shadow anchor quantum dizzy rescue ember fossil harbor tiger spray meadow cabin noble ripple sunset violin glacier pioneer walnut zebra'.split(
    ' ',
  )
const KEY = 'crypt-mock-db'

interface Db {
  exists: boolean
  unlocked: boolean
  vaultId: string
  pin: string | null
  attempts: number
  biometric: boolean
  settings: Settings
  types: TypeDef[]
  folders: Folder[]
  views: ViewDef[]
  records: RecordView[]
}

const ulid = () => {
  const a = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'
  let s = '0'
  for (let i = 0; i < 25; i++) s += a[Math.floor(Math.random() * 32)]
  return s
}
const now = () => Date.now()

function seedDb(): Db {
  const fid = () => ulid()
  const service: TypeDef = {
    id: ulid(),
    name: 'Сервис',
    icon: 'globe',
    fields: [
      {
        id: fid(),
        name: 'Статус',
        kind: {
          kind: 'Select',
          value: {
            options: [
              { id: ulid(), label: 'Активна', color: 'green' },
              { id: ulid(), label: 'Отменена', color: 'zinc' },
            ],
          },
        },
      },
      { id: fid(), name: 'Сайт', kind: { kind: 'Url' } },
      { id: fid(), name: 'Стоимость', kind: { kind: 'Number' } },
      { id: fid(), name: 'Продление', kind: { kind: 'Date' } },
      { id: fid(), name: 'Доступ', kind: { kind: 'SecretLink', value: { multiple: true } } },
    ],
  }
  const db: Db = {
    exists: false,
    unlocked: false,
    vaultId: 'be2c295f99d5a2cdb8196b45b5db9abf',
    pin: null,
    attempts: 0,
    biometric: false,
    settings: {
      autolockMinutes: 15,
      clipboardClearSeconds: 30,
      agentEnabled: true,
      agentTrustMinutes: 0,
      launchAtLogin: false,
      lastBackupAt: null,
    },
    types: [service],
    folders: [],
    views: [],
    records: [],
  }
  const f1 = { id: ulid(), name: 'Финансы', parent: null }
  const f2 = { id: ulid(), name: 'Подписки', parent: f1.id }
  db.folders.push(f1, f2, { id: ulid(), name: 'Работа', parent: null })
  const mk = (title: string, extra: Partial<RecordView> = {}): RecordView => {
    const r: RecordView = {
      id: ulid(),
      title,
      body: '',
      type_id: null,
      folder: null,
      tags: [],
      fields: {},
      hidden_fields: {},
      secret_kind: null,
      created_at: now() - 86400000,
      updated_at: now() - Math.floor(Math.random() * 86400000),
      deleted_at: null,
      ...extra,
    }
    db.records.push(r)
    return r
  }
  const login = mk('Яндекс ID', { secret_kind: 'login', folder: f1.id })
  const budget = mk('Бюджет 2026', { body: '# Бюджет\n\nПодписки в месяц — 4 870 ₽.', tags: ['финансы'] })
  const [st, site, price, due, access] = service.fields
  const active = (st!.kind as { value: { options: { id: string }[] } }).value.options[0]!.id
  mk('Яндекс 360', {
    type_id: service.id,
    folder: f2.id,
    tags: ['подписки', 'семья'],
    body: `Семейная подписка: почта на своём домене, Диск на 1 ТБ.\n\n## Почта на домене\n\n- Для почтовых клиентов нужен **пароль приложения**\n- [x] Перенести архив\n- [ ] Проверить DNS\n\n> Если цена вырастет — см. [[${budget.id}|Бюджет 2026]]\n`,
    fields: {
      [st!.id]: { kind: 'Select', value: active },
      [site!.id]: { kind: 'Url', value: 'https://360.yandex.ru' },
      [price!.id]: { kind: 'Number', value: 399 },
      [due!.id]: { kind: 'Date', value: '2026-11-14' },
      [access!.id]: { kind: 'Links', value: [login.id] },
    },
  })
  mk('Кинопоиск', {
    type_id: service.id,
    tags: ['подписки'],
    fields: { [price!.id]: { kind: 'Number', value: 299 }, [st!.id]: { kind: 'Select', value: active } },
  })
  mk('Идеи на выходные', { body: '- Сходить в музей\n- Починить велосипед' })
  return db
}

function load(): Db {
  try {
    const raw = localStorage.getItem(KEY)
    // Like the real app: a restart always starts locked.
    if (raw) {
      const stored = JSON.parse(raw)
      return { ...stored, settings: { ...seedDb().settings, ...stored.settings }, unlocked: false }
    }
  } catch {}
  return seedDb()
}
let db = load()
const save = () => {
  try {
    localStorage.setItem(KEY, JSON.stringify(db))
  } catch {}
}

class ApiErr {
  code: string
  message: string
  constructor(code: string, message: string) {
    this.code = code
    this.message = message
  }
}
const err = (code: string, message: string) => {
  throw new ApiErr(code, message)
}
const need = () => {
  if (!db.unlocked) err('Locked', 'Хранилище заблокировано')
}
const rec = (id: string) => db.records.find((r) => r.id === id) ?? err('NotFound', 'Запись не найдена')
const status = () =>
  !db.exists
    ? { state: 'noVault' }
    : db.unlocked
      ? { state: 'unlocked', vaultId: db.vaultId }
      : { state: 'locked', vaultId: db.vaultId }
const touch = (r: RecordView) => {
  r.updated_at = Math.max(now(), r.updated_at + 1)
}
const summary = (r: RecordView) => ({
  id: r.id,
  title: r.title,
  type_id: r.type_id,
  folder: r.folder,
  tags: r.tags,
  secret_kind: r.secret_kind,
  updated_at: r.updated_at,
})
const linksOf = (r: RecordView) => {
  const out = new Set<string>()
  for (const m of r.body.matchAll(/\[\[([0-7][0-9A-HJKMNP-TV-Z]{25})(?:\|[^\]\n]*)?\]\]/g)) out.add(m[1]!)
  for (const v of Object.values(r.fields)) if (v.kind === 'Links') v.value.forEach((x) => out.add(x))
  return out
}
function view(r: RecordView): RecordView {
  const t = db.types.find((x) => x.id === r.type_id)
  const fields: Record<string, FieldValue> = {}
  const hidden: Record<string, FieldValue> = { ...r.hidden_fields }
  for (const [k, v] of Object.entries(r.fields)) {
    if (t?.fields.some((f) => f.id === k)) {
      fields[k] =
        v.kind === 'Links' ? { kind: 'Links', value: v.value.filter((id) => db.records.some((x) => x.id === id)) } : v
    } else hidden[k] = v
  }
  return { ...r, fields, hidden_fields: hidden }
}

// Secret values deliberately stay outside Db/localStorage, including in browser preview.
const secretValues = new Map<string, Record<string, string>>()
const secretRevisions = new Map<string, Record<string, { value: string; replacedAt: number }[]>>()
const sshInfos = new Map<string, SshKeyInfo>()
const agentEnabled = new Map<string, boolean>()
const importedPaths = new Set<string>()
function wipeSecrets(id?: string) {
  if (id) {
    secretValues.delete(id)
    secretRevisions.delete(id)
    sshInfos.delete(id)
    agentEnabled.delete(id)
  } else {
    secretValues.clear()
    secretRevisions.clear()
    sshInfos.clear()
    agentEnabled.clear()
    importedPaths.clear()
  }
}
function secretRecord(id: string) {
  need()
  const r = rec(id)
  if (!r.secret_kind) err('Validation', 'Это не секрет')
  return r
}
function values(id: string) {
  secretRecord(id)
  if (!secretValues.has(id)) secretValues.set(id, {})
  return secretValues.get(id)!
}
function allowReveal(name: string) {
  if (name === 'totp_seed') err('Validation', 'Ключ TOTP доступен только для генерации кода')
}
function setValue(id: string, name: string, value: string | null) {
  const r = secretRecord(id)
  if (!SECRET_FIELDS[r.secret_kind!].some((f) => f.name === name)) err('Validation', 'Неизвестное поле')
  const v = values(id)
  if (v[name] === value) return
  if (v[name]) {
    const h = secretRevisions.get(id) ?? {}
    ;(h[name] ??= []).unshift({ value: v[name]!, replacedAt: now() })
    secretRevisions.set(id, h)
  }
  if (value === null) delete v[name]
  else v[name] = value
  touch(r)
}
function secretOverview(id: string): SecretOverview {
  const r = secretRecord(id),
    v = values(id),
    fields = SECRET_FIELDS[r.secret_kind!]
  return {
    kind: r.secret_kind!,
    public: Object.fromEntries(fields.filter((f) => !f.sensitive).map((f) => [f.name, v[f.name] ?? ''])),
    filled: fields.filter((f) => f.sensitive && v[f.name]).map((f) => f.name),
    historyCounts: Object.fromEntries(fields.map((f) => [f.name, secretRevisions.get(id)?.[f.name]?.length ?? 0])),
    ssh: sshInfos.get(id) ?? null,
    agentEnabled: agentEnabled.get(id) ?? true,
    hasTotp: !!v.totp_seed,
  }
}
async function copyMock(text: string) {
  if (!navigator.clipboard) return
  await navigator.clipboard.writeText(text)
  setTimeout(async () => {
    try {
      if ((await navigator.clipboard.readText()) === text) await navigator.clipboard.writeText('')
    } catch {}
  }, db.settings.clipboardClearSeconds * 1000)
}
const MOCK_WORDS =
  'acorn amber anchor apple arrow atlas badge bamboo banjo basin beacon birch bison blanket bloom bonfire bramble breeze bridge cabin cactus canal candle canyon carbon cedar cherry cinder clover cobalt comet copper coral cotton crater cricket delta dragon drift ember falcon fennel fern fjord flint fossil garnet glacier granite harbor hazel heron island ivory jasper juniper kettle lagoon lantern lemon linen lotus maple marble meadow mirror mosaic nectar nickel oasis orbit otter paddle pebble pepper pilot pine plaza quartz quill raven ribbon river saddle salmon shadow spruce summit tango thistle timber tundra velvet violet walnut willow zephyr'.split(
    ' ',
  )
function mockPick(n: number) {
  const bytes = new Uint32Array(1)
  crypto.getRandomValues(bytes)
  return bytes[0]! % n
}
function mockGenerate(o: PasswordOptions): { value: string; bits: number } {
  if (o.mode === 'words') {
    const n = o.words ?? 5
    if (n < 3 || n > 12) err('Validation', 'Число слов — от 3 до 12')
    const parts = Array.from({ length: n }, () => {
      const w = MOCK_WORDS[mockPick(MOCK_WORDS.length)]!
      return o.capitalize ? w[0]!.toUpperCase() + w.slice(1) : w
    })
    if (o.addNumber) parts.push(String(mockPick(100)))
    return {
      value: parts.join(o.separator ?? '-'),
      bits: Math.floor(n * Math.log2(7776) + (o.addNumber ? Math.log2(100) : 0)),
    }
  }
  const sets = [
    [o.lower, 'abcdefghijklmnopqrstuvwxyz'],
    [o.upper, 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'],
    [o.digits, '0123456789'],
    [o.minus, '-'],
    [o.underline, '_'],
    [o.space, ' '],
    [o.symbols, '!"#$%&\'*+,./:;=?@\\^`|~'],
    [o.brackets, '[]{}()<>'],
    [o.latin1, 'äöüßéèçñåø'],
    [!!o.include, o.include ?? ''],
  ] as const
  const keep = (c: string) => !(o.avoidAmbiguous && 'O0Il1|'.includes(c)) && !(o.exclude ?? '').includes(c)
  const selected = sets
    .filter(([on]) => on)
    .map(([, s]) => [...s].filter(keep))
    .filter((s) => s.length)
  const pool = [...new Set(selected.flat())]
  if (!pool.length) err('Validation', 'Выберите хотя бы один набор символов')
  if (!Number.isInteger(o.length) || o.length < 4 || o.length > 128)
    err('Validation', 'Длина пароля — от 4 до 128 символов')
  const chars = o.eachClass !== false && selected.length <= o.length ? selected.map((s) => s[mockPick(s.length)]!) : []
  while (chars.length < o.length) chars.push(pool[mockPick(pool.length)]!)
  for (let i = chars.length - 1; i > 0; i--) {
    const j = mockPick(i + 1)
    ;[chars[i], chars[j]] = [chars[j]!, chars[i]!]
  }
  return { value: chars.join(''), bits: Math.floor(o.length * Math.log2(pool.length)) }
}
function mockPassword(o: PasswordOptions) {
  return mockGenerate(o).value
}
const demoPublic = 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILu+jPmTl2mK6+UfBrEVDYJfXxfQ9104aEN5V39RKfhp'
function demoInfo(name: string): SshKeyInfo {
  return {
    algorithm: 'ssh-ed25519',
    bits: null,
    public_key: demoPublic,
    fingerprint: `SHA256:preview-${name}`,
    comment: `${name}@crypt`,
  }
}
function mockSetKey(id: string, info: SshKeyInfo) {
  if (secretRecord(id).secret_kind !== 'sshkey') err('Validation', 'Это не SSH-ключ')
  setValue(
    id,
    'private_key',
    '-----BEGIN OPENSSH PRIVATE KEY-----\nBROWSER PREVIEW ONLY\n-----END OPENSSH PRIVATE KEY-----',
  )
  setValue(id, 'public_key', info.public_key)
  setValue(id, 'comment', info.comment)
  sshInfos.set(id, info)
  return info
}
const mockCandidates: SshCandidate[] = [
  {
    path: '/preview/.ssh/id_ed25519',
    fileName: 'id_ed25519',
    encrypted: false,
    info: demoInfo('personal'),
    alreadyImported: false,
  },
  {
    path: '/preview/.ssh/id_work',
    fileName: 'id_work',
    encrypted: true,
    info: demoInfo('work'),
    alreadyImported: false,
  },
  {
    path: '/preview/.ssh/id_existing',
    fileName: 'id_existing',
    encrypted: false,
    info: demoInfo('existing'),
    alreadyImported: true,
  },
]
let sshConfigured = false
const secretHandlers: Record<string, (a: any) => unknown> = {
  preview_browser_csv: () => ({
    format: 'chromium',
    skippedDetails: [[5, 'NoPassword']],
    new: 2,
    duplicates: 1,
    skipped: 1,
    sample: [
      ['GitHub', 'alice'],
      ['Mail', 'bob'],
    ],
  }),
  import_browser_csv: () => ({
    report: { imported: 2, duplicates: 1, skipped: [[5, 'NoPassword']] },
    snapshot: 'Перед импортом браузера',
  }),
  preview_otp: ({ source }) => {
    if (source.kind === 'aegis' && !source.password) throw { code: 'NeedsPassword', message: 'Введите пароль экспорта' }
    return {
      token: 'demo-otp',
      rows: [
        { issuer: 'GitHub', account: 'alice', target: { attach: 'GitHub' } },
        { issuer: 'Mail', account: 'bob', target: 'new' },
      ],
      skipped: [['Старый аккаунт', 'Hotp']],
      missingBatches: [],
    }
  },
  import_otp: () => ({
    report: { attached: 1, created: 1, skipped: [['Старый аккаунт', 'Hotp']], missing_batches: [] },
    snapshot: 'Перед импортом 2FA',
  }),
  cancel_otp_import: () => {},
  trash_file: () => {},
  inspect_backup: () => ({ version: 1, created_at: Date.now() - 86400000, objects: 12, bytes: 24576 }),
  restore_backup: () => {
    db.exists = true
    db.unlocked = true
    return { state: 'unlocked', vaultId: 'demo' }
  },
  export_backup: ({ path }) => {
    const now = Date.now()
    db.settings.lastBackupAt = now
    return { path, bytes: 24576, header: { version: 1, created_at: now, objects: 12 } }
  },
  recent_secrets: () => [],
  tray_hide: () => {},
  app_quit: () => {},
  show: () => {},
  secret_overview: ({ id }) => secretOverview(id),
  set_secret: ({ id, name, value }) => {
    if (name === 'private_key' || (name === 'public_key' && secretRecord(id).secret_kind === 'sshkey'))
      err('Validation', 'Используйте импорт SSH-ключа')
    setValue(id, name, value)
  },
  reveal_secret: ({ id, name }) => {
    secretRecord(id)
    allowReveal(name)
    return values(id)[name] ?? ''
  },
  copy_secret: ({ id, name }) => {
    secretRecord(id)
    allowReveal(name)
    return copyMock(values(id)[name] ?? '')
  },
  secret_history: ({ id, name }) => {
    secretRecord(id)
    return (secretRevisions.get(id)?.[name] ?? []).map((r, index) => ({ index, replacedAt: r.replacedAt }))
  },
  reveal_secret_revision: ({ id, name, index }) => {
    secretRecord(id)
    allowReveal(name)
    return secretRevisions.get(id)?.[name]?.[index]?.value ?? err('Validation', 'Версия не найдена')
  },
  copy_secret_revision: ({ id, name, index }) => {
    secretRecord(id)
    allowReveal(name)
    return copyMock(secretRevisions.get(id)?.[name]?.[index]?.value ?? err('Validation', 'Версия не найдена'))
  },
  generate_password: ({ options }) => {
    need()
    return mockPassword(options)
  },
  copy_text: ({ text }) => {
    need()
    return copyMock(text)
  },
  generate_passwords: ({ options, count }) => {
    need()
    if (count < 1 || count > 10) err('Validation', 'Можно создать от 1 до 10 вариантов')
    return Array.from({ length: count }, () => mockGenerate(options))
  },
  totp_now: ({ id }) => {
    if (!values(id).totp_seed) return null
    const period = 30
    return { code: '123456', remaining: period - (Math.floor(now() / 1000) % period), period }
  },
  copy_totp: ({ id }) => (values(id).totp_seed ? copyMock('123456') : undefined),
  generate_ssh_key: ({ id }) => mockSetKey(id, demoInfo(rec(id).title)),
  set_ssh_key: ({ id, pem }) => {
    if (!pem.includes('PRIVATE KEY')) err('Validation', 'Это не приватный ключ')
    return mockSetKey(id, demoInfo(rec(id).title))
  },
  set_agent_enabled: ({ id, enabled }) => {
    secretRecord(id)
    agentEnabled.set(id, enabled)
  },
  scan_ssh_dir: () => {
    need()
    return mockCandidates.map((c) => ({ ...c, alreadyImported: c.alreadyImported || importedPaths.has(c.path) }))
  },
  import_ssh_keys: ({ items }) => {
    need()
    return (items as SshImportItem[]).map((item) => {
      const candidate = mockCandidates.find((c) => c.path === item.path)
      let error: { code: string; message: string } | null = null
      if (!candidate) error = { code: 'Validation', message: 'Ключ не найден' }
      else if (candidate.alreadyImported || importedPaths.has(item.path))
        error = { code: 'Validation', message: 'Ключ уже импортирован' }
      else if (candidate.encrypted && item.passphrase !== 'correct horse')
        error = {
          code: item.passphrase ? 'WrongPassphrase' : 'PassphraseRequired',
          message: item.passphrase ? 'Неверный пароль ключа' : 'Ключ защищён паролем',
        }
      if (error) return { path: item.path, recordId: null, error }
      const id = handlers.create_record!({ new: { title: item.title, secret_kind: 'sshkey' } }) as string
      mockSetKey(id, candidate!.info!)
      importedPaths.add(item.path)
      return { path: item.path, recordId: id, error: null }
    })
  },
  agent_status: () => ({
    running: db.settings.agentEnabled,
    socketPath: '~/Library/Application Support/dev.crypt.app/agent.sock',
    identities: db.records.filter(
      (r) => r.secret_kind === 'sshkey' && sshInfos.has(r.id) && (agentEnabled.get(r.id) ?? true),
    ).length,
  }),
  ssh_config_status: () => ({
    configured: sshConfigured,
    path: '~/.ssh/config',
    block:
      '# >>> Crypt ssh-agent >>>\nHost *\n  IdentityAgent "~/Library/Application Support/dev.crypt.app/agent.sock"\n# <<< Crypt ssh-agent <<<',
    exportLine: 'export SSH_AUTH_SOCK="$HOME/Library/Application Support/dev.crypt.app/agent.sock"',
  }),
  configure_ssh_config: () => {
    need()
    sshConfigured = true
  },
  unconfigure_ssh_config: () => {
    need()
    sshConfigured = false
  },
}

// History preview: in memory only, one entry per labelled edit (same label within 5 min merges).
interface MockVersion {
  id: string
  at: number
  kind: string
  this_device: boolean
  state: RecordView
}
const versions = new Map<string, MockVersion[]>()
const snapshots: {
  id: string
  name: string
  created_at: number
  records: Map<string, { version: string; trashed: boolean }>
}[] = []
const VERSION_KIND: Record<string, string> = {
  create_record: 'create',
  set_title: 'title',
  set_body: 'body',
  set_type: 'type',
  set_folder: 'folder',
  add_tag: 'tags',
  remove_tag: 'tags',
  set_field: 'fields',
  delete_record: 'trash',
  restore_record: 'trash',
  set_secret: 'secret',
}
function recordVersion(id: string, kind: string) {
  const r = db.records.find((x) => x.id === id)
  if (!r) return
  const list = versions.get(id) ?? []
  const last = list.at(-1)
  const entry = { id: ulid(), at: now(), kind, this_device: true, state: structuredClone(r) }
  if (last && last.kind === kind && entry.at - last.at < 300_000) list[list.length - 1] = entry
  else list.push(entry)
  versions.set(id, list)
}
const versionsOf = (id: string) => {
  rec(id)
  if (!versions.has(id)) recordVersion(id, 'create')
  return versions.get(id)!
}
const findVersion = (id: string, version: string) =>
  versionsOf(id).find((v) => v.id === version) ?? err('Validation', 'unknown version')
function revert(id: string, version: string) {
  const old = findVersion(id, version).state
  const r = rec(id)
  Object.assign(r, {
    title: old.title,
    body: old.body,
    type_id: old.type_id,
    tags: [...old.tags],
    fields: structuredClone(old.fields),
  })
  touch(r)
  recordVersion(id, 'revert')
}
const historyHandlers: Record<string, (a: any) => unknown> = {
  record_versions: ({ id }) => {
    need()
    return versionsOf(id)
      .map(({ state: _s, ...v }) => v)
      .reverse()
  },
  record_at_version: ({ id, version }) => {
    need()
    return view({ ...findVersion(id, version).state, id })
  },
  revert_record: ({ id, version }) => {
    need()
    revert(id, version)
  },
  list_snapshots: () =>
    snapshots.map((s) => ({ id: s.id, name: s.name, created_at: s.created_at, records: s.records.size })).reverse(),
  create_snapshot: ({ name }) => {
    need()
    if (!name.trim()) err('Validation', 'snapshot name is empty')
    const records = new Map(
      db.records.map((r) => [r.id, { version: versionsOf(r.id).at(-1)!.id, trashed: !!r.deleted_at }]),
    )
    const s = { id: ulid(), name: name.trim(), created_at: now(), records }
    snapshots.push(s)
    return { id: s.id, name: s.name, created_at: s.created_at, records: records.size }
  },
  delete_snapshot: ({ id }) => {
    const i = snapshots.findIndex((s) => s.id === id)
    if (i < 0) err('NotFound', 'Снимок не найден')
    snapshots.splice(i, 1)
  },
  restore_snapshot: ({ id }) => {
    need()
    const s = snapshots.find((x) => x.id === id) ?? err('NotFound', 'Снимок не найден')
    const report = { restored: 0, trashed: 0, missing: 0 }
    for (const [rid, e] of s.records) {
      const r = db.records.find((x) => x.id === rid)
      if (!r) {
        report.missing++
        continue
      }
      let changed = false
      if (versionsOf(rid).at(-1)!.id !== e.version) {
        revert(rid, e.version)
        changed = true
      }
      if (!!r.deleted_at !== e.trashed) {
        r.deleted_at = e.trashed ? now() : null
        changed = true
      }
      if (changed) report.restored++
    }
    for (const r of db.records)
      if (!s.records.has(r.id) && !r.secret_kind && !r.deleted_at) {
        r.deleted_at = now()
        report.trashed++
      }
    return report
  },
}

const handlers: Record<string, (a: any) => unknown> = {
  ...secretHandlers,
  ...historyHandlers,
  // Preview stand-ins: no files are read or written in the browser.
  'plugin:dialog|open': ({ options }) =>
    options?.directory ? '/Users/demo/Documents/Obsidian' : '/Users/demo/Passwords.kdbx',
  import_keepass: ({ password }) => {
    need()
    if (password !== 'demo') err('WrongImportKey', 'Неверный мастер-пароль или ключ-файл (в превью пароль: demo)')
    return {
      report: { imported: 0, extra_secrets: 0, skipped_recycled: 0, skipped_attachments: 0, invalid_totp: 0 },
      snapshot: 'Перед импортом Passwords.kdbx',
    }
  },
  import_markdown: () => {
    need()
    return {
      report: { notes: 0, folders: 1, typed: 0, links: 0, skipped_files: 0 },
      snapshot: 'Перед импортом Obsidian',
    }
  },
  export_markdown: ({ path }) => ({
    path: `${path}/Crypt ${new Date().toISOString().slice(0, 10)}`,
    notes: 0,
    secrets: 0,
  }),
  status: () => status(),
  delete_vault: ({ confirmation, pin }) => {
    need()
    if (confirmation !== 'УДАЛИТЬ') err('Validation', 'Введите УДАЛИТЬ')
    if (pin && pin !== db.pin) err('WrongPin', 'Неверный PIN')
    wipeSecrets()
    db = { ...seedDb(), types: [], folders: [], records: [], views: [] }
    return status()
  },
  print_seed: () => {
    window.print()
  },
  generate_seed: () => [...WORDS],
  create_vault: () => {
    if (db.exists) err('VaultExists', 'Хранилище уже существует')
    db.exists = true
    db.unlocked = true
    return status()
  },
  unlock_with_seed: ({ words }) => {
    if (words.join(' ') !== WORDS.join(' ')) err('WrongKey', 'Неверная сид-фраза или ключ')
    db.unlocked = true
    db.attempts = 0
    return status()
  },
  lock: () => {
    db.unlocked = false
    return status()
  },
  get_settings: () => db.settings,
  vault_path: () => '~/Library/Application Support/dev.crypt.app/vault',
  set_settings: ({ settings }) => {
    if (
      ![1, 5, 15, 30, 60].includes(settings.autolockMinutes) ||
      ![15, 30, 60, 90].includes(settings.clipboardClearSeconds) ||
      ![0, 1, 5, 15].includes(settings.agentTrustMinutes)
    )
      err('Validation', 'Недопустимый интервал')
    return (db.settings = settings)
  },
  unlock_methods: () => ({
    biometric: db.biometric,
    biometricAvailable: true,
    pin: !!db.pin,
    attemptsLeft: 5 - db.attempts,
  }),
  enable_biometric: () => {
    db.biometric = true
  },
  disable_biometric: () => {
    db.biometric = false
  },
  unlock_biometric: () => {
    if (!db.biometric) err('BiometricNotSet', 'Вход по Touch ID не настроен')
    db.unlocked = true
    db.attempts = 0
    return status()
  },
  set_pin: ({ pin }) => {
    db.pin = pin
    db.attempts = 0
  },
  remove_pin: () => {
    db.pin = null
  },
  unlock_pin: ({ pin }) => {
    if (!db.pin) err('PinLocked', 'Вход по PIN недоступен')
    if (pin !== db.pin) {
      db.attempts++
      if (db.attempts >= 5) {
        db.pin = null
        db.attempts = 0
        err('PinLocked', 'PIN введён неверно 5 раз и отключён')
      }
      err('WrongPin', `Неверный PIN. Осталось попыток: ${5 - db.attempts}`)
    }
    db.unlocked = true
    db.attempts = 0
    return status()
  },
  reveal_seed: () => [...WORDS],
  flush_done: () => {},
  list_types: () => {
    need()
    return [...db.types].sort((a, b) => a.name.localeCompare(b.name, 'ru'))
  },
  put_type: ({ t }) => {
    need()
    db.types = db.types.filter((x) => x.id !== t.id).concat(t)
  },
  delete_type: ({ id }) => {
    need()
    if (db.records.some((r) => r.type_id === id)) err('Validation', 'type is in use')
    db.types = db.types.filter((x) => x.id !== id)
  },
  starter_types: () => [
    {
      id: ulid(),
      name: 'Сервер',
      icon: 'server',
      fields: [
        ['Hostname', 'Text'],
        ['IP', 'Text'],
        ['ОС', 'Text'],
        ['Провайдер', 'Text'],
        ['SSH-порт', 'Number'],
        ['SSH-пользователь', 'Text'],
      ]
        .map(([name, kind]) => ({ id: ulid(), name, kind: { kind } }))
        .concat([{ id: ulid(), name: 'Креды', kind: { kind: 'SecretLink', value: { multiple: true } } as any }]),
    },
  ],
  list_folders: () => {
    need()
    return db.folders
  },
  create_folder: ({ name, parent }) => {
    need()
    const id = ulid()
    db.folders.push({ id, name, parent })
    return id
  },
  rename_folder: ({ id, name }) => {
    db.folders.find((f) => f.id === id)!.name = name
  },
  move_folder: ({ id, parent }) => {
    db.folders.find((f) => f.id === id)!.parent = parent
  },
  delete_folder: ({ id }) => {
    if (db.records.some((r) => r.folder === id)) err('Validation', 'folder has records')
    if (db.folders.some((f) => f.parent === id)) err('Validation', 'folder has children')
    db.folders = db.folders.filter((f) => f.id !== id)
  },
  list_views: ({ typeId }) => db.views.filter((v) => v.type_id === typeId),
  put_view: ({ v }) => {
    db.views = db.views.filter((x) => x.id !== v.id).concat(v)
  },
  delete_view: ({ id }) => {
    db.views = db.views.filter((x) => x.id !== id)
  },
  list_records: ({ filter }) => {
    need()
    return db.records
      .filter((r) => !!r.deleted_at === !!filter?.trashed)
      .filter((r) => !filter?.typeId || r.type_id === filter.typeId)
      .filter((r) => !filter?.folder || r.folder === filter.folder)
      .filter((r) => !filter?.tag || r.tags.includes(filter.tag))
      .filter((r) => filter?.secrets == null || !!r.secret_kind === filter.secrets)
      .sort((a, b) => b.updated_at - a.updated_at)
      .map(summary)
  },
  get_record: ({ id }) => {
    need()
    return view(rec(id))
  },
  get_records: ({ ids }) => {
    need()
    return ids
      .map((id: string) => db.records.find((r) => r.id === id))
      .filter(Boolean)
      .map(view)
  },
  create_record: ({ new: n }) => {
    need()
    const r: RecordView = {
      id: ulid(),
      title: n.title,
      body: '',
      type_id: n.type_id ?? null,
      folder: n.folder ?? null,
      tags: [],
      fields: {},
      hidden_fields: {},
      secret_kind: n.secret_kind ?? null,
      created_at: now(),
      updated_at: now(),
      deleted_at: null,
    }
    db.records.push(r)
    return r.id
  },
  set_title: ({ id, title }) => {
    const r = rec(id)
    r.title = title
    touch(r)
  },
  set_body: ({ id, body }) => {
    const r = rec(id)
    r.body = body
    touch(r)
  },
  set_type: ({ id, typeId }) => {
    const r = rec(id)
    r.type_id = typeId
    touch(r)
  },
  set_folder: ({ id, folder }) => {
    const r = rec(id)
    r.folder = folder
    touch(r)
  },
  add_tag: ({ id, tag }) => {
    const r = rec(id)
    const t = tag.trim().replace(/^#/, '').toLowerCase()
    if (!t) err('Validation', 'empty tag')
    if (!r.tags.includes(t)) r.tags = [...r.tags, t].sort()
    touch(r)
  },
  remove_tag: ({ id, tag }) => {
    const r = rec(id)
    r.tags = r.tags.filter((x) => x !== tag)
    touch(r)
  },
  set_field: ({ id, field, value }) => {
    const r = rec(id)
    const t = db.types.find((x) => x.id === r.type_id) ?? err('Validation', 'record has no current type')
    if (!t.fields.some((f) => f.id === field)) err('Validation', 'unknown field')
    if (value) r.fields[field] = value
    else delete r.fields[field]
    touch(r)
  },
  delete_record: ({ id }) => {
    wipeSecrets(id)
    const r = rec(id)
    if (r.secret_kind) db.records = db.records.filter((x) => x.id !== id)
    else {
      r.deleted_at = now()
      touch(r)
    }
  },
  restore_record: ({ id }) => {
    const r = rec(id)
    r.deleted_at = null
    touch(r)
  },
  purge_record: ({ id }) => {
    wipeSecrets(id)
    db.records = db.records.filter((x) => x.id !== id)
  },
  empty_trash: () => {
    const n = db.records.filter((r) => r.deleted_at).length
    db.records = db.records.filter((r) => !r.deleted_at)
    return n
  },
  search: ({ query, limit }) => {
    const q = query.trim().toLowerCase()
    if (!q) return []
    return db.records
      .filter((r) => !r.deleted_at)
      .map((r) => ({
        r,
        t: r.title.toLowerCase().includes(q),
        b: r.body.toLowerCase().includes(q) || r.tags.some((x) => x.includes(q)),
      }))
      .filter((x) => x.t || x.b)
      .sort((a, b) => Number(b.t) - Number(a.t) || b.r.updated_at - a.r.updated_at)
      .slice(0, limit)
      .map(({ r, t }) => {
        const i = Math.max(0, r.body.toLowerCase().indexOf(q))
        return {
          id: r.id,
          title: r.title,
          snippet: [...r.body].slice(Math.max(0, i - 25), i + 55).join(''),
          in_title: t,
        }
      })
  },
  backlinks: ({ id }) => db.records.filter((r) => !r.deleted_at && linksOf(r).has(id)).map(summary),
  all_tags: () => {
    const m = new Map<string, number>()
    db.records.filter((r) => !r.deleted_at).forEach((r) => r.tags.forEach((t) => m.set(t, (m.get(t) ?? 0) + 1)))
    return [...m].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
  },
}

export async function install() {
  mockWindows('main')
  mockIPC(async (cmd, args) => {
    const h = handlers[cmd]
    if (!h) {
      if (cmd.startsWith('plugin:')) return null
      throw { code: 'Unknown', message: `mock: no handler for ${cmd}` }
    }
    try {
      const out = await h(args ?? {})
      const kind = VERSION_KIND[cmd]
      if (kind) recordVersion(cmd === 'create_record' ? (out as string) : (args as any).id, kind)
      save()
      return structuredClone(out)
    } catch (e) {
      save()
      if (e instanceof ApiErr) throw { code: e.code, message: e.message }
      throw e
    }
  })
  ;(window as unknown as { __cryptMockReset: () => void }).__cryptMockReset = () => {
    wipeSecrets()
    db = seedDb()
    save()
  }
}
