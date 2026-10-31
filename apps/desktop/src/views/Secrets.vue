<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useElementSize, useEventListener } from '@vueuse/core'
import {
  Check,
  CreditCard,
  Dices,
  Download,
  FileLock,
  Key,
  KeyRound,
  LockKeyhole,
  Plus,
  Search,
  Terminal,
  Trash2,
  X,
} from '@lucide/vue'
import { toast } from 'vue-sonner'
import RecordContextMenu from '@/components/RecordContextMenu.vue'
import KindTile from '@/components/crypt/KindTile.vue'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from '@/components/ui/dropdown-menu'
import { SECRET_KIND_LABEL } from '@/lib/secretFields'
import type { Id, RecordSummary, SecretKind } from '@/lib/types'
import { api } from '@/lib/api'
import { useLibrary } from '@/stores/library'
const lib = useLibrary(),
  router = useRouter(),
  query = ref(''),
  tab = ref<SecretKind | null>(null)
const kinds: SecretKind[] = ['login', 'sshkey', 'token', 'card', 'note']
const labels: Record<SecretKind, string> = {
  login: 'Логины',
  sshkey: 'SSH-ключи',
  token: 'Токены',
  card: 'Карты',
  note: 'Заметки',
}
const icons = { login: LockKeyhole, sshkey: Terminal, token: Key, card: CreditCard, note: FileLock }
const secrets = computed(() => lib.records.filter((r) => r.secret_kind))
const countByKind = computed(() => {
  const m = new Map<SecretKind, number>()
  for (const r of secrets.value) m.set(r.secret_kind!, (m.get(r.secret_kind!) ?? 0) + 1)
  return m
})
const filtered = computed(() => {
  const q = query.value.trim().toLocaleLowerCase('ru')
  return secrets.value.filter(
    (r) =>
      (!tab.value || r.secret_kind === tab.value) &&
      (!q || r.title.toLocaleLowerCase('ru').includes(q) || !!r.username?.toLocaleLowerCase('ru').includes(q)),
  )
})
const fmt = new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'short' })
const pill = 'bg-foreground/[.07] text-foreground-2 hover:bg-foreground/[.11]'
const folderNames = computed(() => new Map(lib.folders.map((f) => [f.id, f.name])))
/** Second line: who the login is for, otherwise the kind; then the folder. */
function subtitle(r: RecordSummary) {
  const head = r.username || SECRET_KIND_LABEL[r.secret_kind!]
  const folder = r.folder ? folderNames.value.get(r.folder) : undefined
  return folder ? `${head} · ${folder}` : head
}
async function create(kind: SecretKind) {
  try {
    const id = await api.createRecord({ title: `Новый секрет · ${SECRET_KIND_LABEL[kind]}`, secret_kind: kind })
    await lib.changed()
    await router.push(`/r/${id}`)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось создать секрет')
  }
}

// Virtual list: rows have a fixed height, only the visible window is rendered.
const ROW = 60
const OVERSCAN = 8
const scroller = ref<HTMLElement | null>(null)
const scrollTop = ref(0)
const { height: viewport } = useElementSize(scroller)
const range = computed(() => {
  const start = Math.max(0, Math.floor(scrollTop.value / ROW) - OVERSCAN)
  const end = Math.min(filtered.value.length, Math.ceil((scrollTop.value + viewport.value) / ROW) + OVERSCAN)
  return { start, end }
})
const visible = computed(() =>
  filtered.value.slice(range.value.start, range.value.end).map((r, k) => ({ r, i: range.value.start + k })),
)
watch([tab, query], async () => {
  scrollTop.value = 0
  await nextTick()
  if (scroller.value) scroller.value.scrollTop = 0
})

