import { beforeEach, afterEach, expect, test, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { effectScope, nextTick } from 'vue'
import { useRevealTimer } from '@/lib/reveal'
import { useSession } from '@/stores/session'
vi.mock('@/lib/api', () => ({ api: { getSettings: async () => ({ autolockMinutes: 15 }) } }))
beforeEach(() => {
  vi.useFakeTimers()
  setActivePinia(createPinia())
  useSession().status = { state: 'unlocked', vaultId: 'test' }
})
afterEach(() => vi.useRealTimers())
test('reveals expire, hide explicitly, and restart their timer', () => {
  const r = useRevealTimer()
  r.show('x')
  expect(r.revealed.value).toBe('x')
  vi.advanceTimersByTime(29999)
  r.show('y')
  vi.advanceTimersByTime(1)
  expect(r.revealed.value).toBe('y')
  vi.advanceTimersByTime(30000)
  expect(r.revealed.value).toBeNull()
  r.show('z')
  r.hide()
  expect(r.revealed.value).toBeNull()
})
test('locking and component disposal clear values and reject late replies', async () => {
  const scope = effectScope()
  const r = scope.run(() => useRevealTimer())!
  r.show('secret')
  await useSession().flushAll()
  expect(r.revealed.value).toBeNull()
  useSession().status = { state: 'locked', vaultId: 'test' }
  await nextTick()
  r.show('late')
  expect(r.revealed.value).toBeNull()
  scope.stop()
  r.show('later')
  expect(r.revealed.value).toBeNull()
})
