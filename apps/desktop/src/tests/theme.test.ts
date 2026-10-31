import { afterEach, beforeEach, expect, test } from 'vitest'
import { getThemePref, initTheme, resolvedTheme, setThemePref } from '@/lib/theme'

let systemDark = true
const listeners: Array<() => void> = []
let dispose: (() => void) | null = null

function setSystem(dark: boolean) {
  systemDark = dark
  listeners.forEach((l) => l())
}

beforeEach(() => {
  localStorage.clear()
  document.documentElement.classList.remove('dark')
  listeners.length = 0
  systemDark = true
  window.matchMedia = ((query: string) => ({
    get matches() {
      return query.includes('dark') ? systemDark : !systemDark
    },
    media: query,
    addEventListener: (_: string, l: () => void) => listeners.push(l),
    removeEventListener: (_: string, l: () => void) => {
      const i = listeners.indexOf(l)
      if (i >= 0) listeners.splice(i, 1)
    },
  })) as unknown as typeof window.matchMedia
})
afterEach(() => dispose?.())

const isDark = () => document.documentElement.classList.contains('dark')

test('follows the system when nothing is stored', () => {
  dispose = initTheme()
  expect(isDark()).toBe(true)
  setSystem(false)
  expect(isDark()).toBe(false)
  expect(resolvedTheme()).toBe('light')
})

test('explicit choice is stored and ignores system changes', () => {
  dispose = initTheme()
  setThemePref('light')
  expect(isDark()).toBe(false)
  expect(localStorage.getItem('crypt.theme')).toBe('light')
  setSystem(true)
  expect(isDark()).toBe(false)
})

test('switching back to system follows it again', () => {
  dispose = initTheme()
  setThemePref('light')
  setThemePref('system')
  expect(isDark()).toBe(true)
  setSystem(false)
  expect(isDark()).toBe(false)
})

test('garbage in storage means system', () => {
  localStorage.setItem('crypt.theme', 'purple')
  expect(getThemePref()).toBe('system')
})
