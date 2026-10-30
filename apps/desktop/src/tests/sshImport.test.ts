import { expect, test } from 'vitest'
import { defaultTitle, selectable } from '@/lib/sshImport'
import type { SshCandidate } from '@/lib/types'
test('import titles prefer nonempty comments and otherwise strip id_ prefix', () => {
  expect(defaultTitle('id_ed25519', 'anton@mac')).toBe('anton@mac')
  expect(defaultTitle('id_ed25519_work', '')).toBe('ed25519_work')
  expect(defaultTitle('deploy')).toBe('deploy')
  expect(defaultTitle('id_work', '   ')).toBe('work')
})
test('existing keys cannot be selected', () => {
  const c: SshCandidate = { path: '/tmp/id', fileName: 'id', encrypted: false, info: null, alreadyImported: true }
  expect(selectable(c)).toBe(false)
  expect(selectable({ ...c, alreadyImported: false })).toBe(true)
})
