import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { RouteLocationNormalized, Router } from 'vue-router'
import { api } from '@/lib/api'

/** One tab: its own back/forward history of in-app paths. */
export interface Tab {
  id: string
  stack: string[]
  index: number
}

const MAX_HISTORY = 50
const STORAGE_KEY = 'crypt.tabs'
let seq = 0
const newId = () => `t${Date.now().toString(36)}${(seq++).toString(36)}`
const pathOf = (t: Tab) => t.stack[t.index] ?? '/'

/** Pages inside the app shell (not unlock / onboarding / tray). */
export const isShellRoute = (to: RouteLocationNormalized) => to.matched[0]?.path === '/'

export const useTabs = defineStore('tabs', () => {
  const tabs = ref<Tab[]>([])
  const activeId = ref('')
  const active = computed(() => tabs.value.find((t) => t.id === activeId.value))
  const canBack = computed(() => (active.value?.index ?? 0) > 0)
  const canForward = computed(() => !!active.value && active.value.index < active.value.stack.length - 1)

  let router: Router | null = null
  let persist = false
  let onLastClosed: (() => void) | null = null
  let stopHook: (() => void) | null = null
  // Paths we navigate to ourselves (tab switch, back/forward): not new history steps.
  const internal = new Map<string, number>()

  function save() {
    if (!persist) return
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ tabs: tabs.value, activeId: activeId.value }))
    } catch {
      /* storage unavailable: tabs just won't survive a restart */
    }
  }
  function load(): boolean {
    try {
      const raw = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null')
      const list = (raw?.tabs ?? []).filter(
        (t: Tab) =>
          typeof t?.id === 'string' &&
          Array.isArray(t.stack) &&
          t.stack.length > 0 &&
          t.stack.every((p) => typeof p === 'string' && p.startsWith('/')) &&
          Number.isInteger(t.index) &&
          t.index >= 0 &&
          t.index < t.stack.length,
      ) as Tab[]
      if (!list.length) return false
      tabs.value = list
      activeId.value = list.some((t) => t.id === raw.activeId) ? raw.activeId : list[0]!.id
      return true
    } catch {
      return false
    }
  }

  /** Show `path` in the current window without adding a history step. */
  function show(path: string) {
    if (!router) return
    internal.set(path, (internal.get(path) ?? 0) + 1)
    // Even when already there: this cancels a navigation still in flight.
    router
      .replace(path)
      .catch(() => {})
      .finally(() => {
        const n = (internal.get(path) ?? 1) - 1
        if (n > 0) internal.set(path, n)
        else internal.delete(path)
      })
  }

  /** Record a navigation that happened inside the active tab. */
  function visited(path: string, replaced: boolean) {
    if (internal.has(path)) return
    const t = active.value
    if (!t || pathOf(t) === path) return
    if (replaced) t.stack[t.index] = path
    else {
      t.stack = [...t.stack.slice(0, t.index + 1), path].slice(-MAX_HISTORY)
      t.index = t.stack.length - 1
    }
    save()
  }

  /**
   * Bind to the window's router. Restores this window's tabs (memory first,
   * then saved ones for the main window) and shows the active one.
   */
  function attach(r: Router, opts: { persist: boolean; onLastClosed?: () => void }) {
    router = r
    persist = opts.persist
    onLastClosed = opts.onLastClosed ?? null
    const current = r.currentRoute.value.fullPath
    if (!tabs.value.length && !(persist && load())) {
      const t = { id: newId(), stack: [current], index: 0 }
      tabs.value = [t]
      activeId.value = t.id
    }
    stopHook?.()
    stopHook = r.afterEach((to, _from, failure) => {
      if (failure || !isShellRoute(to)) return
      visited(to.fullPath, !!(window.history.state as { replaced?: boolean } | null)?.replaced)
    })
    if (active.value) show(pathOf(active.value))
  }
  function detach() {
    stopHook?.()
    stopHook = null
    internal.clear()
  }

  function activate(id: string) {
    const t = tabs.value.find((t) => t.id === id)
    if (!t) return
    activeId.value = id
    save()
    show(pathOf(t))
  }
  function back() {
    const t = active.value
    if (!t || t.index === 0) return
    t.index--
    save()
    show(pathOf(t))
  }
  function forward() {
    const t = active.value
    if (!t || t.index >= t.stack.length - 1) return
    t.index++
    save()
    show(pathOf(t))
  }
  /** New tab right after the active one. */
  function open(path = '/', opts: { background?: boolean } = {}) {
    const t: Tab = { id: newId(), stack: [path], index: 0 }
    const at = tabs.value.findIndex((x) => x.id === activeId.value)
    tabs.value.splice(at + 1, 0, t)
    if (opts.background) save()
    else activate(t.id)
    return t.id
  }
  function close(id: string) {
    const at = tabs.value.findIndex((t) => t.id === id)
    if (at < 0) return
    if (tabs.value.length === 1) {
      if (onLastClosed) return onLastClosed()
      // Main window keeps one tab, like an empty new tab.
      const t = tabs.value[0]!
      t.stack = ['/']
      t.index = 0
      save()
      return show('/')
    }
    tabs.value.splice(at, 1)
    if (id === activeId.value) activate(tabs.value[Math.min(at, tabs.value.length - 1)]!.id)
    else save()
  }
  function closeOthers(id: string) {
    tabs.value = tabs.value.filter((t) => t.id === id)
    activate(id)
  }
  /** Move the tab to its own window. */
  async function toWindow(id: string) {
    const t = tabs.value.find((t) => t.id === id)
    if (!t) return
    await api.openWindow(pathOf(t))
    if (tabs.value.length > 1) close(id)
  }
  function cycle(delta: number) {
    const n = tabs.value.length
    const at = tabs.value.findIndex((t) => t.id === activeId.value)
    if (n > 1) activate(tabs.value[(at + delta + n) % n]!.id)
  }
  /** ⌘1…⌘8 pick a tab, ⌘9 the last one (as in browsers). */
  function select(n: number) {
    const t = n === 9 ? tabs.value.at(-1) : tabs.value[n - 1]
    if (t) activate(t.id)
  }
  function move(id: string, to: number) {
    const from = tabs.value.findIndex((t) => t.id === id)
    if (from < 0 || from === to) return
    const [t] = tabs.value.splice(from, 1)
    tabs.value.splice(Math.min(to, tabs.value.length), 0, t!)
    save()
  }

  /** Open a link by modifiers: ⌘ → new tab, ⌘⌥ → new window, otherwise in place. */
  function follow(path: string, e?: Pick<MouseEvent, 'metaKey' | 'altKey'> & { button?: number }) {
    const newTab = !!e && (e.metaKey || e.button === 1)
    if (newTab && e.altKey) return api.openWindow(path)
    if (newTab) return open(path)
    return router?.push(path)
  }

  return {
    tabs,
    activeId,
    active,
    canBack,
    canForward,
    pathOf,
    attach,
    detach,
    activate,
    back,
    forward,
    open,
    close,
    closeOthers,
    toWindow,
    cycle,
    select,
    move,
    follow,
  }
})
