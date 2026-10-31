<script setup lang="ts">
import { computed, ref, onMounted, onBeforeUnmount, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import CryptMark from '@/components/crypt/CryptMark.vue'
import AutolockRing from '@/components/crypt/AutolockRing.vue'
import TotpCode from '@/components/secret/TotpCode.vue'
import { api } from '@/lib/api'
import type { RecordSummary, SecretOverview } from '@/lib/types'
import { useSession } from '@/stores/session'
const session = useSession()
const query = ref(''),
  error = ref(''),
  copied = ref('')
const records = ref<RecordSummary[]>([]),
  recent = ref<string[]>([])
const overviews = ref<Record<string, SecretOverview>>({})
const agent = ref(false)
let generation = 0,
  timer: ReturnType<typeof setTimeout> | undefined
const unlisteners: UnlistenFn[] = []
const visible = computed(() => {
  if (query.value.trim())
    return records.value
      .filter((r) =>
        `${r.title} ${Object.values(overviews.value[r.id]?.public ?? {}).join(' ')}`
          .toLowerCase()
          .includes(query.value.toLowerCase()),
      )
      .slice(0, 9)
  const ordered = recent.value
    .map((id) => records.value.find((r) => r.id === id))
    .filter((r): r is RecordSummary => !!r)
  return (ordered.length ? ordered : [...records.value].sort((a, b) => b.updated_at - a.updated_at)).slice(0, 6)
})
function clear() {
  ++generation
  records.value = []
  recent.value = []
  overviews.value = {}
  copied.value = ''
  query.value = ''
}
async function refresh() {
  clear()
  const seq = generation
  try {
    await session.refresh()
    if (session.status.state !== 'unlocked') return
    const [items, ids, status] = await Promise.all([
      api.listRecords({ secrets: true }),
      api.recentSecrets(),
      api.agentStatus(),
    ])
    const details = await Promise.all(items.map(async (r) => [r.id, await api.secretOverview(r.id)] as const))
    if (seq !== generation || session.status.state !== 'unlocked') return
    records.value = items
    recent.value = ids
    overviews.value = Object.fromEntries(details)
    agent.value = status.running
  } catch (e) {
    if (seq === generation) error.value = String(e)
  }
}
async function copy(r: RecordSummary) {
  error.value = ''
  try {
    await api.copySecret(
      r.id,
      r.secret_kind === 'sshkey'
        ? 'public_key'
        : r.secret_kind === 'token'
          ? 'token'
          : r.secret_kind === 'card'
            ? 'number'
            : r.secret_kind === 'note'
              ? 'text'
              : 'password',
    )
    copied.value = r.id
    clearTimeout(timer)
    timer = setTimeout(() => (copied.value = ''), 1500)
  } catch (e) {
    error.value = String(e)
  }
}
async function unlock() {
  try {
    await session.unlockBiometric()
    await refresh()
  } catch (e) {
    error.value = String(e)
  }
}
async function open() {
  await api.show()
  await api.trayHide()
}
function key(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    void api.trayHide()
    return
  }
  if (!e.metaKey) return
  const index = Number(e.key) - 1
  if (index >= 0 && index < 9 && visible.value[index]) {
    e.preventDefault()
    void copy(visible.value[index]!)
    return
  }
  if (['o', 'l', 'q'].includes(e.key.toLowerCase())) {
    e.preventDefault()
    if (e.key.toLowerCase() === 'o') void open()
    if (e.key.toLowerCase() === 'l') void session.lock()
    if (e.key.toLowerCase() === 'q') void api.appQuit()
  }
}
watch(
  () => session.status.state,
  (s) => {
    if (s !== 'unlocked') clear()
  },
)
const unhook = session.beforeLock(clear)
onMounted(async () => {
  window.addEventListener('keydown', key)
  for (const name of ['tray://shown', 'tray://refresh'])
    unlisteners.push(await listen(name, refresh).catch(() => () => {}))
  unlisteners.push(await listen('vault-locked', clear).catch(() => () => {}))
  await refresh()
})
onBeforeUnmount(() => {
  clear()
  unhook()
  clearTimeout(timer)
  unlisteners.forEach((f) => f())
  window.removeEventListener('keydown', key)
})
</script>
<template>
  <main class="flex h-full flex-col overflow-hidden rounded-[14px] border bg-card shadow-xl">
    <template v-if="session.status.state === 'unlocked'">
      <input
        v-model="query"
        aria-label="Поиск секретов"
        placeholder="Поиск секретов…"
        class="m-3 rounded-lg border bg-background px-3 py-2 text-sm"
      />
      <div class="flex-1 overflow-auto px-3">
        <p class="mb-2 text-xs text-muted-foreground">Быстро скопировать</p>
        <div v-for="(r, i) in visible" :key="r.id" class="mb-1">
          <button class="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left hover:bg-muted" @click="copy(r)">
            <span class="min-w-0 flex-1 truncate text-sm"
              >{{ copied === r.id ? 'Скопировано' : r.title
              }}<span class="block truncate text-xs text-muted-foreground">{{
                overviews[r.id]?.public.username
              }}</span></span
            ><kbd class="text-xs text-muted-foreground">⌘{{ i + 1 }}</kbd>
          </button>
          <TotpCode v-if="overviews[r.id]?.hasTotp" :record-id="r.id" compact />
        </div>
        <p v-if="!visible.length" class="py-4 text-sm text-muted-foreground">Секретов пока нет</p>
      </div>
      <div class="flex items-center gap-2 border-t px-4 py-2 text-xs text-muted-foreground">
        <span>ssh-agent: {{ agent ? 'работает' : 'выключен' }}</span
        ><span class="relative ml-auto flex size-7 items-center justify-center"
          ><AutolockRing :left="session.autolockLeft" :total="session.autolockTotal" /><span class="text-[9px]"
            >{{ Math.ceil((session.autolockLeft ?? 0) / 60000) }}м</span
          ></span
        >
      </div>
    </template>
    <div v-else class="flex flex-1 flex-col items-center justify-center gap-4 p-6">
      <CryptMark :size="60" />
      <p>Хранилище заблокировано</p>
      <button class="rounded-lg bg-primary px-4 py-2 text-primary-foreground" @click="unlock">Войти с Touch ID</button>
    </div>
    <p v-if="error" role="alert" class="px-4 text-xs text-destructive">{{ error }}</p>
    <footer class="space-y-1 border-t p-2 text-sm">
      <button class="flex w-full justify-between rounded px-2 py-1 hover:bg-muted" @click="open">
        Открыть Crypt <kbd>⌘O</kbd></button
      ><button
        v-if="session.status.state === 'unlocked'"
        class="flex w-full justify-between rounded px-2 py-1 hover:bg-muted"
        @click="session.lock"
      >
        Заблокировать <kbd>⌘L</kbd></button
      ><button class="flex w-full justify-between rounded px-2 py-1 hover:bg-muted" @click="api.appQuit">
        Выйти <kbd>⌘Q</kbd>
      </button>
    </footer>
  </main>
</template>
