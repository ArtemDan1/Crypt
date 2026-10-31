// Obsidian-style live preview: markup is hidden on lines without the cursor.
import { syntaxTree } from '@codemirror/language'
import { RangeSetBuilder, StateEffect, type EditorState, type Extension, type Range } from '@codemirror/state'
import { Decoration, type DecorationSet, EditorView, ViewPlugin, type ViewUpdate, WidgetType } from '@codemirror/view'
import { findLinks, linkLabel } from '@/lib/wikilinks'

export interface LinkHost {
  titleOf(id: string): string | undefined
  open(id: string): void
}

/** Dispatch to re-render link widgets after titles changed elsewhere. */
export const refreshLinks = StateEffect.define<null>()

class CheckboxWidget extends WidgetType {
  readonly checked: boolean
  readonly pos: number
  constructor(checked: boolean, pos: number) {
    super()
    this.checked = checked
    this.pos = pos
  }
  eq(o: CheckboxWidget) {
    return o.checked === this.checked && o.pos === this.pos
  }
  toDOM(view: EditorView) {
    const box = document.createElement('input')
    box.type = 'checkbox'
    box.checked = this.checked
    box.className = 'cm-task'
    box.setAttribute('aria-label', this.checked ? 'Отметить как невыполненное' : 'Отметить как выполненное')
    box.addEventListener('mousedown', (e) => {
      e.preventDefault()
      view.dispatch({ changes: { from: this.pos + 1, to: this.pos + 2, insert: this.checked ? ' ' : 'x' } })
    })
    return box
  }
  ignoreEvent() {
    return false
  }
}

class LinkWidget extends WidgetType {
  readonly id: string
  readonly text: string
  readonly broken: boolean
  readonly host: LinkHost
  constructor(id: string, text: string, broken: boolean, host: LinkHost) {
    super()
    this.id = id
    this.text = text
    this.broken = broken
    this.host = host
  }
  eq(o: LinkWidget) {
    return o.id === this.id && o.text === this.text && o.broken === this.broken
  }
  toDOM() {
    const a = document.createElement(this.broken ? 'span' : 'a')
    a.textContent = this.text
    a.className = this.broken ? 'cm-wikilink cm-wikilink-broken' : 'cm-wikilink'
    if (!this.broken) {
      a.setAttribute('href', `#/r/${this.id}`)
      a.addEventListener('mousedown', (e) => {
        e.preventDefault()
        this.host.open(this.id)
      })
    } else a.title = 'Запись удалена'
    return a
  }
  ignoreEvent() {
    return true
  }
}

const hide = Decoration.replace({})
const mark = (cls: string) => Decoration.mark({ class: cls })
const line = (cls: string) => Decoration.line({ class: cls })

function activeLines(state: EditorState, focused: boolean): Set<number> {
  const out = new Set<number>()
  if (!focused) return out
  for (const r of state.selection.ranges) {
    const a = state.doc.lineAt(r.from).number
    const b = state.doc.lineAt(r.to).number
    for (let n = a; n <= b; n++) out.add(n)
  }
  return out
}

function build(view: EditorView, host: LinkHost): DecorationSet {
  const { state } = view
  const active = activeLines(state, view.hasFocus)
  const isActive = (pos: number) => active.has(state.doc.lineAt(pos).number)
  const decos: Range<Decoration>[] = []

  for (const { from, to } of view.visibleRanges) {
    syntaxTree(state).iterate({
      from,
      to,
      enter: (n) => {
        const name = n.name
        const heading = /^ATXHeading(\d)$/.exec(name)
        if (heading) decos.push(line(`cm-h${Math.min(+heading[1]!, 3)}`).range(state.doc.lineAt(n.from).from))
        else if (name === 'Emphasis') decos.push(mark('cm-em').range(n.from, n.to))
        else if (name === 'StrongEmphasis') decos.push(mark('cm-strong').range(n.from, n.to))
        else if (name === 'Strikethrough') decos.push(mark('cm-strike').range(n.from, n.to))
        else if (name === 'InlineCode') decos.push(mark('cm-inline-code').range(n.from, n.to))
        else if (name === 'Blockquote') {
          for (let p = n.from; p <= n.to;) {
            const l = state.doc.lineAt(p)
            decos.push(line('cm-quote').range(l.from))
            p = l.to + 1
          }
        } else if (name === 'FencedCode') {
          for (let p = n.from; p <= n.to;) {
            const l = state.doc.lineAt(p)
            decos.push(line('cm-codeblock').range(l.from))
            p = l.to + 1
          }
          return false
        } else if (name === 'TaskMarker' && !isActive(n.from)) {
          const text = state.doc.sliceString(n.from, n.to)
          decos.push(Decoration.replace({ widget: new CheckboxWidget(/x/i.test(text), n.from) }).range(n.from, n.to))
        }

        if (isActive(n.from)) return
        if (name === 'HeaderMark') {
          const end = state.doc.sliceString(n.to, n.to + 1) === ' ' ? n.to + 1 : n.to
          decos.push(hide.range(n.from, end))
        } else if (name === 'EmphasisMark' || name === 'StrikethroughMark') {
          decos.push(hide.range(n.from, n.to))
        } else if (name === 'CodeMark' && n.node.parent?.name === 'InlineCode') {
          decos.push(hide.range(n.from, n.to))
        } else if (name === 'QuoteMark') {
          const end = state.doc.sliceString(n.to, n.to + 1) === ' ' ? n.to + 1 : n.to
          decos.push(hide.range(n.from, end))
        }
      },
    })

    const text = state.doc.sliceString(from, to)
    for (const l of findLinks(text)) {
      const a = from + l.from
      const b = from + l.to
      if (isActive(a)) {
        decos.push(mark('cm-wikilink-raw').range(a, b))
      } else {
        const { text: label, broken } = linkLabel(l.id, l.alias, host.titleOf)
        decos.push(Decoration.replace({ widget: new LinkWidget(l.id, label, broken, host) }).range(a, b))
      }
    }
  }

  decos.sort((x, y) => x.from - y.from || x.value.startSide - y.value.startSide)
  const builder = new RangeSetBuilder<Decoration>()
  let lastTo = -1
  for (const d of decos) {
    // Replacements must not overlap; marks and line decorations may.
    const isReplace = (d.value as unknown as { point?: boolean }).point && d.to > d.from
    if (isReplace && d.from < lastTo) continue
    builder.add(d.from, d.to, d.value)
    if (isReplace) lastTo = d.to
  }
  return builder.finish()
}

