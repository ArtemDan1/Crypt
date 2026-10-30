/** Saves only the latest value after `ms` of quiet; `flush()` saves now. */
export function debouncedSaver<T>(save: (v: T) => Promise<void>, ms: number) {
  let timer: ReturnType<typeof setTimeout> | null = null
  let latest: { v: T } | null = null
  let inflight: Promise<void> = Promise.resolve()

  async function run() {
    if (timer) clearTimeout(timer)
    timer = null
    if (!latest) return inflight
    const { v } = latest
    latest = null
    inflight = inflight.then(() => save(v))
    return inflight
  }

  return {
    push(v: T) {
      latest = { v }
      if (timer) clearTimeout(timer)
      timer = setTimeout(run, ms)
    },
    flush: run,
    pending: () => latest !== null,
  }
}
