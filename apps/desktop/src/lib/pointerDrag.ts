interface PointerDragOptions {
  onMove?(event: PointerEvent): void
  onDrop?(event: PointerEvent): void
  onEnd?(): void
}

// Native HTML drag events are consumed by Tauri's file-drop handler on macOS.
// Internal reordering uses the pointer stream so external file drops still work.
export function startPointerDrag(event: PointerEvent, options: PointerDragOptions): () => void {
  if (event.button !== 0 || event.isPrimary === false) return () => {}
  event.preventDefault()
  const capture = event.currentTarget instanceof Element ? event.currentTarget : null
  try {
    capture?.setPointerCapture(event.pointerId)
  } catch {
    /* A synthetic pointer has no active capture. */
  }
  let moved = false
  let ended = false
  const move = (next: PointerEvent) => {
    if (next.pointerId !== event.pointerId) return
    if (!moved && Math.hypot(next.clientX - event.clientX, next.clientY - event.clientY) < 5) return
    if (!moved) document.documentElement.classList.add('crypt-grabbing')
    moved = true
    next.preventDefault()
    options.onMove?.(next)
  }
  const finish = (next?: PointerEvent) => {
    if (ended || (next && next.pointerId !== event.pointerId)) return
    ended = true
    document.documentElement.classList.remove('crypt-grabbing')
    window.removeEventListener('pointermove', move, true)
    window.removeEventListener('pointerup', finish, true)
    window.removeEventListener('pointercancel', cancel, true)
    window.removeEventListener('keydown', key, true)
    window.removeEventListener('blur', cancel)
    try {
      if (capture?.hasPointerCapture(event.pointerId)) capture.releasePointerCapture(event.pointerId)
    } catch {
      /* The host may have already released it. */
    }
    try {
      if (next && moved) options.onDrop?.(next)
    } finally {
      options.onEnd?.()
    }
  }
  const cancel = () => finish()
  const key = (next: KeyboardEvent) => {
    if (next.key === 'Escape') {
      next.preventDefault()
      cancel()
    }
  }
  window.addEventListener('pointermove', move, { capture: true, passive: false })
  window.addEventListener('pointerup', finish, true)
  window.addEventListener('pointercancel', cancel, true)
  window.addEventListener('keydown', key, true)
  window.addEventListener('blur', cancel)
  return cancel
}

export function scrollWhileDragging(element: HTMLElement, y: number) {
  for (let parent = element.parentElement; parent; parent = parent.parentElement) {
    if (parent.scrollHeight <= parent.clientHeight || !/(auto|scroll)/.test(getComputedStyle(parent).overflowY))
      continue
    const rect = parent.getBoundingClientRect()
    if (y < rect.top + 40) parent.scrollTop -= 18
    else if (y > rect.bottom - 40) parent.scrollTop += 18
    return
  }
}
