import { expect, test } from 'vitest'
import { clampDelta, settleOffset, sortShift, sortTarget } from '@/lib/sortable'

// Three rows 40px tall with 6px gaps.
const spans = [0, 46, 92].map((top) => ({ top, height: 40 }))

test('the target index follows the dragged center past neighbour midpoints', () => {
  expect(sortTarget(spans, 0, 20, 6)).toBe(0)
  expect(sortTarget(spans, 0, 50, 6)).toBe(1)
  expect(sortTarget(spans, 0, 130, 6)).toBe(2)
  expect(sortTarget(spans, 2, 10, 6)).toBe(0)
  expect(sortTarget(spans, 2, 112, 6)).toBe(2)
})

test('neighbours between the source and the target slide by one slot', () => {
  expect([0, 1, 2].map((i) => sortShift(i, 0, 2, 46))).toEqual([0, -46, -46])
  expect([0, 1, 2].map((i) => sortShift(i, 2, 0, 46))).toEqual([46, 46, 0])
  expect([0, 1, 2].map((i) => sortShift(i, 1, 1, 46))).toEqual([0, 0, 0])
})

test('the dragged row settles exactly where the slid neighbours left room', () => {
  expect(settleOffset(spans, 0, 2, 6)).toBe(92)
  expect(settleOffset(spans, 2, 0, 6)).toBe(-92)
  expect(settleOffset(spans, 1, 1, 6)).toBe(0)
})

test('a tall item can still be dragged above a short first item', () => {
  const mixed = [
    { top: 0, height: 48 },
    { top: 54, height: 84 },
  ]
  const delta = clampDelta(mixed, 1, -500)
  expect(sortTarget(mixed, 1, 54 + 42 + delta, 6)).toBe(0)
  expect(clampDelta(mixed, 1, 500)).toBe(42)
})
