import { expect, test } from 'vitest'
import { findLinks, linkLabel, makeLink } from '@/lib/wikilinks'

const A = '01J9ZK7Q3M4N5P6R7S8T9V0W1X'
const B = '01J9ZK7Q3M4N5P6R7S8T9V0W2Y'

test('findLinks parses id and alias, skips garbage', () => {
  const text = `см. [[${A}]] и [[${B}|Аня]] и [[не-id]]`
  const links = findLinks(text)
  expect(links).toHaveLength(2)
  expect(links[0]).toMatchObject({ id: A, alias: undefined, from: 4 })
  expect(links[1]).toMatchObject({ id: B, alias: 'Аня' })
  expect(text.slice(links[1]!.from, links[1]!.to)).toBe(`[[${B}|Аня]]`)
})

test('makeLink sanitizes title', () => {
  expect(makeLink(A, 'a]|b')).toBe(`[[${A}|a  b]]`)
})

test('linkLabel prefers the current title and flags missing targets', () => {
  const titleOf = (id: string) => (id === A ? 'Новое имя' : undefined)
  expect(linkLabel(A, 'Старое', titleOf)).toEqual({ text: 'Новое имя', broken: false })
  expect(linkLabel(B, 'Аня', titleOf)).toEqual({ text: 'Аня', broken: true })
  expect(linkLabel(B, undefined, titleOf)).toEqual({ text: 'удалённая запись', broken: true })
})
