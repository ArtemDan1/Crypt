// Same pattern as crypt-core's parse_wikilinks: [[<ULID>]] or [[<ULID>|alias]].
export const WIKILINK_RE = /\[\[([0-7][0-9A-HJKMNP-TV-Z]{25})(?:\|([^\]\n]*))?\]\]/g

export interface FoundLink {
  from: number
  to: number
  id: string
  alias?: string
}

export function findLinks(text: string): FoundLink[] {
  return [...text.matchAll(WIKILINK_RE)].map((m) => ({
    from: m.index!,
    to: m.index! + m[0].length,
    id: m[1]!,
    alias: m[2],
  }))
}

export function makeLink(id: string, title: string): string {
  return `[[${id}|${title.replace(/[\]|\n]/g, ' ')}]]`
}

export function linkLabel(id: string, alias: string | undefined, titleOf: (id: string) => string | undefined) {
  const t = titleOf(id)
  if (t !== undefined) return { text: t || 'Без названия', broken: false }
  return { text: alias || 'удалённая запись', broken: true }
}