export function livePreview(host: LinkHost): Extension {
  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet
      constructor(view: EditorView) {
        this.decorations = build(view, host)
      }
      update(u: ViewUpdate) {
        const refresh = u.transactions.some((t) => t.effects.some((e) => e.is(refreshLinks)))
        if (u.docChanged || u.selectionSet || u.viewportChanged || u.focusChanged || refresh) {
          this.decorations = build(u.view, host)
        }
      }
    },
    { decorations: (v) => v.decorations },
  )
  // ⌘+click on a raw link (cursor line) opens it too.
  const click = EditorView.domEventHandlers({
    mousedown(e, view) {
      if (!e.metaKey) return false
      const pos = view.posAtCoords({ x: e.clientX, y: e.clientY })
      if (pos == null) return false
      const l = view.state.doc.lineAt(pos)
      const hit = findLinks(l.text).find((x) => l.from + x.from <= pos && pos <= l.from + x.to)
      if (!hit || host.titleOf(hit.id) === undefined) return false
      e.preventDefault()
      host.open(hit.id)
      return true
    },
  })
  return [plugin, click]
}

export const previewTheme = EditorView.theme({
  '&': { color: 'var(--foreground)', backgroundColor: 'transparent', fontSize: '15px' },
  '&.cm-focused': { outline: 'none' },
  '.cm-scroller': { fontFamily: 'var(--font-sans)', lineHeight: '1.7', overflow: 'visible' },
  '.cm-content': { padding: '0 0 120px', caretColor: 'var(--color-secret)' },
  '.cm-line': { padding: '0' },
  '.cm-cursor': { borderLeftColor: 'var(--color-secret)', borderLeftWidth: '2px' },
  '.cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection': {
    backgroundColor: 'color-mix(in oklch, var(--color-secret) 25%, transparent) !important',
  },
  '.cm-h1': { fontSize: '1.75em', fontWeight: '600', lineHeight: '1.4', paddingTop: '0.4em' },
  '.cm-h2': { fontSize: '1.35em', fontWeight: '600', lineHeight: '1.45', paddingTop: '0.3em' },
  '.cm-h3': { fontSize: '1.15em', fontWeight: '600', paddingTop: '0.2em' },
  '.cm-strong': { fontWeight: '650' },
  '.cm-em': { fontStyle: 'italic' },
  '.cm-strike': { textDecoration: 'line-through', color: 'var(--muted-foreground)' },
  '.cm-inline-code': {
    fontFamily: 'var(--font-mono)',
    fontSize: '0.88em',
    backgroundColor: 'var(--muted)',
    borderRadius: '4px',
    padding: '1px 4px',
  },
  '.cm-quote': {
    borderLeft: '3px solid var(--border)',
    paddingLeft: '14px !important',
    color: 'var(--muted-foreground)',
  },
  '.cm-codeblock': {
    fontFamily: 'var(--font-mono)',
    fontSize: '13px',
    backgroundColor: 'var(--card)',
    paddingLeft: '12px !important',
  },
  '.cm-task': { margin: '0 6px 0 0', verticalAlign: 'middle', accentColor: 'var(--primary)', cursor: 'pointer' },
  '.cm-wikilink': {
    color: 'var(--primary-soft-fg)',
    background: 'var(--primary-soft)',
    borderRadius: '5px',
    padding: '1px 5px',
    fontWeight: '500',
    textDecoration: 'none',
    cursor: 'pointer',
  },
  '.cm-wikilink:hover': { filter: 'brightness(.95)' },
  '.cm-wikilink-broken': { color: 'var(--muted-foreground)', textDecoration: 'line-through', cursor: 'default' },
  '.cm-wikilink-raw': { color: 'var(--muted-foreground)', fontFamily: 'var(--font-mono)', fontSize: '0.85em' },
  '.cm-tooltip-autocomplete': {
    backgroundColor: 'var(--popover)',
    border: '1px solid var(--border)',
    borderRadius: '8px',
    overflow: 'hidden',
  },
  '.cm-tooltip-autocomplete ul li': { padding: '4px 10px !important', fontFamily: 'var(--font-sans)' },
  '.cm-tooltip-autocomplete ul li[aria-selected]': {
    backgroundColor: 'var(--accent) !important',
    color: 'var(--foreground) !important',
  },
  '.cm-completionDetail': { color: 'var(--muted-foreground)', fontStyle: 'normal', marginLeft: '8px' },
})