// Selection
const selected = ref(new Set<Id>())
const anchor = ref<number | null>(null)
const selecting = computed(() => selected.value.size > 0)
watch(filtered, (list) => {
  if (!selected.value.size) return
  const ids = new Set(list.map((r) => r.id))
  const next = new Set([...selected.value].filter((id) => ids.has(id)))
  if (next.size !== selected.value.size) selected.value = next
})
function toggle(i: number, range: boolean) {
  const next = new Set(selected.value)
  if (range && anchor.value !== null) {
    const [a, b] = [Math.min(anchor.value, i), Math.max(anchor.value, i)]
    for (const r of filtered.value.slice(a, b + 1)) next.add(r.id)
  } else {
    const id = filtered.value[i]!.id
    if (next.has(id)) next.delete(id)
    else next.add(id)
  }
  anchor.value = i
  selected.value = next
}
const selectAll = () => (selected.value = new Set(filtered.value.map((r) => r.id)))
function clearSelection() {
  selected.value = new Set()
  anchor.value = null
}
function onRowClick(e: MouseEvent, i: number, navigate: (e?: MouseEvent) => unknown) {
  if (selecting.value || e.shiftKey || e.metaKey || e.ctrlKey) {
    e.preventDefault()
    toggle(i, e.shiftKey)
  } else navigate(e)
}
useEventListener(window, 'keydown', (e: KeyboardEvent) => {
  const t = e.target as HTMLElement | null
  if (t?.closest('input, textarea, [contenteditable="true"], [role="dialog"]')) return
  if (e.key === 'Escape' && selecting.value) clearSelection()
  else if (e.key === 'a' && (e.metaKey || e.ctrlKey) && filtered.value.length) {
    e.preventDefault()
    selectAll()
  }
})

