import { expect, test } from 'vitest'
import { SECRET_FIELDS, SECRET_KIND_LABEL } from '@/lib/secretFields'
import type { SecretKind } from '@/lib/types'
test('fields follow core value_names and disclose only public values', () => {
  const names: Record<SecretKind, string[]> = {
    login: ['username', 'password', 'url', 'totp_seed'],
    card: ['number', 'holder', 'expiry', 'cvv', 'pin'],
    token: ['token', 'url'],
    sshkey: ['private_key', 'public_key', 'passphrase', 'comment'],
    note: ['note'],
  }
  const visible: Record<SecretKind, string[]> = {
    login: ['username', 'url'],
    card: ['holder', 'expiry'],
    token: ['url'],
    sshkey: ['public_key', 'comment'],
    note: [],
  }
  for (const kind of Object.keys(names) as SecretKind[]) {
    expect(SECRET_FIELDS[kind].map((f) => f.name)).toEqual(names[kind])
    expect(SECRET_FIELDS[kind].filter((f) => !f.sensitive).map((f) => f.name)).toEqual(visible[kind])
    expect(SECRET_KIND_LABEL[kind]).toBeTruthy()
  }
})
