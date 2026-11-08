import { afterEach, expect, test } from 'vitest'
import { Editor, rootCtx, parserCtx, serializerCtx } from '@milkdown/kit/core'
import { commonmark } from '@milkdown/kit/preset/commonmark'
import { gfm } from '@milkdown/kit/preset/gfm'
import { cryptLinks } from '@/components/editor/milkdownLinks'

const id = '01J9ZK7Q3M4N5P6R7S8T9V0W1X'
let editor: Editor | undefined
afterEach(async () => {
  await editor?.destroy()
  document.body.replaceChildren()
})
async function setup() {
  const root = document.createElement('div')
  document.body.append(root)
  editor = await Editor.make()
    .config((ctx) => ctx.set(rootCtx, root))
    .use(commonmark)
    .use(gfm)
    .use(cryptLinks({ titleOf: () => 'Текущее имя', open: () => {} }))
    .create()
  return editor
}

test('existing ULID links preserve identifiers and aliases through Markdown serialization', async () => {
  const e = await setup()
  e.action((ctx) => {
    const parse = ctx.get(parserCtx)
    const serialize = ctx.get(serializerCtx)
    const doc = parse(`До [[${id}|Название]] и [[${id}]] после`)
    const links: unknown[] = []
    doc.descendants((node) => {
      if (node.type.name === 'crypt_wikilink') links.push(node.attrs)
    })
    expect(links).toEqual([
      { id, alias: 'Название' },
      { id, alias: null },
    ])
    expect(serialize(doc)).toContain(`[[${id}|Название]] и [[${id}]]`)
    expect(parse(serialize(doc)).eq(doc)).toBe(true)
  })
})

test('link-shaped text inside inline and fenced code remains literal code', async () => {
  const e = await setup()
  e.action((ctx) => {
    const source = `\`[[${id}|пример]]\`\n\n\`\`\`js\nconst link = "[[${id}]]"\n\`\`\``
    const doc = ctx.get(parserCtx)(source)
    let links = 0
    doc.descendants((node) => {
      if (node.type.name === 'crypt_wikilink') links++
    })
    expect(links).toBe(0)
    expect(ctx.get(serializerCtx)(doc)).toContain(`const link = "[[${id}]]"`)
  })
})

test('links in formatted text and table cells round-trip alongside tasks', async () => {
  const e = await setup()
  e.action((ctx) => {
    const parse = ctx.get(parserCtx)
    const source = `**[[${id}|Имя]]**\n\n- [x] Готово\n\n| Запись |\n| --- |\n| [[${id}]] |`
    const doc = parse(source)
    const serialized = ctx.get(serializerCtx)(doc)
    expect(serialized).toContain(`[[${id}|Имя]]`)
    expect(serialized).toContain(`[[${id}]]`)
    expect(serialized).toContain('[x]')
    expect(parse(serialized).eq(doc)).toBe(true)
  })
})

test('aliased links in table cells never split or discard neighbouring cells', async () => {
  const e = await setup()
  e.action((ctx) => {
    const parse = ctx.get(parserCtx)
    const doc = parse(`| Запись | Сосед |\n| --- | --- |\n| [[${id}\\|Имя]] | Не потерять |`)
    let count = 0
    doc.descendants((node) => {
      if (node.type.name === 'crypt_wikilink') count++
    })
    expect(count).toBe(1)
    const saved = ctx.get(serializerCtx)(doc)
    expect(saved).toContain('Не потерять')
    expect(parse(saved).eq(doc)).toBe(true)
  })
})

test('Markdown punctuation in record titles remains a literal wikilink alias', async () => {
  const e = await setup()
  e.action((ctx) => {
    const parse = ctx.get(parserCtx)
    const source = `Ссылка [[${id}|**Bold** и \`код\` <tag> _имя_]] конец`
    const doc = parse(source)
    const links: unknown[] = []
    doc.descendants((node) => {
      if (node.type.name === 'crypt_wikilink') links.push(node.attrs)
    })
    expect(links).toEqual([{ id, alias: '**Bold** и `код` <tag> _имя_' }])
    expect(parse(ctx.get(serializerCtx)(doc)).eq(doc)).toBe(true)
  })
})
