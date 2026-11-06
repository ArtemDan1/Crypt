import type { Tokenizer, State } from 'micromark-util-types'
import type { Extension as FromMarkdown } from 'mdast-util-from-markdown'
import type { Options as ToMarkdown } from 'mdast-util-to-markdown'
import type { RemarkPluginRaw } from '@milkdown/kit/transformer'

declare module 'micromark-util-types' {
  interface TokenTypeMap {
    cryptWikilink: 'cryptWikilink'
  }
}

interface WikiNode {
  type: 'cryptWikilink'
  id: string
  alias: string | null
}
declare module 'mdast' {
  interface PhrasingContentMap {
    cryptWikilink: WikiNode
  }
  interface RootContentMap {
    cryptWikilink: WikiNode
  }
}

// Recognize links before Markdown parses punctuation in aliases as formatting.
const tokenize: Tokenizer = (effects, ok, nok) => {
  let text = ''
  return start
  function start(code: number | null): State | undefined {
    if (code !== 91) return nok(code)
    effects.enter('cryptWikilink')
    effects.consume(code)
    return second
  }
  function second(code: number | null): State | undefined {
    if (code !== 91) return nok(code)
    effects.consume(code)
    return content
  }
  function content(code: number | null): State | undefined {
    if (code === null || code < 0 || code === 10 || code === 13) return nok(code)
    if (code === 93) {
      if (!/^[0-7][0-9A-HJKMNP-TV-Z]{25}(?:\\?\|[^\]\n]*)?$/.test(text)) return nok(code)
      effects.consume(code)
      return close
    }
    text += String.fromCharCode(code)
    effects.consume(code)
    return content
  }
  function close(code: number | null): State | undefined {
    if (code !== 93) return nok(code)
    effects.consume(code)
    effects.exit('cryptWikilink')
    return ok
  }
}

export const remarkCryptLinks: RemarkPluginRaw<Record<string, never>> = function () {
  const data = this.data()
  const from: FromMarkdown = {
    enter: {
      cryptWikilink(token) {
        this.enter({ type: 'cryptWikilink', id: '', alias: null }, token)
      },
    },
    exit: {
      cryptWikilink(token) {
        const raw = this.sliceSerialize(token).slice(2, -2).replace(/\\\|/g, '|')
        const split = raw.indexOf('|')
        const node = this.stack.at(-1) as WikiNode
        node.id = split < 0 ? raw : raw.slice(0, split)
        node.alias = split < 0 ? null : raw.slice(split + 1)
        this.exit(token)
      },
    },
  }
  const to: ToMarkdown = {
    handlers: {
      cryptWikilink(node, _parent, state) {
        const link = node as unknown as WikiNode
        let text = `[[${link.id}${link.alias == null ? '' : `|${link.alias}`}]]`
        // GFM splits table cells before inline parsing. Escape only in that context.
        if (state.stack.some((name) => String(name) === 'tableCell')) text = text.replaceAll('|', '\\|')
        return text
      },
    },
  }
  ;(data.micromarkExtensions ??= []).push({ text: { 91: { name: 'cryptWikilink', tokenize } } })
  ;(data.fromMarkdownExtensions ??= []).push(from)
  ;(data.toMarkdownExtensions ??= []).push(to)
}
