import { formatValue } from './fields'
import type { FieldValue, RecordView, TypeDef } from './types'

export type FilterOp = 'eq' | 'contains' | 'empty' | 'notEmpty'
export interface Filter {
  col: string
  op: FilterOp
  value?: string
}
export interface SortKey {
  col: string
  dir: 'asc' | 'desc'
}
/** col: 'title' | 'tags' | 'folder' | <FieldId> */
export interface ViewConfig {
  columns: string[]
  hidden: string[]
  sort: SortKey[]
  filters: Filter[]
}

const BUILTIN = ['title', 'tags', 'folder']
const OPS: FilterOp[] = ['eq', 'contains', 'empty', 'notEmpty']

export function defaultConfig(t: TypeDef): ViewConfig {
  return { columns: ['title', ...t.fields.map((f) => f.id)], hidden: [], sort: [], filters: [] }
}

/** Accept whatever is stored; drop columns, sorts and filters the type no longer has. */
export function sanitizeConfig(cfg: unknown, t: TypeDef): ViewConfig {
  const def = defaultConfig(t)
  if (!cfg || typeof cfg !== 'object') return def
  const c = cfg as Partial<ViewConfig>
  const known = new Set([...BUILTIN, ...t.fields.map((f) => f.id)])
  const ok = (col: unknown): col is string => typeof col === 'string' && known.has(col)
  const cols = Array.isArray(c.columns) ? [...new Set(c.columns.filter(ok))] : []
  // Fields added to the type after the view was saved go to the end.
  for (const col of def.columns) if (!cols.includes(col)) cols.push(col)
  return {
    columns: cols,
    hidden: Array.isArray(c.hidden) ? c.hidden.filter(ok) : [],
    sort: Array.isArray(c.sort) ? c.sort.filter((s) => s && ok(s.col) && (s.dir === 'asc' || s.dir === 'desc')) : [],
    filters: Array.isArray(c.filters)
      ? c.filters
          .filter((f) => f && ok(f.col) && OPS.includes(f.op))
          .map((f) =>
            f.value === undefined ? { col: f.col, op: f.op } : { col: f.col, op: f.op, value: String(f.value) },
          )
      : [],
  }
}

type Cell = { kind: 'num'; v: number } | { kind: 'text'; v: string } | null

function cell(r: RecordView, col: string, t: TypeDef, titleOf: (id: string) => string | undefined): Cell {
  if (col === 'title') return r.title ? { kind: 'text', v: r.title } : null
  if (col === 'tags') return r.tags.length ? { kind: 'text', v: r.tags.join(', ') } : null
  if (col === 'folder') return r.folder ? { kind: 'text', v: r.folder } : null
  const def = t.fields.find((f) => f.id === col)
  const v: FieldValue | undefined = r.fields[col]
  if (!def || !v) return null
  if (v.kind === 'Number') return { kind: 'num', v: v.value }
  if (v.kind === 'Date') return { kind: 'text', v: v.value }
  const s = formatValue(v, def.kind, { titleOf })
  return s ? { kind: 'text', v: s } : null
}

const coll = new Intl.Collator('ru', { sensitivity: 'base', numeric: true })

function compare(a: Cell, b: Cell): number {
  if (!a && !b) return 0
  if (!a) return 1
  if (!b) return -1
  if (a.kind === 'num' && b.kind === 'num') return a.v - b.v
  return coll.compare(String(a.v), String(b.v))
}

function matches(c: Cell, f: Filter): boolean {
  if (f.op === 'empty') return !c
  if (f.op === 'notEmpty') return !!c
  if (!c) return false
  const needle = (f.value ?? '').trim().toLowerCase()
  const hay = String(c.v).toLowerCase()
  return f.op === 'eq'
    ? hay === needle || (c.kind === 'num' && Number(needle.replace(',', '.')) === c.v)
    : hay.includes(needle)
}

export function applyView(
  rows: RecordView[],
  cfg: ViewConfig,
  t: TypeDef,
  titleOf: (id: string) => string | undefined,
): RecordView[] {
  const filtered = rows.filter((r) => cfg.filters.every((f) => matches(cell(r, f.col, t, titleOf), f)))
  if (!cfg.sort.length) return filtered
  return [...filtered].sort((a, b) => {
    for (const s of cfg.sort) {
      const ca = cell(a, s.col, t, titleOf)
      const cb = cell(b, s.col, t, titleOf)
      // Empty cells stay last in both directions.
      if (!ca || !cb) {
        const d = compare(ca, cb)
        if (d) return d
        continue
      }
      const d = compare(ca, cb) * (s.dir === 'asc' ? 1 : -1)
      if (d) return d
    }
    return 0
  })
}

export function columnLabel(col: string, t: TypeDef): string {
  if (col === 'title') return 'Название'
  if (col === 'tags') return 'Теги'
  if (col === 'folder') return 'Папка'
  return t.fields.find((f) => f.id === col)?.name ?? col
}
