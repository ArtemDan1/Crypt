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
      class="top-[16%] translate-y-0 gap-0 overflow-hidden rounded-[14px] p-0 shadow-[0_30px_80px_-20px_rgba(0,0,0,.45)] sm:max-w-[600px]"
      :show-close-button="false"
    >
      <DialogTitle class="sr-only">Поиск</DialogTitle>
      <DialogDescription class="sr-only">Поиск по записям и командам</DialogDescription>
      <label class="text-muted-foreground flex h-[54px] items-center gap-3 border-b px-[18px]">
        <Search class="size-[18px]" />
        <input
          ref="input"
          v-model="query"
          placeholder="Найти запись или команду…"
          class="text-foreground placeholder:text-muted-foreground flex-1 bg-transparent text-[15px] outline-none"
          aria-label="Поиск"
          @keydown="onKey"
        />
        <kbd class="rounded border px-1.5 font-mono text-[11px]">esc</kbd>
      </label>
      <div class="max-h-[400px] overflow-auto p-2" role="listbox">
        <template v-for="(it, i) in items" :key="it.key">
          <div
            v-if="i === 0 && !it.key.startsWith('c:')"
            class="text-muted-foreground px-2.5 pt-1.5 pb-1.5 text-[11px] font-semibold tracking-wider uppercase"
          >
            {{ query.trim() ? 'Записи' : 'Недавние' }}
          </div>
          <div
            v-if="it.key.startsWith('c:') && (i === 0 || !items[i - 1]!.key.startsWith('c:'))"
            class="text-muted-foreground px-2.5 pt-2.5 pb-1.5 text-[11px] font-semibold tracking-wider uppercase"
          >
            Команды
          </div>
          <button
            role="option"
            :aria-selected="i === active"
            class="flex min-h-10 w-full items-center gap-3 rounded-lg px-2.5 py-1.5 text-left transition-colors"
            :class="i === active ? 'bg-accent' : ''"
            @mousemove="active = i"
            @click="it.run()"
          >
            <component :is="it.icon" class="text-muted-foreground size-4 shrink-0" />
            <span class="min-w-0 flex-1">
              <span class="block truncate text-[14px]">{{ it.title }}</span>
              <span v-if="it.sub && !it.key.startsWith('c:')" class="text-muted-foreground block truncate text-xs">{{
                it.sub
              }}</span>
            </span>
            <span v-if="it.sub && it.key.startsWith('c:')" class="text-muted-foreground font-mono text-xs">{{
              it.sub
            }}</span>
          </button>
        </template>
        <p v-if="query.trim() && !hits.length" class="text-muted-foreground px-2.5 py-3 text-[13px]">
          Ничего не найдено. Enter на «Новая запись» создаст «{{ query.trim() }}».
        </p>
      </div>
      <div class="text-muted-foreground flex gap-4 border-t px-[18px] py-2.5 font-mono text-xs">
        <span>↑↓ выбрать</span><span>↵ открыть</span><span>esc закрыть</span>
      </div>
    </DialogContent>
  </Dialog>
</template>
