/** Calls `onIdle` once after `delay` ms without `activity()`; only while started. */
export function idleTimer(onIdle: () => void, delay: number) {
  let timer: ReturnType<typeof setTimeout> | null = null
  let running = false
  const arm = () => {
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => {
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
      if (timer) clearTimeout(timer)
      timer = null
    },
    activity() {
      if (running) arm()
    },
    setDelay(ms: number) {
      delay = ms
      if (running) arm()
    },
  }
}
