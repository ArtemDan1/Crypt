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
  zinc: 'bg-[#EFEDE8] text-[#3F3D39] dark:bg-[#2A2926] dark:text-[#CFCCC5]',
  red: 'bg-red-100 text-red-800 dark:bg-red-950/70 dark:text-red-200',
  orange: 'bg-orange-100 text-orange-800 dark:bg-orange-950/70 dark:text-orange-200',
  amber: 'bg-amber-100 text-amber-800 dark:bg-amber-950/70 dark:text-amber-200',
  green: 'bg-[#E3F1EC] text-[#0B5E46] dark:bg-[#173129] dark:text-[#7FD9B7]',
  teal: 'bg-teal-100 text-teal-800 dark:bg-teal-950/70 dark:text-teal-200',
  blue: 'bg-[#E4E8F1] text-[#33456B] dark:bg-[#1E2433] dark:text-[#A9B8DA]',
  purple: 'bg-[#F1E6F5] text-[#6A2F7F] dark:bg-[#2A1E31] dark:text-[#D3A8E2]',
}
export const optionClass = (c: string | null) => OPTION_COLORS[c ?? 'zinc'] ?? OPTION_COLORS.zinc
