import { nextTick, ref, type CSSProperties, type Ref } from 'vue'
import { scrollWhileDragging, startPointerDrag } from './pointerDrag'

interface Span {
  top: number
  height: number
}

const SETTLE_MS = 180
const EASE = 'cubic-bezier(.2,0,0,1)'

// Index the dragged item would take, judged by its center against the
// other items' centers as if the dragged one were already removed.
export function sortTarget(spans: Span[], from: number, center: number, gap: number) {
  const removed = spans[from]!.height + gap
  let target = 0
  spans.forEach((span, i) => {
    if (i === from) return
    const mid = span.top + span.height / 2 - (i > from ? removed : 0)
    if (mid < center) target++
  })
  return target
}

// How far item `i` slides to make room while `from` hovers over `to`.
export function sortShift(i: number, from: number, to: number, size: number) {
  if (from < to && i > from && i <= to) return -size
  if (from > to && i >= to && i < from) return size
  return 0
}

// Keep the dragged item's center within the list so it never drifts far
// away, yet a tall item can still pass a short one at either end.
export function clampDelta(spans: Span[], from: number, delta: number) {
  const span = spans[from]!,
    last = spans.at(-1)!
  const center = span.top + span.height / 2
  return Math.min(Math.max(delta, spans[0]!.top - center), last.top + last.height - center)
}

// Where the dragged item lands relative to its original place.
export function settleOffset(spans: Span[], from: number, to: number, gap: number) {
  let offset = 0
  if (to > from) for (let i = from + 1; i <= to; i++) offset += spans[i]!.height + gap
  else for (let i = to; i < from; i++) offset -= spans[i]!.height + gap
  return offset
}

function scrollParent(element: HTMLElement) {
  for (let parent = element.parentElement; parent; parent = parent.parentElement)
    if (/(auto|scroll)/.test(getComputedStyle(parent).overflowY)) return parent
  return null
}

interface SortableOptions {
  list: Ref<HTMLElement | null>
  itemSelector: string
  // Return false when the list changed under the pointer.
  canDrop?(): boolean
  onSort(from: number, to: number): void
}

// Pointer-driven list sorting: the dragged item follows the pointer and its
// neighbours slide aside; the data order changes only once it settles.
export function useSortable(options: SortableOptions) {
  const from = ref<number | null>(null)
  const to = ref(0)
  const dy = ref(0)
  const phase = ref<'idle' | 'drag' | 'settle' | 'commit'>('idle')
  let size = 0
  let stop: (() => void) | undefined
  let timer: ReturnType<typeof setTimeout> | undefined

  function reset() {
    from.value = null
    dy.value = 0
    phase.value = 'idle'
  }

  function settle(spans: Span[], gap: number, commit: boolean) {
    const start = from.value!
    const target = commit ? to.value : start
    phase.value = 'settle'
    to.value = target
    dy.value = settleOffset(spans, start, target, gap)
    timer = setTimeout(async () => {
      if (target !== start) {
        phase.value = 'commit'
        options.onSort(start, target)
        await nextTick()
        requestAnimationFrame(reset)
      } else reset()
    }, SETTLE_MS)
  }

  function start(event: PointerEvent, index: number) {
    if (event.button !== 0 || event.isPrimary === false || phase.value !== 'idle') return
    const list = options.list.value
    if (!list) return
    const rows = [...list.querySelectorAll<HTMLElement>(options.itemSelector)]
    if (!rows[index]) return
    const scroller = scrollParent(list)
    const startScroll = scroller?.scrollTop ?? 0
    const spans = rows.map((row) => {
      const rect = row.getBoundingClientRect()
      return { top: rect.top, height: rect.height }
    })
    const gap = spans.length > 1 ? Math.max(0, spans[1]!.top - spans[0]!.top - spans[0]!.height) : 0
    size = spans[index]!.height + gap
    let dropped = false
    stop = startPointerDrag(event, {
      onMove: (next) => {
        scrollWhileDragging(list, next.clientY)
        const delta = next.clientY - event.clientY + ((scroller?.scrollTop ?? 0) - startScroll)
        dy.value = clampDelta(spans, index, delta)
        from.value = index
        phase.value = 'drag'
        const span = spans[index]!
        to.value = sortTarget(spans, index, span.top + span.height / 2 + dy.value, gap)
      },
      onDrop: () => {
        dropped = options.canDrop?.() ?? true
      },
      onEnd: () => {
        stop = undefined
        if (from.value === null) return reset()
        settle(spans, gap, dropped)
      },
    })
  }

  function styleFor(i: number): CSSProperties | undefined {
    if (from.value === null || phase.value === 'idle') return undefined
    if (phase.value === 'commit') return { transition: 'none' }
    if (i === from.value) {
      const lifted = phase.value === 'drag'
      return {
        position: 'relative',
        zIndex: 20,
        transform: `translate3d(0, ${dy.value}px, 0) scale(${lifted ? 1.012 : 1})`,
        boxShadow: lifted ? '0 14px 34px -14px rgba(0,0,0,.45), 0 0 0 1px var(--border-strong)' : undefined,
        transition: lifted
          ? `box-shadow 160ms ${EASE}`
          : `transform ${SETTLE_MS}ms ${EASE}, box-shadow ${SETTLE_MS}ms ${EASE}`,
        cursor: 'grabbing',
      }
    }
    return {
      transform: `translate3d(0, ${sortShift(i, from.value, to.value, size)}px, 0)`,
      transition: `transform 200ms ${EASE}`,
      // Neighbours should not light up their hover controls mid-drag.
      pointerEvents: 'none',
    }
  }

  function cancel() {
    stop?.()
    if (timer) clearTimeout(timer)
    timer = undefined
    reset()
  }

  return { dragging: from, active: phase, start, styleFor, cancel }
}
