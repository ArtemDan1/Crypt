// DTOs mirroring the serde output of crypt-core / crypt-desktop.

export type VaultStatus =
  { state: 'noVault' } | { state: 'locked'; vaultId: string } | { state: 'unlocked'; vaultId: string }

export interface UnlockMethods {
  biometric: boolean
  biometricAvailable: boolean
  pin: boolean
  attemptsLeft: number | null
}

export interface Settings {
  lastBackupAt: number | null
  autolockMinutes: number
  clipboardClearSeconds: number
  agentEnabled: boolean
  agentTrustMinutes: number
  launchAtLogin: boolean
}

export type Id = string

export interface SelectOption {
  id: Id
  label: string
  color: string | null
}

export type FieldKind =
  | { kind: 'Text' }
  | { kind: 'Number' }
  | { kind: 'Date' }
  | { kind: 'Url' }
  | { kind: 'Checkbox' }
  | { kind: 'Select'; value: { options: SelectOption[] } }
  | { kind: 'MultiSelect'; value: { options: SelectOption[] } }
  | { kind: 'RecordLink'; value: { target_type: Id | null; multiple: boolean } }
  | { kind: 'SecretLink'; value: { multiple: boolean } }

export type FieldKindName = FieldKind['kind']

export type FieldValue =
  | { kind: 'Text'; value: string }
  | { kind: 'Number'; value: number }
  | { kind: 'Date'; value: string }
  | { kind: 'Url'; value: string }
  | { kind: 'Checkbox'; value: boolean }
  | { kind: 'Select'; value: Id }
  | { kind: 'MultiSelect'; value: Id[] }
  | { kind: 'Links'; value: Id[] }

export interface FieldDef {
  id: Id
  name: string
  kind: FieldKind
}

export interface TypeDef {
  id: Id
  name: string
  icon: string
  fields: FieldDef[]
}

export interface Folder {
  id: Id
  name: string
  parent: Id | null
}

export interface ViewDef {
  id: Id
  type_id: Id
  name: string
  config: unknown
}

export type SecretKind = 'login' | 'card' | 'token' | 'sshkey' | 'note'

export interface RecordView {
  id: Id
  title: string
  body: string
  type_id: Id | null
  folder: Id | null
  tags: string[]
  fields: Record<Id, FieldValue>
  hidden_fields: Record<Id, FieldValue>
  secret_kind: SecretKind | null
  created_at: number
  updated_at: number
  deleted_at: number | null
}

export interface RecordSummary {
  id: Id
  title: string
  type_id: Id | null
  folder: Id | null
  tags: string[]
  secret_kind: SecretKind | null
  updated_at: number
}

export interface ListFilter {
  typeId?: Id | null
  folder?: Id | null
  tag?: string | null
  secrets?: boolean | null
  trashed?: boolean
}

export interface NewRecord {
  title: string
  type_id?: Id | null
  folder?: Id | null
  secret_kind?: SecretKind | null
}

export interface SearchHit {
  id: Id
  title: string
  snippet: string
  in_title: boolean
}

export interface SshKeyInfo {
  algorithm: string
  bits: number | null
  public_key: string
  fingerprint: string
  comment: string
}
export interface SecretOverview {
  kind: SecretKind
  public: Record<string, string>
  filled: string[]
  historyCounts: Record<string, number>
  ssh: SshKeyInfo | null
  agentEnabled: boolean
  hasTotp: boolean
}
export interface TotpCode {
  code: string
  remaining: number
  period: number
}
export interface PasswordOptions {
  mode?: 'chars' | 'words'
  length: number
  lower: boolean
  upper: boolean
  digits: boolean
  symbols: boolean
  avoidAmbiguous: boolean
  minus?: boolean
  underline?: boolean
  space?: boolean
  brackets?: boolean
  latin1?: boolean
  eachClass?: boolean
  include?: string
  exclude?: string
  words?: number
  separator?: string
  capitalize?: boolean
  addNumber?: boolean
}
export interface GeneratedPassword {
  value: string
  bits: number
}
export interface SecretRevisionMeta {
  index: number
  replacedAt: number
}
export interface SshCandidate {
  path: string
  fileName: string
  encrypted: boolean
  info: SshKeyInfo | null
  alreadyImported: boolean
}
export interface SshImportItem {
  path: string
  title: string
  passphrase: string | null
}
export interface SshImportResult {
  path: string
  recordId: Id | null
  error: { code: string; message: string } | null
}
export interface AgentStatus {
  running: boolean
  socketPath: string
  identities: number
  error?: string | null
}
export interface SshConfigStatus {
  configured: boolean
  path: string
  block: string
  exportLine: string
}

/** One entry of a record's history; `id` is opaque. */
export interface RecordVersion {
  id: string
  at: number
  kind: string
  this_device: boolean
}
export interface SnapshotInfo {
  id: Id
  name: string
  created_at: number
  records: number
}
export interface SnapshotRestoreReport {
  restored: number
  trashed: number
  missing: number
}

export interface Imported<T> {
  report: T
  /** Name of the snapshot taken right before the import. */
  snapshot: string
}
export interface KeepassReport {
  imported: number
  extra_secrets: number
  skipped_recycled: number
  skipped_attachments: number
  invalid_totp: number
}
export interface MarkdownImportReport {
  notes: number
  folders: number
  typed: number
  links: number
  skipped_files: number
}
export interface ExportReport {
  path: string
  notes: number
  secrets: number
}

export interface BackupHeader {
  version: number
  created_at: number
  objects: number
}
export interface BackupExport {
  path: string
  bytes: number
  header: BackupHeader
}
