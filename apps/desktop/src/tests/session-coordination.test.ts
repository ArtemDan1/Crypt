import { afterEach, expect, test, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises } from '@vue/test-utils'
import { useSession } from '@/stores/session'
const { lock } = vi.hoisted(() => ({ lock: vi.fn(async () => ({ state: 'locked', vaultId: 'v' })) }))
vi.mock('@/lib/api', () => ({
  api: { getSettings: async () => ({ autolockMinutes: 1 }), unlockMethods: async () => ({}), lock },
}))
afterEach(() => {
  vi.useRealTimers()
  window.history.replaceState(null, '', '#')
  vi.clearAllMocks()
})
test('tray never owns an idle timer that can override the main interval', async () => {
  vi.useFakeTimers()
  window.history.replaceState(null, '', '#/tray')
  setActivePinia(createPinia())
  const s = useSession()
  s.status = { state: 'unlocked', vaultId: 'v' }
  await flushPromises()
  vi.advanceTimersByTime(61_000)
  await flushPromises()
  expect(lock).not.toHaveBeenCalled()
})
test('UI locking delegates the editor flush to the backend coordinator', async () => {
  setActivePinia(createPinia())
  const s = useSession()
  s.status = { state: 'unlocked', vaultId: 'v' }
  await flushPromises()
  const flush = vi.fn()
  s.beforeLock(flush)
  await s.lock()
  expect(lock).toHaveBeenCalledOnce()
  expect(flush).not.toHaveBeenCalled()
})
