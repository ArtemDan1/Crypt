import { autocompletion, type Completion, type CompletionContext } from '@codemirror/autocomplete'
import type { Extension } from '@codemirror/state'
import { makeLink } from '@/lib/wikilinks'

export interface LinkSource {
  /** Records matching the query (empty query → recent ones). */
  find(query: string): Promise<{ id: string; title: string; detail?: string }[]>
  /** Create a record titled `title` and return its id. */
  create(title: string): Promise<string>
}

export function wikilinkCompletion(src: LinkSource): Extension {
  return autocompletion({
    activateOnTyping: true,
    icons: false,
    override: [
      async (ctx: CompletionContext) => {
        const m = ctx.matchBefore(/\[\[[^\]\n|]*$/)
        if (!m) return null
        const query = m.text.slice(2)
        const found = await src.find(query)
        const options: Completion[] = found.map((r) => ({
          label: r.title || 'Без названия',
          detail: r.detail,
          apply: (view, _c, from, to) => {
            const ins = makeLink(r.id, r.title)
            view.dispatch({ changes: { from, to, insert: ins }, selection: { anchor: from + ins.length } })
          },
        }))
        if (query.trim()) {
          options.push({
            label: `Создать «${query.trim()}»`,
            boost: -99,
            apply: async (view, _c, from, to) => {
              const id = await src.create(query.trim())
              const ins = makeLink(id, query.trim())
              view.dispatch({ changes: { from, to, insert: ins }, selection: { anchor: from + ins.length } })
            },
          })
        }
        // Replace the whole `[[query` so the inserted link has no stray brackets.
        return { from: m.from, to: ctx.pos, options, filter: false }
      },
    ],
  })
}
