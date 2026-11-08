import type { MilkdownPlugin } from '@milkdown/kit/ctx'
import { $inputRule, $node, $prose, $remark } from '@milkdown/kit/utils'
import { InputRule } from '@milkdown/kit/prose/inputrules'
import { Plugin } from '@milkdown/kit/prose/state'
import type { Node as ProseNode } from '@milkdown/kit/prose/model'
import { remarkCryptLinks } from './wikilinkMarkdown'

export interface NoteLinks {
  titleOf(id: string): string | undefined
  open(id: string): void
}

export const wikilinkNode = $node('crypt_wikilink', () => ({
  inline: true,
  group: 'inline',
  atom: true,
  attrs: { id: { default: '' }, alias: { default: null } },
  parseDOM: [
    {
      tag: 'a[data-crypt-link]',
      getAttrs: (dom) => ({ id: dom.getAttribute('data-crypt-link'), alias: dom.getAttribute('data-alias') }),
    },
  ],
  toDOM: (node) => [
    'a',
    {
      'data-crypt-link': node.attrs.id,
      'data-alias': node.attrs.alias,
      href: `#/r/${node.attrs.id}`,
      class: 'crypt-wikilink',
      contenteditable: 'false',
    },
    node.attrs.alias || node.attrs.id,
  ],
  leafText: (node) => node.attrs.alias || node.attrs.id,
  parseMarkdown: {
    match: (node) => node.type === 'cryptWikilink',
    runner: (state, node, type) => {
      state.addNode(type, { id: node.id, alias: node.alias ?? null })
    },
  },
  toMarkdown: {
    match: (node) => node.type.name === 'crypt_wikilink',
    runner: (state, node) => {
      state.addNode('cryptWikilink', undefined, undefined, { id: node.attrs.id, alias: node.attrs.alias })
    },
  },
}))

export function refreshLinkLabels(root: HTMLElement, host: NoteLinks) {
  root.querySelectorAll<HTMLAnchorElement>('a[data-crypt-link]').forEach((link) => {
    const title = host.titleOf(link.dataset.cryptLink!)
    link.textContent = title === undefined ? link.dataset.alias || 'удалённая запись' : title || 'Без названия'
    link.classList.toggle('crypt-wikilink-broken', title === undefined)
    link.title = title === undefined ? 'Запись удалена' : 'Открыть запись'
  })
}

export function linkQuery(doc: ProseNode, pos: number) {
  const $pos = doc.resolve(pos)
  if (!$pos.parent.isTextblock || $pos.parent.type.spec.code) return null
  if ($pos.marks().some((mark) => mark.type.spec.code)) return null
  const before = $pos.parent.textBetween(0, $pos.parentOffset, '\n', '\ufffc')
  const match = /\[\[([^\]\n|\ufffc]*)$/.exec(before)
  return match ? { query: match[1]!, from: pos - match[0].length, to: pos } : null
}

export function cryptLinks(host: NoteLinks): MilkdownPlugin[] {
  const remark = $remark('crypt-wikilinks', () => remarkCryptLinks)
  const input = $inputRule(
    (ctx) =>
      new InputRule(/\[\[([0-7][0-9A-HJKMNP-TV-Z]{25})(?:\|([^\]\n]*))?\]\]$/, (state, match, start, end) => {
        if (state.selection.$from.parent.type.spec.code || state.selection.$from.marks().some((m) => m.type.spec.code))
          return null
        return state.tr.replaceWith(
          start,
          end,
          wikilinkNode.type(ctx).create({ id: match[1], alias: match[2] ?? null }),
        )
      }),
  )
  const navigation = $prose(
    () =>
      new Plugin({
        props: {
          handleClickOn: (_view, _pos, node, _nodePos, event) => {
            if (node.type.name !== 'crypt_wikilink' || event.button !== 0 || event.metaKey || event.ctrlKey)
              return false
            if (host.titleOf(node.attrs.id) !== undefined) host.open(node.attrs.id)
            return true
          },
        },
        view: (view) => {
          refreshLinkLabels(view.dom, host)
          return { update: (view) => refreshLinkLabels(view.dom, host) }
        },
      }),
  )
  return [...remark, wikilinkNode, input, navigation]
}
