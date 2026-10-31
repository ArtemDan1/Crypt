/** Calls `onIdle` once after `delay` ms without `activity()`; only while started. */
export function idleTimer(onIdle: () => void, delay: number) {
  let timer: ReturnType<typeof setTimeout> | null = null
  let running = false
  let firesAt: number | null = null
  const arm = () => {
    if (timer) clearTimeout(timer)
    firesAt = Date.now() + delay
    timer = setTimeout(() => {
      firesAt = null
      running = false
      timer = null
      onIdle()
    }, delay)
  }
  return {
    start() {
      running = true
      arm()
    },
    stop() {
      running = false
      firesAt = null
      if (timer) clearTimeout(timer)
      timer = null
    },
    activity() {
      if (running) arm()
    },
    /** Milliseconds until it fires, or null when stopped. */
    remaining(): number | null {
      return running && firesAt !== null ? Math.max(0, firesAt - Date.now()) : null
    },
    setDelay(ms: number) {
      delay = ms
      if (running) arm()
    },
  }
}
