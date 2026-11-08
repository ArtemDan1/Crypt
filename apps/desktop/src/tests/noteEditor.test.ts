import { afterEach, beforeAll, expect, test, vi } from 'vitest'
import { editorViewCtx } from '@milkdown/kit/core'
import { createNoteEditor } from '@/components/editor/noteEditor'

beforeAll(() => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  )
  vi.stubGlobal(
    'IntersectionObserver',
    class {
      private callback: IntersectionObserverCallback
      constructor(callback: IntersectionObserverCallback) {
        this.callback = callback
      }
      observe(target: Element) {
        this.callback(
          [{ isIntersecting: true, target } as IntersectionObserverEntry],
          this as unknown as IntersectionObserver,
        )
      }
      unobserve() {}
      disconnect() {}
    },
  )
  Range.prototype.getBoundingClientRect = () => new DOMRect()
  Range.prototype.getClientRects = () => [] as unknown as DOMRectList
})
let note: ReturnType<typeof createNoteEditor> | undefined
afterEach(async () => {
  await note?.destroy()
  document.body.replaceChildren()
})
function setup(markdown = '# Заметка\n\nТекст') {
  const root = document.createElement('div')
  document.body.append(root)
  const changed = vi.fn()
  note = createNoteEditor({ root, markdown, links: { titleOf: () => undefined, open: () => {} }, onChange: changed })
  return { note, changed, root }
}

test('opening a note does not normalize or save its Markdown', async () => {
  const { note, changed, root } = setup('Заголовок\n=========\n\n-   пункт\n')
  await note.create()
  expect(root.querySelector('h1')?.textContent).toBe('Заголовок')
  expect(changed).not.toHaveBeenCalled()
})

test('an edit emits Markdown immediately before navigating or locking', async () => {
  const { note, changed } = setup('Текст')
  await note.create()
  note.crepe.editor.action((ctx) => {
    const view = ctx.get(editorViewCtx)
    view.dispatch(view.state.tr.insertText(' новый', 6))
  })
  expect(changed).toHaveBeenCalledTimes(1)
  expect(changed.mock.calls[0]![0]).toContain('Текст новый')
})

test('external Markdown replacement updates the view without emitting an edit', async () => {
  const { note, changed, root } = setup('Старое')
  await note.create()
  note.replace('## Восстановлено\n\n- [x] Сделано')
  expect(root.querySelector('h2')?.textContent).toBe('Восстановлено')
  expect(changed).not.toHaveBeenCalled()
})

test('replacement during initialization uses the latest document', async () => {
  const { note, changed, root } = setup('Старое')
  const creating = note.create()
  note.replace('Новое')
  await creating
  expect(root.querySelector('.ProseMirror')?.textContent).toContain('Новое')
  expect(changed).not.toHaveBeenCalled()
})

test('unmount during initialization destroys the late editor and never emits', async () => {
  const { note, changed, root } = setup()
  const creating = note.create()
  await note.destroy()
  await creating
  expect(root.querySelector('.ProseMirror')).toBeNull()
  expect(changed).not.toHaveBeenCalled()
})

test('tables and language-aware code blocks are ready-made editable components', async () => {
  const { note, root } = setup('```js\nconst answer = 42\n```\n\n| A | B |\n| - | - |\n| 1 | 2 |')
  await note.create()
  expect(root.querySelector('.milkdown-code-block')).not.toBeNull()
  expect(root.querySelector('.cm-editor')).not.toBeNull()
  expect(root.querySelector('table')).not.toBeNull()
  expect(root.querySelector('.milkdown-top-bar')).toBeNull()
})
