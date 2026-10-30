import type { FieldKind, FieldValue } from './types'

export type ParseResult = FieldValue | null | { error: string }

const numFmt = new Intl.NumberFormat('ru', { maximumFractionDigits: 6 })
const dateFmt = new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'long', year: 'numeric' })

/** Turn what the user typed into a field value; `null` clears the field. */
export function parseInput(raw: string, kind: FieldKind): ParseResult {
  const s = raw.trim()
  if (!s) return null
  switch (kind.kind) {
    case 'Number': {
      const n = Number(s.replace(/[\s  ]/g, '').replace(',', '.'))
      return Number.isFinite(n) ? { kind: 'Number', value: n } : { error: 'Нужно число' }
    }
    case 'Date':
      return /^\d{4}-\d{2}-\d{2}$/.test(s) && !Number.isNaN(Date.parse(s))
        ? { kind: 'Date', value: s }
        : { error: 'Дата в формате ГГГГ-ММ-ДД' }
    case 'Url':
      return { kind: 'Url', value: /^[a-z][a-z0-9+.-]*:/i.test(s) ? s : `https://${s}` }
    case 'Text':
      return { kind: 'Text', value: s }
    default:
      return { error: 'Это поле редактируется не текстом' }
  }
}

export interface FormatCtx {
  titleOf(id: string): string | undefined
}

export function optionsOf(kind: FieldKind) {
  return kind.kind === 'Select' || kind.kind === 'MultiSelect' ? kind.value.options : []
}

/** Plain-text rendering, used in tables and previews. */
export function formatValue(v: FieldValue | undefined, kind: FieldKind, ctx: FormatCtx): string {
  if (!v) return ''
  switch (v.kind) {
    case 'Number':
      return numFmt.format(v.value)
    case 'Date': {
      const d = new Date(`${v.value}T00:00:00`)
      return Number.isNaN(d.getTime()) ? v.value : dateFmt.format(d)
    }
    case 'Checkbox':
      return v.value ? 'Да' : 'Нет'
    case 'Select':
      return optionsOf(kind).find((o) => o.id === v.value)?.label ?? ''
    case 'MultiSelect':
      return v.value
        .map((id) => optionsOf(kind).find((o) => o.id === id)?.label)
        .filter(Boolean)
        .join(', ')
    case 'Links':
      return v.value.map((id) => ctx.titleOf(id) ?? '(удалено)').join(', ')
    default:
      return v.value
  }
}

/** Value as it goes into an <input>. */
export function inputValue(v: FieldValue | undefined): string {
  if (!v) return ''
  if (v.kind === 'Number') return String(v.value).replace('.', ',')
  if (v.kind === 'Text' || v.kind === 'Url' || v.kind === 'Date') return v.value
  return ''
}

export const OPTION_COLORS: Record<string, string> = {
  zinc: 'bg-zinc-700/60 text-zinc-200',
  red: 'bg-red-900/60 text-red-200',
  orange: 'bg-orange-900/60 text-orange-200',
  amber: 'bg-amber-900/60 text-amber-200',
  green: 'bg-green-900/60 text-green-200',
  teal: 'bg-teal-900/60 text-teal-200',
  blue: 'bg-blue-900/60 text-blue-200',
  purple: 'bg-purple-900/60 text-purple-200',
}
export const optionClass = (c: string | null) => OPTION_COLORS[c ?? 'zinc'] ?? OPTION_COLORS.zinc
