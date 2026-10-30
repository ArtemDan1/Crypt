import type { SecretKind } from './types'
export interface SecretFieldMeta {
  name: string
  label: string
  sensitive: boolean
  multiline?: boolean
  mono?: boolean
  generator?: boolean
}
export const SECRET_FIELDS: Record<SecretKind, SecretFieldMeta[]> = {
  login: [
    { name: 'username', label: 'Логин', sensitive: false },
    { name: 'password', label: 'Пароль', sensitive: true, generator: true },
    { name: 'url', label: 'Адрес', sensitive: false },
    { name: 'totp_seed', label: 'Ключ TOTP', sensitive: true },
  ],
  card: [
    { name: 'number', label: 'Номер', sensitive: true, mono: true },
    { name: 'holder', label: 'Владелец', sensitive: false },
    { name: 'expiry', label: 'Срок', sensitive: false },
    { name: 'cvv', label: 'CVV', sensitive: true },
    { name: 'pin', label: 'PIN', sensitive: true },
  ],
  token: [
    { name: 'token', label: 'Токен', sensitive: true, mono: true },
    { name: 'url', label: 'Адрес', sensitive: false },
  ],
  sshkey: [
    { name: 'private_key', label: 'Приватный ключ', sensitive: true, multiline: true, mono: true },
    { name: 'public_key', label: 'Публичный ключ', sensitive: false, multiline: true, mono: true },
    { name: 'passphrase', label: 'Пароль ключа', sensitive: true },
    { name: 'comment', label: 'Комментарий', sensitive: false },
  ],
  note: [{ name: 'note', label: 'Текст', sensitive: true, multiline: true }],
}
export const SECRET_KIND_LABEL: Record<SecretKind, string> = {
  login: 'Логин',
  card: 'Карта',
  token: 'Токен',
  sshkey: 'SSH-ключ',
  note: 'Защищённая заметка',
}
export function maskPreview(_kind: SecretKind, _name: string): string {
  return '••••••••'
}
