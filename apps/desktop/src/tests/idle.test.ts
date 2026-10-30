import { afterEach, beforeEach, expect, test, vi } from 'vitest'
import { idleTimer } from '@/lib/idle'

beforeEach(() => vi.useFakeTimers())
afterEach(() => vi.useRealTimers())

test('fires once after the quiet period; activity postpones it', () => {
  const onIdle = vi.fn()
  const t = idleTimer(onIdle, 60_000)
  t.start()
  vi.advanceTimersByTime(59_000)
  t.activity()
  vi.advanceTimersByTime(59_000)
  expect(onIdle).not.toHaveBeenCalled()
  vi.advanceTimersByTime(1_000)
  expect(onIdle).toHaveBeenCalledTimes(1)
  vi.advanceTimersByTime(120_000)
  expect(onIdle).toHaveBeenCalledTimes(1)
})

test('setDelay applies to the running timer; stop cancels', () => {
  const onIdle = vi.fn()
  const t = idleTimer(onIdle, 60_000)
  t.start()
  t.setDelay(5_000)
  vi.advanceTimersByTime(5_000)
  expect(onIdle).toHaveBeenCalledTimes(1)
  t.start()
  t.stop()
  vi.advanceTimersByTime(10_000)
  expect(onIdle).toHaveBeenCalledTimes(1)
})

test('activity while stopped does not arm the timer', () => {
  const onIdle = vi.fn()
  const t = idleTimer(onIdle, 1_000)
  t.activity()
  vi.advanceTimersByTime(5_000)
  expect(onIdle).not.toHaveBeenCalled()
})
