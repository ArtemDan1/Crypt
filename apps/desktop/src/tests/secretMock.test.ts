import { beforeEach, expect, test } from 'vitest'
import { install } from '@/dev/mockBackend'
import { api } from '@/lib/api'
beforeEach(async () => {
  localStorage.clear()
  await install()
  ;(window as unknown as { __cryptMockReset: () => void }).__cryptMockReset()
  await api.createVault([])
})
test('secret values and revisions stay out of persisted mock metadata', async () => {
  const id = await api.createRecord({ title: 'login', secret_kind: 'login' })
  await api.setSecret(id, 'password', 'private-old-123')
  await api.setSecret(id, 'password', 'private-new-456')
  const overview = await api.secretOverview(id)
  expect(overview.filled).toEqual(['password'])
  expect(JSON.stringify(overview)).not.toContain('private-')
  expect(await api.secretHistory(id, 'password')).toHaveLength(1)
  expect(await api.revealSecretRevision(id, 'password', 0)).toBe('private-old-123')
  expect(await api.revealSecret(id, 'password')).toBe('private-new-456')
  expect(localStorage.getItem('crypt-mock-db')).not.toContain('private-')
  await api.deleteRecord(id)
  await expect(api.revealSecret(id, 'password')).rejects.toMatchObject({ code: 'NotFound' })
})
test('SSH mock import returns independent errors and allows retry', async () => {
  const candidates = await api.scanSshDir()
  expect(candidates).toHaveLength(3)
  expect(candidates.filter((c) => c.alreadyImported)).toHaveLength(1)
  const selected = candidates.filter((c) => !c.alreadyImported)
  const result = await api.importSshKeys(
    selected.map((c) => ({ path: c.path, title: c.fileName, passphrase: 'wrong' })),
  )
  expect(result.filter((r) => r.recordId)).toHaveLength(1)
  expect(result.find((r) => r.error)?.error?.code).toBe('WrongPassphrase')
  const encrypted = selected.find((c) => c.encrypted)!
  const retry = await api.importSshKeys([{ path: encrypted.path, title: 'encrypted', passphrase: 'correct horse' }])
  expect(retry[0]?.recordId).toBeTruthy()
})
