import { describe, expect, test } from 'vitest'
import { formatValue, parseInput } from '@/lib/fields'
import type { FieldKind } from '@/lib/types'

const num: FieldKind = { kind: 'Number' }
const text: FieldKind = { kind: 'Text' }
const url: FieldKind = { kind: 'Url' }
const date: FieldKind = { kind: 'Date' }
const sel: FieldKind = { kind: 'Select', value: { options: [{ id: 'o1', label: 'Активна', color: 'green' }] } }
const links: FieldKind = { kind: 'RecordLink', value: { target_type: null, multiple: true } }

describe('parseInput', () => {
  test('number accepts ru formats', () => {
    expect(parseInput('1 690', num)).toEqual({ kind: 'Number', value: 1690 })
    expect(parseInput('399,5', num)).toEqual({ kind: 'Number', value: 399.5 })
    expect(parseInput('1 690', num)).toEqual({ kind: 'Number', value: 1690 })
    expect(parseInput('abc', num)).toHaveProperty('error')
  })
  test('empty clears', () => {
    expect(parseInput('  ', text)).toBeNull()
    expect(parseInput('', num)).toBeNull()
  })
  test('url gets scheme', () => {
    expect(parseInput('360.yandex.ru', url)).toEqual({ kind: 'Url', value: 'https://360.yandex.ru' })
    expect(parseInput('http://x.ru', url)).toEqual({ kind: 'Url', value: 'http://x.ru' })
  })
  test('date must be ISO', () => {
    expect(parseInput('2026-11-14', date)).toEqual({ kind: 'Date', value: '2026-11-14' })
    expect(parseInput('14.11.2026', date)).toHaveProperty('error')
  })
})

describe('formatValue', () => {
  const ctx = { titleOf: (id: string) => (({ a: 'Аня' }) as Record<string, string>)[id] }
  test('links use current titles and mark missing', () => {
    expect(formatValue({ kind: 'Links', value: ['a', 'b'] }, links, ctx)).toBe('Аня, (удалено)')
  })
  test('number in ru locale, select by label, empty is blank', () => {
    expect(formatValue({ kind: 'Number', value: 1690.5 }, num, ctx)).toBe('1\u00a0690,5')
    expect(formatValue({ kind: 'Select', value: 'o1' }, sel, ctx)).toBe('Активна')
    expect(formatValue(undefined, num, ctx)).toBe('')
  })
})
