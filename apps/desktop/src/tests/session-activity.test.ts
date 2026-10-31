import { expect, test, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises } from '@vue/test-utils'
import { useSession } from '@/stores/session'
const { lock } = vi.hoisted(() => ({ lock: vi.fn(async () => ({ state: 'locked', vaultId: 'v' })) }))
vi.mock('@/lib/api', () => ({
  api: {
    getSettings: async () => ({ autolockMinutes: 1 }),
    unlockMethods: async () => ({}),
    lock,
  },
}))
test('activity from another window postpones automatic locking', async () => {
  vi.useFakeTimers()
  setActivePinia(createPinia())
  const s = useSession()
  s.status = { state: 'unlocked', vaultId: 'v' }
  await flushPromises()
  vi.advanceTimersByTime(50_000)
  s.activity()
  vi.advanceTimersByTime(50_000)
  expect(lock).not.toHaveBeenCalled()
  vi.advanceTimersByTime(10_000)
  await flushPromises()
  expect(lock).toHaveBeenCalledOnce()
  vi.useRealTimers()
})
