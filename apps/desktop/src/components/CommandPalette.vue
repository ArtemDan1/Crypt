<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useDebounceFn } from '@vueuse/core'
import { FilePlus, Lock, Plus, Search } from '@lucide/vue'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { api } from '@/lib/api'
import { typeIcon } from '@/lib/icons'
import type { SearchHit } from '@/lib/types'
import { useLibrary } from '@/stores/library'
import { useSession } from '@/stores/session'

const open = defineModel<boolean>('open', { required: true })
const router = useRouter()
const lib = useLibrary()
const session = useSession()

const query = ref('')
const hits = ref<SearchHit[]>([])
const active = ref(0)
const input = ref<HTMLInputElement | null>(null)

interface Item {
  key: string
  title: string
  sub?: string
  icon: unknown
  run: () => void | Promise<void>
}

const recent = computed(() => lib.records.filter((r) => !r.secret_kind).slice(0, 8))
const commands = computed<Item[]>(() => [
  { key: 'c:new', title: 'Новая запись', sub: '⌘N', icon: FilePlus, run: newRecord },
  { key: 'c:type', title: 'Новый тип', icon: Plus, run: () => go('/types?new=1') },
  { key: 'c:lock', title: 'Заблокировать', sub: '⌘L', icon: Lock, run: () => session.lock() },
])
const items = computed<Item[]>(() => {
  const q = query.value.trim()
  const records: Item[] = q
    ? hits.value.map((h) => ({
        key: h.id,
        title: h.title || 'Без названия',
        sub: h.snippet,
        icon: typeIcon(lib.typeById.get(lib.records.find((r) => r.id === h.id)?.type_id ?? '')?.icon),
        run: () => go(`/r/${h.id}`),
      }))
    : recent.value.map((r) => ({
        key: r.id,
        title: r.title || 'Без названия',
        icon: typeIcon(lib.typeById.get(r.type_id ?? '')?.icon),
        run: () => go(`/r/${r.id}`),
      }))
  const cmds = commands.value.filter((c) => !q || c.title.toLowerCase().includes(q.toLowerCase()))
  return [...records, ...cmds]
})

const runSearch = useDebounceFn(async (q: string) => {
  hits.value = q.trim() ? await api.search(q, 20).catch(() => []) : []
  active.value = 0
}, 150)
watch(query, (q) => runSearch(q))
watch(open, async (o) => {
  if (!o) return
  query.value = ''
  hits.value = []
  active.value = 0
  await nextTick()
  input.value?.focus()
})

function go(path: string) {
  open.value = false
  router.push(path)
}
async function newRecord() {
  open.value = false
  const id = await lib.createRecord({ title: query.value.trim() })
  router.push(`/r/${id}`)
}
function onKey(e: KeyboardEvent) {
  const n = items.value.length
  if (e.key === 'ArrowDown') {
    active.value = (active.value + 1) % Math.max(n, 1)
    e.preventDefault()
  } else if (e.key === 'ArrowUp') {
    active.value = (active.value - 1 + n) % Math.max(n, 1)
    e.preventDefault()
  } else if (e.key === 'Enter') {
    items.value[active.value]?.run()
    e.preventDefault()
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="top-[18%] translate-y-0 gap-0 overflow-hidden p-0 sm:max-w-[560px]"
      :show-close-button="false"
    >
      <DialogTitle class="sr-only">Поиск</DialogTitle>
      <DialogDescription class="sr-only">Поиск по записям и командам</DialogDescription>
      <label class="flex h-12 items-center gap-2.5 border-b px-4">
        <Search class="text-muted-foreground size-4" />
        <input
          ref="input"
          v-model="query"
          placeholder="Найти запись или команду…"
          class="flex-1 bg-transparent text-[15px] outline-none"
          aria-label="Поиск"
          @keydown="onKey"
        />
      </label>
      <div class="max-h-[380px] overflow-auto p-1.5" role="listbox">
        <div
          v-if="!query.trim() && recent.length"
          class="text-muted-foreground px-2.5 pt-1.5 pb-1 text-[11px] font-medium tracking-wider uppercase"
        >
          Недавние
        </div>
        <button
          v-for="(it, i) in items"
          :key="it.key"
          role="option"
          :aria-selected="i === active"
          class="flex w-full items-start gap-3 rounded-md px-2.5 py-2 text-left"
          :class="i === active ? 'bg-accent' : ''"
          @mousemove="active = i"
          @click="it.run()"
        >
          <component :is="it.icon" class="text-muted-foreground mt-0.5 size-4 shrink-0" />
          <span class="min-w-0 flex-1">
            <span class="block truncate">{{ it.title }}</span>
            <span v-if="it.sub && it.key.length > 10" class="text-muted-foreground block truncate text-xs">{{
              it.sub
            }}</span>
          </span>
          <span v-if="it.sub && it.key.startsWith('c:')" class="text-muted-foreground text-xs">{{ it.sub }}</span>
        </button>
        <p v-if="query.trim() && !hits.length" class="text-muted-foreground px-2.5 py-3 text-[13px]">
          Ничего не найдено. Enter на «Новая запись» создаст «{{ query.trim() }}».
        </p>
      </div>
    </DialogContent>
  </Dialog>
</template>
