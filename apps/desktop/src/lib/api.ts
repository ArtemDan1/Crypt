import { invoke } from '@tauri-apps/api/core'
import type {
  FieldValue,
  Folder,
  Id,
  ListFilter,
  NewRecord,
  RecordSummary,
  RecordView,
  SearchHit,
  Settings,
  TypeDef,
  UnlockMethods,
  VaultStatus,
  ViewDef,
  SecretOverview,
  SshKeyInfo,
  TotpCode,
  GeneratedPassword,
  PasswordOptions,
  SecretRevisionMeta,
  SshCandidate,
  SshImportItem,
  SshImportResult,
  AgentStatus,
  SshConfigStatus,
  RecordVersion,
  SnapshotInfo,
  SnapshotRestoreReport,
  Imported,
  KeepassReport,
  MarkdownImportReport,
  ExportReport,
} from './types'

export class CryptApiError extends Error {
  code: string
  constructor(code: string, message: string) {
    super(message)
    this.code = code
    this.name = 'CryptApiError'
  }
}

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args)
  } catch (e) {
    if (e && typeof e === 'object' && 'code' in e) {
      const { code, message } = e as { code: string; message?: string }
      throw new CryptApiError(String(code), String(message ?? code))
    }
    throw new CryptApiError('Unknown', typeof e === 'string' ? e : String(e))
  }
}

