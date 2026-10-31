import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { listen } from '@tauri-apps/api/event'
import { api } from './lib/api'
import { initTheme } from './lib/theme'
import type { VaultStatus } from './lib/types'
import './styles.css'
import 'vue-sonner/style.css'
import App from './App.vue'
import { router } from './router'
import { useSession } from './stores/session'

// System theme by default; Settings can pin light or dark.
initTheme()
if (window.location.hash === '#/tray') document.documentElement.classList.add('tray-window')

// No browser menu (Reload / Inspect) outside text fields; our own menus call preventDefault first.
if (!import.meta.env.DEV)
  window.addEventListener('contextmenu', (e) => {
    const target = e.target as HTMLElement | null
    if (!e.defaultPrevented && !target?.closest('input, textarea, [contenteditable="true"]')) e.preventDefault()
  })

async function boot() {
  // Browser preview without Tauri: in-memory backend for UI work.
  if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
    await (await import('./dev/mockBackend')).install()
  }
  const app = createApp(App).use(createPinia())
  const session = useSession()
  await session.refresh().catch(() => {})
  app.use(router).mount('#app')

  await listen<{ left: number | null; total: number | null }>('session-countdown', (e) =>
    session.applyCountdown(e.payload),
  ).catch(() => {})
  await listen('session-activity', () => session.activity()).catch(() => {})
  await listen('vault-unlocked', async () => {
    await session.refresh()
    if (router.currentRoute.value.path === '/unlock') await router.replace('/')
  }).catch(() => {})

  // Sleep / screen lock: Rust asks us to save pending edits, then drops the key.
  if (window.location.hash !== '#/tray') {
    await listen<string>('vault-will-lock', async (e) => {
      await session.flushAll()
      await api.flushDone(e.payload).catch(() => {})
    }).catch(() => {})
  }
  await listen<VaultStatus>('vault-locked', (e) => {
    if (e.payload.state === 'locked') session.markLocked(e.payload.vaultId)
  }).catch(() => {})
}
boot()
