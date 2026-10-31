export type ThemePref = 'system' | 'light' | 'dark'

const KEY = 'crypt.theme'
const EVENT = 'crypt-theme'

export function getThemePref(): ThemePref {
  try {
    const v = localStorage.getItem(KEY)
    if (v === 'light' || v === 'dark') return v
  } catch {}
  return 'system'
}

const systemDark = () => window.matchMedia('(prefers-color-scheme: dark)').matches

export function resolvedTheme(): 'light' | 'dark' {
  const p = getThemePref()
  if (p !== 'system') return p
  return systemDark() ? 'dark' : 'light'
}

function apply() {
  const dark = resolvedTheme() === 'dark'
  document.documentElement.classList.toggle('dark', dark)
  window.dispatchEvent(new CustomEvent(EVENT, { detail: dark ? 'dark' : 'light' }))
}

export function setThemePref(p: ThemePref) {
  try {
    if (p === 'system') localStorage.removeItem(KEY)
    else localStorage.setItem(KEY, p)
  } catch {}
  apply()
}

/** Applies the stored choice and follows the system while "system" is chosen. Returns a disposer. */
export function initTheme(): () => void {
  const mq = window.matchMedia('(prefers-color-scheme: dark)')
  const onChange = () => {
    if (getThemePref() === 'system') apply()
  }
  apply()
  mq.addEventListener('change', onChange)
  return () => mq.removeEventListener('change', onChange)
}

/** Subscribe to theme changes (for components that need the resolved value). */
export function onThemeChange(cb: (t: 'light' | 'dark') => void): () => void {
  const h = (e: Event) => cb((e as CustomEvent).detail)
  window.addEventListener(EVENT, h)
  return () => window.removeEventListener(EVENT, h)
}
