import { expect, test } from 'vitest'
import { Schema } from '@milkdown/kit/prose/model'
import { EditorState } from '@milkdown/kit/prose/state'
import { moveEditorBlock } from '@/components/editor/blockDrag'

const schema = new Schema({
  nodes: {
    doc: { content: 'block+' },
    paragraph: { content: 'text*', group: 'block' },
    code_block: { content: 'text*', group: 'block', code: true },
    text: {},
    bullet_list: { content: 'list_item+', group: 'block' },
    list_item: { content: 'paragraph+', attrs: { checked: { default: null } } },
  },
})
const state = () =>
  EditorState.create({
    schema,
    doc: schema.node('doc', null, [
      schema.node('paragraph', null, schema.text('first')),
      schema.node('code_block', null, schema.text('code')),
      schema.node('paragraph', null, schema.text('last')),
    ]),
  })

test('moving a block below the final paragraph preserves its type and all content', () => {
  const before = state()
  const from = before.doc.child(0).nodeSize
  const tr = moveEditorBlock(before, from, before.doc.content.size)!
  expect(tr.doc.child(0).textContent).toBe('first')
  expect(tr.doc.child(1).textContent).toBe('last')
  expect(tr.doc.child(2).type.name).toBe('code_block')
  expect(tr.doc.child(2).textContent).toBe('code')
})

test('moving the last block to the top preserves the intervening code', () => {
  const before = state()
  const from = before.doc.child(0).nodeSize + before.doc.child(1).nodeSize
  const tr = moveEditorBlock(before, from, 0)!
  expect(tr.doc.child(0).textContent).toBe('last')
  expect(tr.doc.child(1).textContent).toBe('first')
  expect(tr.doc.child(2).textContent).toBe('code')
})

test('dropping on the same block or inside a paragraph does not alter the document', () => {
  const before = state()
  expect(moveEditorBlock(before, 0, 0)).toBeNull()
  expect(moveEditorBlock(before, 0, before.doc.child(0).nodeSize)).toBeNull()
  expect(moveEditorBlock(before, 0, 3)).toBeNull()
})

test('moving an individual task preserves the surrounding list and checked state', () => {
  const item = (text: string, checked: boolean) =>
    schema.node('list_item', { checked }, schema.node('paragraph', null, schema.text(text)))
  const items = [item('one', false), item('two', true), item('three', false)]
  const before = EditorState.create({ schema, doc: schema.node('doc', null, schema.node('bullet_list', null, items)) })
  const from = 1 + items[0]!.nodeSize
  const target = 1 + items.reduce((sum, node) => sum + node.nodeSize, 0)
  const tr = moveEditorBlock(before, from, target)!
  expect(tr).not.toBeNull()
  expect(tr.doc.childCount).toBe(1)
  const list = tr.doc.firstChild!
  expect(list.childCount).toBe(3)
  expect(list.child(0).textContent).toBe('one')
  expect(list.child(1).textContent).toBe('three')
  expect(list.child(2).textContent).toBe('two')
  expect(list.child(2).attrs.checked).toBe(true)
})
