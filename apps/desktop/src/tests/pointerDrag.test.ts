import { expect, test, vi } from 'vitest'
import { startPointerDrag } from '@/lib/pointerDrag'

const pointer = (type: string, y = 0, id = 1) =>
  Object.assign(new MouseEvent(type, { clientX: 20, clientY: y, button: 0, cancelable: true }), {
    pointerId: id,
    isPrimary: true,
  }) as PointerEvent

test('moving and releasing the primary pointer commits one drag without HTML drag events', () => {
  const move = vi.fn(),
    drop = vi.fn(),
    end = vi.fn()
  startPointerDrag(pointer('pointerdown'), { onMove: move, onDrop: drop, onEnd: end })
  window.dispatchEvent(pointer('pointermove', 30))
  window.dispatchEvent(pointer('pointerup', 40))
  window.dispatchEvent(pointer('pointerup', 50))
  expect(move).toHaveBeenCalledOnce()
  expect(drop).toHaveBeenCalledOnce()
  expect(drop.mock.calls[0]![0].clientY).toBe(40)
  expect(end).toHaveBeenCalledOnce()
})

test('clicks and other pointers never move an item', () => {
  const drop = vi.fn()
  startPointerDrag(pointer('pointerdown'), { onDrop: drop })
  window.dispatchEvent(pointer('pointermove', 50, 2))
  window.dispatchEvent(pointer('pointerup', 1))
  expect(drop).not.toHaveBeenCalled()
})

test.each(['pointercancel', 'blur', 'escape', 'dispose'])('%s cancels a drag and removes global handlers', (reason) => {
  const drop = vi.fn(),
    move = vi.fn(),
    end = vi.fn()
  const stop = startPointerDrag(pointer('pointerdown'), { onMove: move, onDrop: drop, onEnd: end })
  window.dispatchEvent(pointer('pointermove', 30))
  if (reason === 'dispose') stop()
  else if (reason === 'escape') window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
  else window.dispatchEvent(reason === 'blur' ? new Event('blur') : pointer(reason))
  window.dispatchEvent(pointer('pointermove', 80))
  window.dispatchEvent(pointer('pointerup', 80))
  expect(drop).not.toHaveBeenCalled()
  expect(move).toHaveBeenCalledOnce()
  expect(end).toHaveBeenCalledOnce()
})
