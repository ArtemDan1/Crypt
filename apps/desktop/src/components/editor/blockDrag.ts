import { $prose } from '@milkdown/kit/utils'
import { Plugin, PluginKey, Selection, type EditorState } from '@milkdown/kit/prose/state'
import { Decoration, DecorationSet, type EditorView } from '@milkdown/kit/prose/view'
import type { DeriveContext } from '@milkdown/kit/plugin/block'
import { scrollWhileDragging, startPointerDrag } from '@/lib/pointerDrag'

// Six dots, drawn small so the handle stays quiet next to the text.
export const gripIcon = `<svg viewBox="0 0 10 16" xmlns="http://www.w3.org/2000/svg"><circle cx="2.5" cy="3" r="1.25"/><circle cx="7.5" cy="3" r="1.25"/><circle cx="2.5" cy="8" r="1.25"/><circle cx="7.5" cy="8" r="1.25"/><circle cx="2.5" cy="13" r="1.25"/><circle cx="7.5" cy="13" r="1.25"/></svg>`

export function moveEditorBlock(state: EditorState, from: number, target: number) {
  if (from < 0 || target < 0 || from > state.doc.content.size || target > state.doc.content.size) return null
  const $from = state.doc.resolve(from),
    $target = state.doc.resolve(target)
  if ($from.depth !== $target.depth || $from.start() !== $target.start()) return null
  const node = state.doc.nodeAt(from)
  if (!node || target === from || target === from + node.nodeSize) return null
  const to = from + node.nodeSize
  const destination = target > from ? target - node.nodeSize : target
  const tr = state.tr.delete(from, to).insert(destination, node)
  // Put the caret at the end of the moved block; selecting the whole node
  // would pop up the formatting toolbar over it.
  return tr.setSelection(Selection.near(tr.doc.resolve(destination + node.nodeSize - 1), -1)).setMeta('uiEvent', 'drop')
}

// The first rendered line of a block, so the handle sits beside it rather
// than in the middle of a tall paragraph or list.
function firstLine(element: HTMLElement) {
  const rect = element.getBoundingClientRect()
  let line: DOMRect | undefined
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) => (node.textContent?.trim() ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_SKIP),
  })
  const text = walker.nextNode()
  if (text && typeof document.createRange === 'function') {
    const range = document.createRange()
    range.setStart(text, 0)
    range.setEnd(text, 1)
    line = range.getClientRects?.()[0]
  }
  const center = line && line.height > 0 ? line.top + line.height / 2 : rect.top + Math.min(rect.height, 26) / 2
  return {
    x: rect.left,
    y: center - 12,
    left: rect.left,
    right: rect.right,
    width: rect.width,
    top: center - 12,
    bottom: center + 12,
    height: 24,
  }
}

export function trackBlockPosition({ active, blockDom }: DeriveContext) {
  blockDom.dataset.cryptBlockFrom = String(active.$pos.pos)
  return firstLine(active.el)
}

function blocks(view: EditorView, from: number) {
  const result: { from: number; element: HTMLElement; rect: DOMRect }[] = []
  const $from = view.state.doc.resolve(from)
  const start = $from.start()
  $from.parent.forEach((_node, offset) => {
    const position = start + offset
    const element = view.nodeDOM(position)
    if (element instanceof HTMLElement) result.push({ from: position, element, rect: element.getBoundingClientRect() })
  })
  return result
}

// A read-only copy of the block that follows the pointer. It reuses the
// editor's classes so it looks the same as the original.
function createGhost(element: HTMLElement, rect: DOMRect) {
  const ghost = document.createElement('div')
  ghost.className = 'crypt-note-editor crypt-block-ghost'
  ghost.setAttribute('aria-hidden', 'true')
  const milkdown = document.createElement('div')
  milkdown.className = 'milkdown'
  const prose = document.createElement('div')
  prose.className = 'ProseMirror'
  const copy = element.cloneNode(true) as HTMLElement
  copy.querySelectorAll('[contenteditable]').forEach((node) => node.removeAttribute('contenteditable'))
  copy.style.margin = '0'
  // A list item only renders correctly inside its list.
  let content: HTMLElement = copy
  if (element.parentElement && element.tagName === 'LI') {
    content = element.parentElement.cloneNode(false) as HTMLElement
    content.style.margin = '0'
    content.append(copy)
  }
  prose.append(content)
  milkdown.append(prose)
  ghost.append(milkdown)
  ghost.style.width = `${rect.width}px`
  document.body.append(ghost)
  return ghost
}