export const api = {
  exportBackup: (path: string, pin?: string) =>
    call<import('./types').BackupExport>('export_backup', { path, pin: pin || null }),
  recentSecrets: (limit = 6) => call<Id[]>('recent_secrets', { limit }),
  trayHide: () => call<void>('tray_hide'),
  appQuit: () => call<void>('app_quit'),
  show: () => call<void>('show'),
  secretOverview: (id: Id) => call<SecretOverview>('secret_overview', { id }),
  revealSecret: (id: Id, name: string) => call<string>('reveal_secret', { id, name }),
  copySecret: (id: Id, name: string) => call<void>('copy_secret', { id, name }),
  setSecret: (id: Id, name: string, value: string | null) => call<void>('set_secret', { id, name, value }),
  secretHistory: (id: Id, name: string) => call<SecretRevisionMeta[]>('secret_history', { id, name }),
  revealSecretRevision: (id: Id, name: string, index: number) =>
    call<string>('reveal_secret_revision', { id, name, index }),
  copySecretRevision: (id: Id, name: string, index: number) => call<void>('copy_secret_revision', { id, name, index }),
  generatePassword: (options: PasswordOptions) => call<string>('generate_password', { options }),
  copyText: (text: string) => call<void>('copy_text', { text }),
  generatePasswords: (options: PasswordOptions, count: number) =>
    call<GeneratedPassword[]>('generate_passwords', { options, count }),
  totpNow: (id: Id) => call<TotpCode | null>('totp_now', { id }),
  copyTotp: (id: Id) => call<void>('copy_totp', { id }),
  setSshKey: (id: Id, pem: string, passphrase?: string) =>
    call<SshKeyInfo>('set_ssh_key', { id, pem, passphrase: passphrase ?? null }),
  generateSshKey: (id: Id) => call<SshKeyInfo>('generate_ssh_key', { id }),
  setAgentEnabled: (id: Id, enabled: boolean) => call<void>('set_agent_enabled', { id, enabled }),
  scanSshDir: () => call<SshCandidate[]>('scan_ssh_dir'),
  importSshKeys: (items: SshImportItem[]) => call<SshImportResult[]>('import_ssh_keys', { items }),
  agentStatus: () => call<AgentStatus>('agent_status'),
  sshConfigStatus: () => call<SshConfigStatus>('ssh_config_status'),
  configureSshConfig: () => call<void>('configure_ssh_config'),
  unconfigureSshConfig: () => call<void>('unconfigure_ssh_config'),
  // lifecycle
  status: () => call<VaultStatus>('status'),
  generateSeed: () => call<string[]>('generate_seed'),
  createVault: (words: string[]) => call<VaultStatus>('create_vault', { words }),
  unlockWithSeed: (words: string[]) => call<VaultStatus>('unlock_with_seed', { words }),
  lock: () => call<VaultStatus>('lock'),
  getSettings: () => call<Settings>('get_settings'),
  setSettings: (settings: Settings) => call<Settings>('set_settings', { settings }),
  vaultPath: () => call<string>('vault_path'),
  deleteVault: (confirmation: string, pin?: string) =>
    call<VaultStatus>('delete_vault', { confirmation, pin: pin || null }),
  printSeed: () => call<void>('print_seed'),
  // unlock
  unlockMethods: () => call<UnlockMethods>('unlock_methods'),
  enableBiometric: () => call<void>('enable_biometric'),
  disableBiometric: () => call<void>('disable_biometric'),
  unlockBiometric: () => call<VaultStatus>('unlock_biometric'),
  setPin: (pin: string) => call<void>('set_pin', { pin }),
  removePin: () => call<void>('remove_pin'),
  unlockPin: (pin: string) => call<VaultStatus>('unlock_pin', { pin }),
  revealSeed: (pin?: string) => call<string[]>('reveal_seed', { pin: pin ?? null }),
  // library
  listTypes: () => call<TypeDef[]>('list_types'),
  putType: (t: TypeDef) => call<void>('put_type', { t }),
  deleteType: (id: Id) => call<void>('delete_type', { id }),
  starterTypes: () => call<TypeDef[]>('starter_types'),
  listFolders: () => call<Folder[]>('list_folders'),
  createFolder: (name: string, parent: Id | null) => call<Id>('create_folder', { name, parent }),
  renameFolder: (id: Id, name: string) => call<void>('rename_folder', { id, name }),
  moveFolder: (id: Id, parent: Id | null) => call<void>('move_folder', { id, parent }),
  deleteFolder: (id: Id) => call<void>('delete_folder', { id }),
  listViews: (typeId: Id) => call<ViewDef[]>('list_views', { typeId }),
  putView: (v: ViewDef) => call<void>('put_view', { v }),
  deleteView: (id: Id) => call<void>('delete_view', { id }),
  listRecords: (filter: ListFilter = {}) => call<RecordSummary[]>('list_records', { filter }),
  getRecord: (id: Id) => call<RecordView>('get_record', { id }),
  getRecords: (ids: Id[]) => call<RecordView[]>('get_records', { ids }),
  createRecord: (rec: NewRecord) => call<Id>('create_record', { new: rec }),
  setTitle: (id: Id, title: string) => call<void>('set_title', { id, title }),
  setBody: (id: Id, body: string) => call<void>('set_body', { id, body }),
  setType: (id: Id, typeId: Id | null) => call<void>('set_type', { id, typeId }),
  setFolder: (id: Id, folder: Id | null) => call<void>('set_folder', { id, folder }),
  addTag: (id: Id, tag: string) => call<void>('add_tag', { id, tag }),
  removeTag: (id: Id, tag: string) => call<void>('remove_tag', { id, tag }),
  setField: (id: Id, field: Id, value: FieldValue | null) => call<void>('set_field', { id, field, value }),
  deleteRecord: (id: Id) => call<void>('delete_record', { id }),
  restoreRecord: (id: Id) => call<void>('restore_record', { id }),
  purgeRecord: (id: Id) => call<void>('purge_record', { id }),
  emptyTrash: () => call<number>('empty_trash'),
  search: (query: string, limit = 20) => call<SearchHit[]>('search', { query, limit }),
  backlinks: (id: Id) => call<RecordSummary[]>('backlinks', { id }),
  allTags: () => call<[string, number][]>('all_tags'),
  recordVersions: (id: Id) => call<RecordVersion[]>('record_versions', { id }),
  recordAtVersion: (id: Id, version: string) => call<RecordView>('record_at_version', { id, version }),
  revertRecord: (id: Id, version: string) => call<void>('revert_record', { id, version }),
  listSnapshots: () => call<SnapshotInfo[]>('list_snapshots'),
  createSnapshot: (name: string) => call<SnapshotInfo>('create_snapshot', { name }),
  deleteSnapshot: (id: Id) => call<void>('delete_snapshot', { id }),
  restoreSnapshot: (id: Id) => call<SnapshotRestoreReport>('restore_snapshot', { id }),
  importKeepass: (path: string, password: string | null, keyfile: string | null) =>
    call<Imported<KeepassReport>>('import_keepass', { path, password, keyfile }),
  importMarkdown: (path: string) => call<Imported<MarkdownImportReport>>('import_markdown', { path }),
  exportMarkdown: (path: string, includeSecrets: boolean) =>
    call<ExportReport>('export_markdown', { path, includeSecrets }),
  flushDone: () => call<void>('flush_done'),
}
