<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useRoute, useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import AppSidebar from '@/components/AppSidebar.vue'
import CommandPalette from '@/components/CommandPalette.vue'
import TabBar from '@/components/TabBar.vue'
import UpdateBanner from '@/components/update/UpdateBanner.vue'
import { api } from '@/lib/api'
import { useLibrary } from '@/stores/library'
import { useSession } from '@/stores/session'
import { useTabs } from '@/stores/tabs'
import { useUpdates } from '@/stores/updates'

const lib = useLibrary()
const session = useSession()
const router = useRouter()
const route = useRoute()
const palette = ref(false)
const sidebar = ref<InstanceType<typeof AppSidebar> | null>(null)
const tabs = useTabs()

// The main window hides on close; windows opened from tabs really close.
const win = (() => {
  try {
    return getCurrentWindow()
  } catch {
    return null // browser preview
  }
})()
const isMain = (win?.label ?? 'main') === 'main'
async function closeWindow() {
  if (!win) return
  if (!isMain) await session.flushAll()
  await win.close()
}
async function newWindow(path = '/') {
  try {
    await api.openWindow(path)
  } catch {
    toast.error('Не удалось открыть окно')
  }
}

async function newRecord() {
  const id = await lib.createRecord({
    folder: route.path.startsWith('/folder/') ? String(route.params.folderId) : null,
  })
  router.push(`/r/${id}`)
}

// Same actions from keyboard and from the native menu bar.
const actions: Record<string, () => unknown> = {
  search: () => (palette.value = !palette.value),
  'new-record': newRecord,
  'new-folder': () => sidebar.value?.newFolder(),
  settings: () => router.push('/settings'),
  lock: () => session.lock(),
  'new-tab': () => tabs.open(),
  'new-window': () => newWindow(),
  'close-tab': () => tabs.close(tabs.activeId),
  'close-window': closeWindow,
  back: () => tabs.back(),
  forward: () => tabs.forward(),
  'next-tab': () => tabs.cycle(1),
  'prev-tab': () => tabs.cycle(-1),
}

/** Shortcut → action; `code` keeps ⌥ combos layout-independent. */
function shortcut(e: KeyboardEvent): string | null {
  if (e.ctrlKey && !e.metaKey && e.code === 'Tab') return e.shiftKey ? 'prev-tab' : 'next-tab'
  if (!e.metaKey || e.ctrlKey) return null
  if (e.altKey) {
    if (e.code === 'KeyN') return 'new-window'
    if (e.code === 'ArrowLeft') return 'back'
    if (e.code === 'ArrowRight') return 'forward'
    return null
  }
  switch (e.code) {
    case 'KeyK':
      return 'search'
    case 'KeyN':
      return e.shiftKey ? 'new-folder' : 'new-record'
    case 'KeyL':
      return 'lock'
    case 'KeyT':
      return e.shiftKey ? null : 'new-tab'
    case 'KeyW':
      return e.shiftKey ? 'close-window' : 'close-tab'
    case 'BracketLeft':
      return e.shiftKey ? 'prev-tab' : 'back'
    case 'BracketRight':
      return e.shiftKey ? 'next-tab' : 'forward'
  }
  return null
}

async function onKey(e: KeyboardEvent) {
  // The editor uses ⌘[ / ⌘] for indent: it marks those handled.
  if (e.defaultPrevented || session.status.state !== 'unlocked') return
  const digit = /^Digit([1-9])$/.exec(e.code)
  if (digit && e.metaKey && !e.altKey && !e.ctrlKey && !e.shiftKey) {
    e.preventDefault()
    return tabs.select(Number(digit[1]))
  }
  const action = shortcut(e)
  if (!action) return
  e.preventDefault()
  await actions[action]!()
}

// ⌘-click or middle click on any in-app link → new tab; ⌘⌥-click → new window.
function onLinkClick(e: MouseEvent) {
  if (e.defaultPrevented) return
  const viaMiddle = e.type === 'auxclick' && e.button === 1
  if (!viaMiddle && !(e.type === 'click' && e.metaKey)) return
  const a = (e.target as Element | null)?.closest?.('a[href^="#/"]')
  if (!a) return
  e.preventDefault()
  e.stopPropagation()
  const path = a.getAttribute('href')!.slice(1)
  if (e.altKey) newWindow(path)
  else tabs.open(path)
}
// Mouse side buttons.
function onMouseUp(e: MouseEvent) {
  if (e.button === 3) tabs.back()
  else if (e.button === 4) tabs.forward()
}

let unlisten: UnlistenFn | undefined
listen<string>('menu-action', (e) => {
  if (session.status.state === 'unlocked') actions[e.payload]?.()
})
  .then((u) => (unlisten = u))
  .catch(() => {})

watch(
  () => session.status.state,
  (s) => {
    if (s !== 'unlocked') {
      lib.clear()
      router.replace(s === 'noVault' ? '/onboarding' : '/unlock')
    }
  },
)

let unlistenClose: UnlistenFn | undefined
onMounted(() => {
  tabs.attach(router, { persist: isMain, onLastClosed: isMain ? undefined : closeWindow })
  window.addEventListener('keydown', onKey)
  window.addEventListener('click', onLinkClick)
  window.addEventListener('auxclick', onLinkClick)
  window.addEventListener('mouseup', onMouseUp)
  // Extra windows save pending edits before they go away.
  if (win && !isMain)
    win
      .onCloseRequested(() => session.flushAll())
      .then((u) => (unlistenClose = u))
      .catch(() => {})
  lib.refreshAll()
  // Updates are offered in the main window only.
  if (isMain) useUpdates().checkSilently()
})
onBeforeUnmount(() => {
  tabs.detach()
  window.removeEventListener('keydown', onKey)
  window.removeEventListener('click', onLinkClick)
  window.removeEventListener('auxclick', onLinkClick)
  window.removeEventListener('mouseup', onMouseUp)
  unlisten?.()
  unlistenClose?.()
})
</script>

<template>
  <div class="flex h-full">
    <AppSidebar ref="sidebar" @search="palette = true" />
    <main class="flex min-w-0 flex-1 flex-col">
      <TabBar />
      <div class="min-h-0 flex-1">
        <RouterView :key="$route.fullPath" />
      </div>
    </main>
    <CommandPalette v-model:open="palette" />
    <UpdateBanner v-if="isMain" />
  </div>
</template>
