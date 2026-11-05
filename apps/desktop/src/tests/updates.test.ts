import { beforeEach, expect, test, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useUpdates } from '@/stores/updates'

const api = vi.hoisted(() => ({ checkUpdate: vi.fn(), downloadUpdate: vi.fn(), installUpdate: vi.fn() }))
vi.mock('@/lib/api', () => ({ api }))
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }))

const info = { version: '2026.11.5', pkgUrl: 'u', releaseUrl: 'r', size: 10, notes: '' }
beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
  Object.values(api).forEach((f) => f.mockReset())
})

test('background check runs at most twice a day and stays quiet on errors', async () => {
  const u = useUpdates()
  api.checkUpdate.mockRejectedValueOnce(new Error('offline'))
  await u.checkSilently(1_000_000_000)
  expect(u.available).toBeNull()
  // Retry pause after a failure.
  await u.checkSilently(1_000_000_000 + 60_000)
  expect(api.checkUpdate).toHaveBeenCalledTimes(1)
  api.checkUpdate.mockResolvedValue(info)
  await u.checkSilently(1_000_000_000 + 31 * 60_000)
  expect(u.offer?.version).toBe('2026.11.5')
  await u.checkSilently(1_000_000_000 + 2 * 3600_000)
  expect(api.checkUpdate).toHaveBeenCalledTimes(2)
})

test('«Позже» hides until a manual check; a skipped version is not offered again', async () => {
  api.checkUpdate.mockResolvedValue(info)
  const u = useUpdates()
  await u.check()
  u.dismiss()
  expect(u.offer).toBeNull()
  await u.check()
  expect(u.offer).not.toBeNull()
  u.skip()
  expect(u.offer).toBeNull()
  setActivePinia(createPinia())
  const again = useUpdates()
  await again.check()
  expect(again.offer).toBeNull()
  expect(again.available?.version).toBe('2026.11.5')
})

test('install downloads then hands over to Installer; failures are shown', async () => {
  api.checkUpdate.mockResolvedValue(info)
  const u = useUpdates()
  await u.check()
  api.downloadUpdate.mockRejectedValueOnce(new Error('Нет сети'))
  await u.install()
  expect(u.error).toBe('Нет сети')
  expect(api.installUpdate).not.toHaveBeenCalled()
  await u.install()
  expect(api.downloadUpdate).toHaveBeenLastCalledWith('u', 10)
  expect(api.installUpdate).toHaveBeenCalled()
  expect(u.error).toBe('')
})
