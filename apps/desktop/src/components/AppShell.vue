<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useRoute, useRouter } from 'vue-router'
import AppSidebar from '@/components/AppSidebar.vue'
import CommandPalette from '@/components/CommandPalette.vue'
import { useLibrary } from '@/stores/library'
import { useSession } from '@/stores/session'

const lib = useLibrary()
const session = useSession()
const router = useRouter()
const route = useRoute()
const palette = ref(false)
const sidebar = ref<InstanceType<typeof AppSidebar> | null>(null)

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
}

async function onKey(e: KeyboardEvent) {
  if (!e.metaKey || e.altKey || e.ctrlKey || session.status.state !== 'unlocked') return
  const k = e.key.toLowerCase()
  const action = k === 'k' ? 'search' : k === 'n' ? (e.shiftKey ? 'new-folder' : 'new-record') : k === 'l' ? 'lock' : null
  if (!action) return
  e.preventDefault()
  await actions[action]!()
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

onMounted(() => {
  window.addEventListener('keydown', onKey)
  lib.refreshAll()
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKey)
  unlisten?.()
})
</script>

<template>
  <div class="flex h-full">
    <AppSidebar ref="sidebar" @search="palette = true" />
    <main class="min-w-0 flex-1">
      <RouterView :key="$route.fullPath" />
    </main>
    <CommandPalette v-model:open="palette" />
  </div>
</template>
