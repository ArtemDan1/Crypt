import type { FieldKindName } from '@/lib/types'

/** Kind name, Russian label and a one-character glyph shown in field rows and the type builder. */
export const FIELD_KINDS: { kind: FieldKindName; label: string; glyph: string }[] = [
  { kind: 'Text', label: 'Текст', glyph: 'Aa' },
  { kind: 'Number', label: 'Число', glyph: '#' },
  { kind: 'Date', label: 'Дата', glyph: '▦' },
  { kind: 'Select', label: 'Выбор', glyph: '◉' },
  { kind: 'MultiSelect', label: 'Мультивыбор', glyph: '◎' },
  { kind: 'Checkbox', label: 'Флажок', glyph: '☑' },
  { kind: 'Url', label: 'URL', glyph: '↗' },
  { kind: 'RecordLink', label: 'Ссылка на запись', glyph: '↔' },
  { kind: 'SecretLink', label: 'Секрет', glyph: '⚿' },
]
export const kindMeta = (k: FieldKindName) => FIELD_KINDS.find((x) => x.kind === k)!