const dragKey = new PluginKey<number | null>('crypt-block-drag')

export const pointerBlockDrag = $prose(
  () =>
    new Plugin<number | null>({
      key: dragKey,
      state: {
        init: () => null,
        apply: (tr, value) => {
          const meta = tr.getMeta(dragKey) as number | null | undefined
          if (meta !== undefined) return meta
          return value === null ? null : tr.mapping.map(value)
        },
      },
      props: {
        // The block being moved fades out in place.
        decorations(state) {
          const from = dragKey.getState(state)
          const node = from === null || from === undefined ? null : state.doc.nodeAt(from)
          if (from === null || from === undefined || !node) return null
          return DecorationSet.create(state.doc, [
            Decoration.node(from, from + node.nodeSize, { class: 'crypt-block-dragging' }),
          ])
        },
      },
      view(view) {
        const root = view.dom.parentElement!
        let cancel: (() => void) | undefined
        let indicator: HTMLDivElement | undefined
        let ghost: HTMLDivElement | undefined
        const clear = () => {
          root.classList.remove('crypt-block-drag-active')
          indicator?.remove()
          indicator = undefined
          ghost?.remove()
          ghost = undefined
          cancel = undefined
          if (dragKey.getState(view.state) !== null) view.dispatch(view.state.tr.setMeta(dragKey, null))
        }
        const pointerdown = (event: PointerEvent) => {
          if (event.button !== 0 || event.isPrimary === false || !view.editable) return
          const handle = (event.target as Element).closest<HTMLElement>(
            '.milkdown-block-handle .operation-item:last-child',
          )
          if (!handle) return
          const from = Number(handle.closest<HTMLElement>('.milkdown-block-handle')?.dataset.cryptBlockFrom)
          if (!Number.isInteger(from) || from < 0 || from >= view.state.doc.content.size) return
          const element = view.nodeDOM(from)
          if (!(element instanceof HTMLElement)) return
          cancel?.()
          event.stopImmediatePropagation()
          const original = view.state.doc
          const origin = element.getBoundingClientRect()
          const targetAt = (clientY: number) => {
            const current = blocks(view, from)
            const index = current.findIndex((block) => clientY < block.rect.top + block.rect.height / 2)
            const next = index < 0 ? undefined : current[index]
            const previous = index < 0 ? current.at(-1) : current[index - 1]
            // Draw the line in the middle of the gap between two blocks.
            const y =
              next && previous
                ? (previous.rect.bottom + next.rect.top) / 2
                : next
                  ? next.rect.top - 3
                  : (previous?.rect.bottom ?? clientY) + 3
            return {
              pos: next?.from ?? view.state.doc.resolve(from).end(),
              y,
              left: (next ?? previous)?.rect.left ?? origin.left,
              same: next?.from === from || previous?.from === from,
            }
          }
          let started = false
          cancel = startPointerDrag(event, {
            onMove: (next) => {
              if (!started) {
                started = true
                root.classList.add('crypt-block-drag-active')
                ghost = createGhost(element, origin)
                view.dispatch(view.state.tr.setMeta(dragKey, from))
              }
              scrollWhileDragging(view.dom, next.clientY)
              ghost!.style.transform = `translate3d(${origin.left}px, ${origin.top + next.clientY - event.clientY}px, 0)`
              const target = targetAt(next.clientY)
              indicator ??= Object.assign(document.createElement('div'), { className: 'crypt-block-drop-line' })
              if (!indicator.isConnected) document.body.append(indicator)
              const editorRect = view.dom.getBoundingClientRect()
              indicator.dataset.idle = String(target.same)
              indicator.style.cssText = `left:${target.left}px;top:${target.y}px;width:${editorRect.right - target.left}px`
            },
            onDrop: (next) => {
              if (!view.state.doc.eq(original)) return
              const tr = moveEditorBlock(view.state, from, targetAt(next.clientY).pos)
              if (!tr) return
              view.dispatch(tr.setMeta(dragKey, null).scrollIntoView())
              view.focus()
            },
            onEnd: clear,
          })
        }
        root.addEventListener('pointerdown', pointerdown, true)
        return {
          update(next, previous) {
            // Never dispatch from inside an update; finish the cancel afterwards.
            if (!next.state.doc.eq(previous.doc) && cancel) queueMicrotask(() => cancel?.())
          },
          destroy() {
            cancel?.()
            indicator?.remove()
            ghost?.remove()
            root.removeEventListener('pointerdown', pointerdown, true)
          },
        }
      },
    }),
)
