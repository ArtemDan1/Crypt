import { expect, test } from 'vitest'
import { diffLines } from '@/lib/diff'

const show = (a: string, b: string) => diffLines(a, b).map((p) => `${p.op[0]}${p.text}`)

test('identical texts are all unchanged', () => {
  expect(show('a\nb', 'a\nb')).toEqual(['sa', 'sb'])
})
test('insertions, deletions and replacements keep order', () => {
  expect(show('a\nb\nc', 'a\nx\nc\nd')).toEqual(['sa', 'db', 'ax', 'sc', 'ad'])
  expect(show('a\nb\nc', 'a\nc')).toEqual(['sa', 'db', 'sc'])
})
test('empty sides', () => {
  expect(show('', 'x')).toEqual(['ax'])
  expect(show('x', '')).toEqual(['dx'])
  expect(show('', '')).toEqual([])
})
test('huge inputs fall back to a block replace instead of hanging', () => {
  const a = Array.from({ length: 5000 }, (_, i) => `a${i}`).join('\n')
  const b = Array.from({ length: 5000 }, (_, i) => `b${i}`).join('\n')
  const parts = diffLines(`top\n${a}\nend`, `top\n${b}\nend`)
  expect(parts[0]).toEqual({ op: 'same', text: 'top' })
  expect(parts.at(-1)).toEqual({ op: 'same', text: 'end' })
  expect(parts.filter((p) => p.op === 'del')).toHaveLength(5000)
})
