export function backupReminder(lastAt: number | null, now: number): 'none' | 'never' | 'stale' {
  if (lastAt === null) return 'never'
  return now - lastAt > 30 * 86400000 ? 'stale' : 'none'
}
