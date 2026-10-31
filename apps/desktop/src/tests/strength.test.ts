import { expect, test } from 'vitest'
import { passwordStrength } from '@/lib/strength'

test.each([
  ['', 0, 'Слабый'],
  ['abcdefghij', 1, 'Средний'],
  ['Abcdefgh1234!x', 2, 'Надёжный'],
  ['Hollow-Brick-Tundra-42', 3, 'Очень надёжный'],
  ['a'.repeat(28), 3, 'Очень надёжный'],
])('%s → %i', (value, score, label) => {
  expect(passwordStrength(value)).toEqual({ score, label })
})
