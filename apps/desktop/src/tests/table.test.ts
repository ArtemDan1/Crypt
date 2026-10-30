import { expect, test } from 'vitest'
import { applyView, defaultConfig, sanitizeConfig, type ViewConfig } from '@/lib/table'
import type { RecordView, TypeDef } from '@/lib/types'

const t: TypeDef = {
  id: 'T',
  name: 'Сервис',
  icon: 'globe',
  fields: [
    { id: 'price', name: 'Стоимость', kind: { kind: 'Number' } },
    {
      id: 'status',
      name: 'Статус',
      kind: {
        kind: 'Select',
        value: {
          options: [
            { id: 'a', label: 'Активна', color: null },
            { id: 'c', label: 'Отменена', color: null },
          ],
        },
      },
    },
    { id: 'site', name: 'Сайт', kind: { kind: 'Url' } },
  ],
}
const rec = (
  id: string,
  title: string,
  fields: RecordView['fields'] = {},
  hidden: RecordView['fields'] = {},
): RecordView => ({
  id,
  title,
  body: '',
  type_id: 'T',
  folder: null,
  tags: [],
  fields,
  hidden_fields: hidden,
  secret_kind: null,
  created_at: 0,
  updated_at: 0,
  deleted_at: null,
})
const titleOf = () => undefined
const ids = (rows: RecordView[]) => rows.map((r) => r.id)

test('defaultConfig lists title and every field in order', () => {
  expect(defaultConfig(t)).toEqual({ columns: ['title', 'price', 'status', 'site'], hidden: [], sort: [], filters: [] })
})

test('sanitizeConfig drops unknown columns and filters', () => {
  const cfg = {
    columns: ['title', 'gone', 'price'],
    hidden: ['gone'],
    sort: [{ col: 'gone', dir: 'asc' }],
    filters: [
      { col: 'gone', op: 'eq', value: 'x' },
      { col: 'price', op: 'notEmpty' },
    ],
  }
  const out = sanitizeConfig(cfg, t)
  expect(out.columns).toEqual(['title', 'price', 'status', 'site'])
  expect(out.hidden).toEqual([])
  expect(out.sort).toEqual([])
  expect(out.filters).toEqual([{ col: 'price', op: 'notEmpty' }])
})

test('sanitizeConfig on garbage returns default', () => {
  expect(sanitizeConfig('x', t)).toEqual(defaultConfig(t))
  expect(sanitizeConfig(null, t)).toEqual(defaultConfig(t))
})

test('sort numbers numerically, empties last', () => {
  const rows = [
    rec('a', 'A', { price: { kind: 'Number', value: 399 } }),
    rec('b', 'B', { price: { kind: 'Number', value: 1690 } }),
    rec('c', 'C'),
    rec('d', 'D', { price: { kind: 'Number', value: 12.4 } }),
  ]
  const asc: ViewConfig = { ...defaultConfig(t), sort: [{ col: 'price', dir: 'asc' }] }
  expect(ids(applyView(rows, asc, t, titleOf))).toEqual(['d', 'a', 'b', 'c'])
  const desc: ViewConfig = { ...defaultConfig(t), sort: [{ col: 'price', dir: 'desc' }] }
  expect(ids(applyView(rows, desc, t, titleOf))).toEqual(['b', 'a', 'd', 'c'])
})

test('sort text by ru locale', () => {
  const rows = [rec('1', 'Яндекс'), rec('2', 'Авито'), rec('3', 'ёлка')]
  const cfg: ViewConfig = { ...defaultConfig(t), sort: [{ col: 'title', dir: 'asc' }] }
  expect(applyView(rows, cfg, t, titleOf).map((r) => r.title)).toEqual(['Авито', 'ёлка', 'Яндекс'])
})

test('filter contains is case-insensitive; select compares by label', () => {
  const rows = [
    rec('1', 'Яндекс 360', { status: { kind: 'Select', value: 'a' } }),
    rec('2', 'Кинопоиск', { status: { kind: 'Select', value: 'c' } }),
  ]
  const c1: ViewConfig = { ...defaultConfig(t), filters: [{ col: 'title', op: 'contains', value: 'ЯНДЕКС' }] }
  expect(ids(applyView(rows, c1, t, titleOf))).toEqual(['1'])
  const c2: ViewConfig = { ...defaultConfig(t), filters: [{ col: 'status', op: 'eq', value: 'отменена' }] }
  expect(ids(applyView(rows, c2, t, titleOf))).toEqual(['2'])
  const c3: ViewConfig = { ...defaultConfig(t), filters: [{ col: 'price', op: 'empty' }] }
  expect(ids(applyView(rows, c3, t, titleOf))).toEqual(['1', '2'])
})

test('values of changed field kinds stay hidden and do not crash', () => {
  const rows = [rec('1', 'A', {}, { price: { kind: 'Text', value: 'старое' } })]
  const cfg: ViewConfig = {
    ...defaultConfig(t),
    sort: [{ col: 'price', dir: 'asc' }],
    filters: [{ col: 'price', op: 'empty' }],
  }
  expect(ids(applyView(rows, cfg, t, titleOf))).toEqual(['1'])
})
