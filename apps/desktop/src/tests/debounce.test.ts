import { afterEach, beforeEach, expect, test, vi } from 'vitest'
import { debouncedSaver } from '@/lib/debounce'

beforeEach(() => vi.useFakeTimers())
afterEach(() => vi.useRealTimers())

test('coalesces pushes and saves the last value after the delay', async () => {
  const save = vi.fn(async (_v: number) => {})
  const s = debouncedSaver(save, 600)
  s.push(1)
  s.push(2)
  await vi.advanceTimersByTimeAsync(599)
  expect(save).not.toHaveBeenCalled()
  await vi.advanceTimersByTimeAsync(1)
  expect(save).toHaveBeenCalledTimes(1)
  expect(save).toHaveBeenCalledWith(2)
  expect(s.pending()).toBe(false)
})

test('flush saves immediately and only once', async () => {
  const save = vi.fn(async (_v: string) => {})
  const s = debouncedSaver(save, 600)
  s.push('abc')
  expect(s.pending()).toBe(true)
  await s.flush()
  expect(save).toHaveBeenCalledWith('abc')
  await vi.advanceTimersByTimeAsync(1000)
  expect(save).toHaveBeenCalledTimes(1)
  await s.flush()
  expect(save).toHaveBeenCalledTimes(1)
})
