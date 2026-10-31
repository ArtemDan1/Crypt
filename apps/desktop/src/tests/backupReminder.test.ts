import { expect, test } from 'vitest'
import { backupReminder } from '@/lib/backupReminder'
test('reminds about missing backups and backups older than thirty days', () => {
  const now = 50 * 86400000
  expect(backupReminder(null, now)).toBe('never')
  expect(backupReminder(now - 10 * 86400000, now)).toBe('none')
  expect(backupReminder(now - 31 * 86400000, now)).toBe('stale')
})