function plural(n: number, one: string, few: string, many: string) {
  const m10 = n % 10,
    m100 = n % 100
  if (m10 === 1 && m100 !== 11) return one
  if (m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14)) return few
  return many
}
const confirmDelete = ref(false)
const deleting = ref(false)
async function deleteSelected() {
  const ids = [...selected.value]
  deleting.value = true
  try {
    const [, purged] = await api.deleteRecords(ids)
    clearSelection()
    confirmDelete.value = false
    await lib.changed()
    toast(`Удалено: ${purged} ${plural(purged, 'секрет', 'секрета', 'секретов')}`)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось удалить')
  } finally {
    deleting.value = false
  }
}
</script>
<template>
  <div class="relative flex h-full flex-col">
    <header data-tauri-drag-region class="flex h-[52px] shrink-0 items-center gap-2 border-b pr-5 pl-7">
      <h1 class="flex-1 text-[17px] font-semibold tracking-[-.01em]">Секреты</h1>
      <Button variant="outline" size="sm" aria-label="Импорт" title="Импорт" @click="router.push('/import')"
        ><Download class="size-4" /> Импорт</Button
      >
      <Button variant="outline" size="sm" @click="router.push('/generator')"><Dices class="size-4" /> Генератор</Button>
      <DropdownMenu
        ><DropdownMenuTrigger as-child
          ><Button size="sm"><Plus class="size-4" /> Новый секрет</Button></DropdownMenuTrigger
        ><DropdownMenuContent align="end"
          ><DropdownMenuItem v-for="kind in kinds" :key="kind" @select="create(kind)"
            ><component :is="icons[kind]" class="text-secret size-4" />{{ SECRET_KIND_LABEL[kind] }}</DropdownMenuItem
          ></DropdownMenuContent
        ></DropdownMenu
      >
    </header>
    <div class="flex flex-col gap-3 px-7 pt-5 pb-3">
      <label
        class="bg-card text-muted-foreground focus-within:border-primary flex h-9 max-w-md items-center gap-2 rounded-lg border px-3 transition-colors"
      >
        <Search class="size-3.5" />
        <input
          v-model="query"
          placeholder="Найти по названию или логину…"
          aria-label="Поиск секретов"
          class="text-foreground w-full bg-transparent text-sm outline-none"
        />
      </label>
      <div class="flex flex-wrap gap-1" role="tablist" aria-label="Виды секретов">
        <button
          role="tab"
          :aria-selected="tab === null"
          class="flex h-[26px] items-center gap-1.5 rounded-full px-2.5 text-xs font-medium transition-colors active:scale-95"
          :class="tab === null ? 'bg-foreground text-background' : pill"
          @click="tab = null"
        >
          Все <span class="opacity-65">{{ secrets.length }}</span></button
        ><button
          v-for="kind in kinds"
          :key="kind"
          role="tab"
          :aria-selected="tab === kind"
          class="flex h-[26px] items-center gap-1.5 rounded-full px-2.5 text-xs font-medium transition-colors active:scale-95"
          :class="tab === kind ? 'bg-foreground text-background' : pill"
          @click="tab = kind"
        >
          {{ labels[kind] }} <span class="opacity-65">{{ countByKind.get(kind) ?? 0 }}</span>
        </button>
      </div>
    </div>
    <div
      ref="scroller"
      :key="tab ?? 'all'"
      class="anim-in flex-1 overflow-auto px-7"
      :class="selecting ? 'pb-24' : 'pb-8'"
      @scroll="scrollTop = ($event.target as HTMLElement).scrollTop"
    >
      <div
        v-if="filtered.length"
        class="bg-card relative overflow-hidden rounded-xl border"
        :style="{ height: `${filtered.length * ROW + 2}px` }"
      >
        <RecordContextMenu v-for="{ r, i } in visible" :key="r.id" :record="r">
          <div
            class="group/row absolute inset-x-0 flex items-center gap-3 px-3.5 transition-colors"
            :class="[i > 0 && 'border-t', selected.has(r.id) ? 'bg-primary/[.08]' : 'hover:bg-foreground/[.04]']"
            :style="{ top: `${i * ROW}px`, height: `${ROW}px` }"
          >
            <button
              type="button"
              role="checkbox"
              :aria-checked="selected.has(r.id)"
              :aria-label="`Выбрать ${r.title || 'Без названия'}`"
              class="focus-visible:ring-ring/50 relative shrink-0 rounded-[9px] outline-none focus-visible:ring-3"
              @click="toggle(i, $event.shiftKey)"
            >
              <KindTile
                :icon="icons[r.secret_kind!]"
                tone="secret"
                size="lg"
                class="transition-opacity"
                :class="selecting || selected.has(r.id) ? 'opacity-0' : 'group-hover/row:opacity-0'"
              />
              <span
                class="absolute inset-0 grid place-items-center rounded-[9px] border transition-opacity"
                :class="
                  selected.has(r.id)
                    ? 'bg-primary border-primary text-primary-foreground'
                    : selecting
                      ? 'bg-card border-foreground/30'
                      : 'bg-card border-foreground/30 opacity-0 group-hover/row:opacity-100'
                "
                ><Check v-if="selected.has(r.id)" class="size-4" :stroke-width="2.6"
              /></span>
            </button>
            <RouterLink v-slot="{ href, navigate }" custom :to="`/r/${r.id}`">
              <a
                :href="href"
                class="flex h-full min-w-0 flex-1 items-center gap-3 outline-none"
                @click="onRowClick($event, i, navigate)"
                ><span class="flex min-w-0 flex-1 flex-col"
                  ><span class="truncate text-[14px] font-semibold">{{ r.title || 'Без названия' }}</span
                  ><span class="text-muted-foreground truncate text-xs">{{ subtitle(r) }}</span></span
                ><time class="text-muted-foreground w-16 shrink-0 text-right text-xs">{{
                  fmt.format(r.updated_at)
                }}</time></a
              >
            </RouterLink>
          </div>
        </RecordContextMenu>
      </div>
      <div v-else class="py-20 text-center">
        <KindTile :icon="KeyRound" tone="secret" size="lg" class="mx-auto mb-3" />
        <p class="text-sm font-medium">{{ query ? 'Ничего не найдено' : 'Секретов пока нет' }}</p>
        <p class="text-muted-foreground mt-1 text-xs">Создайте логин, ключ, токен, карту или защищённую заметку.</p>
      </div>
    </div>

    <Transition
      enter-from-class="opacity-0 translate-y-2"
      leave-to-class="opacity-0 translate-y-2"
      enter-active-class="transition duration-200"
      leave-active-class="transition duration-150"
    >
      <div
        v-if="selecting"
        role="toolbar"
        aria-label="Действия с выбранными"
        class="bg-popover ring-foreground/10 absolute bottom-5 left-1/2 flex -translate-x-1/2 items-center gap-1 rounded-xl py-1.5 pr-1.5 pl-3.5 shadow-lg ring-1"
      >
        <span class="pr-2 text-sm font-medium tabular-nums">Выбрано: {{ selected.size }}</span>
        <Button v-if="selected.size < filtered.length" variant="ghost" size="sm" @click="selectAll"
          >Выбрать все ({{ filtered.length }})</Button
        >
        <Button variant="ghost" size="sm" class="text-destructive" @click="confirmDelete = true"
          ><Trash2 class="size-4" /> Удалить</Button
        >
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Снять выделение"
          title="Снять выделение (Esc)"
          @click="clearSelection"
          ><X class="size-4"
        /></Button>
      </div>
    </Transition>

    <Dialog v-model:open="confirmDelete">
      <DialogContent>
        <DialogHeader>
          <DialogTitle
            >Удалить {{ selected.size }} {{ plural(selected.size, 'секрет', 'секрета', 'секретов') }}?</DialogTitle
          >
          <DialogDescription
            >Секреты удаляются сразу и навсегда — в корзину они не попадают. Отменить это нельзя.</DialogDescription
          >
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" @click="confirmDelete = false">Отмена</Button>
          <Button variant="destructive" :disabled="deleting" @click="deleteSelected">Удалить навсегда</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>
