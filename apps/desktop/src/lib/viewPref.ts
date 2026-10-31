import { ref, watch, type Ref } from 'vue'

export type ViewMode = 'list' | 'grid'

/** A list/grid choice remembered per screen. */
export function useViewPref(key: string, initial: ViewMode): Ref<ViewMode> {
  const storageKey = `crypt.view.${key}`
  let start = initial
  try {
    const v = localStorage.getItem(storageKey)
    if (v === 'list' || v === 'grid') start = v
  } catch {}
  const mode = ref<ViewMode>(start)
  watch(mode, (v) => {
    try {
      localStorage.setItem(storageKey, v)
    } catch {}
  })
  return mode
}
