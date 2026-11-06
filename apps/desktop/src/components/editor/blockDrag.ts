import { $prose } from '@milkdown/kit/utils'
import { Plugin, NodeSelection, Selection, type EditorState } from '@milkdown/kit/prose/state'
import type { EditorView } from '@milkdown/kit/prose/view'
import type { DeriveContext } from '@milkdown/kit/plugin/block'
import { scrollWhileDragging, startPointerDrag } from '@/lib/pointerDrag'

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
  return tr
    .setSelection(
      NodeSelection.isSelectable(node)
        ? NodeSelection.create(tr.doc, destination)
        : Selection.near(tr.doc.resolve(destination)),
    )
    .setMeta('uiEvent', 'drop')
}

export function trackBlockPosition({ active, blockDom }: DeriveContext) {
  blockDom.dataset.cryptBlockFrom = String(active.$pos.pos)
  return active.el.getBoundingClientRect()
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

export const pointerBlockDrag = $prose(
  () =>
    new Plugin({
      view(view) {
        const root = view.dom.parentElement!
        let cancel: (() => void) | undefined
        let source: HTMLElement | undefined
        let indicator: HTMLDivElement | undefined
        const clear = () => {
          source?.classList.remove('crypt-block-dragging')
          source = undefined
          indicator?.remove()
          indicator = undefined
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
          source = element
          const targetAt = (clientY: number) => {
            const current = blocks(view, from)
            const next = current.find((block) => clientY < block.rect.top + block.rect.height / 2)
            return {
              pos: next?.from ?? view.state.doc.resolve(from).end(),
              y: next?.rect.top ?? current.at(-1)?.rect.bottom ?? clientY,
            }
          }
          cancel = startPointerDrag(event, {
            onMove: (next) => {
              scrollWhileDragging(view.dom, next.clientY)
              const target = targetAt(next.clientY)
              if (!indicator) {
                indicator = document.createElement('div')
                indicator.className = 'crypt-block-drop-line'
                document.body.append(indicator)
              }
              const editorRect = view.dom.getBoundingClientRect()
              indicator.style.cssText = `left:${editorRect.left}px;top:${target.y}px;width:${editorRect.width}px`
              source?.classList.add('crypt-block-dragging')
            },
            onDrop: (next) => {
              if (!view.state.doc.eq(original)) return
              const tr = moveEditorBlock(view.state, from, targetAt(next.clientY).pos)
              if (tr) {
                view.dispatch(tr.scrollIntoView())
                view.focus()
              }
            },
            onEnd: clear,
          })
        }
        root.addEventListener('pointerdown', pointerdown, true)
        return {
          update(next, previous) {
            if (!next.state.doc.eq(previous.doc)) cancel?.()
          },
          destroy() {
            cancel?.()
            clear()
            root.removeEventListener('pointerdown', pointerdown, true)
          },
        }
      },
    }),
)
