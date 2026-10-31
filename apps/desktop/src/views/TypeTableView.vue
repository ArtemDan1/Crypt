<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowDown, ArrowUpRight, KeyRound, ListFilter, Plus, Search, X } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import RecordContextMenu from '@/components/RecordContextMenu.vue'
import FieldEditor from '@/components/record/FieldEditor.vue'
import { api, CryptApiError } from '@/lib/api'
import { debouncedSaver } from '@/lib/debounce'
import { formatValue, optionClass, optionsOf } from '@/lib/fields'
import { typeIcon } from '@/lib/icons'
import KindTile from '@/components/crypt/KindTile.vue'
import { toneForType } from '@/lib/tones'
import { applyView, columnLabel, sanitizeConfig, type Filter, type FilterOp, type ViewConfig } from '@/lib/table'
import type { FieldValue, Id, RecordView, ViewDef } from '@/lib/types'
import { useLibrary } from '@/stores/library'

const props = defineProps<{ typeId: string }>()
const lib = useLibrary()
const router = useRouter()

const type = computed(() => lib.typeById.get(props.typeId))
const rows = ref<RecordView[]>([])
const views = ref<ViewDef[]>([])
const activeView = ref<Id | null>(null)
const allCfg = ref<ViewConfig | null>(null)
const q = ref('')
const editing = ref<{ row: Id; col: string } | null>(null)
const newFilter = ref<Filter>({ col: 'title', op: 'contains', value: '' })
const filterOpen = ref(false)
const freshId = ref<Id | null>(null)
const tabs = ref<HTMLElement | null>(null)
const underline = ref({ left: 0, width: 0 })
function measureTabs() {
  const el = tabs.value?.querySelector<HTMLElement>('[aria-selected="true"]')
  if (el) underline.value = { left: el.offsetLeft, width: el.offsetWidth }
}
onMounted(() => nextTick(measureTabs))
watch(
  () => [activeView.value, views.value.length],
  () => nextTick(measureTabs),
)

const ulid = () => {
  const a = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'
  let t = Date.now(),
    time = ''
  for (let i = 0; i < 10; i++) {
    time = a[t % 32] + time
    t = Math.floor(t / 32)
  }
  return time + Array.from(crypto.getRandomValues(new Uint8Array(16)), (b) => a[b % 32]).join('')
}

const cfg = computed<ViewConfig>({
  get() {
    if (!type.value) return { columns: [], hidden: [], sort: [], filters: [] }
    const v = views.value.find((x) => x.id === activeView.value)
    return sanitizeConfig(v ? v.config : allCfg.value, type.value)
  },
  set(next) {
    const v = views.value.find((x) => x.id === activeView.value)
    if (v) {
      v.config = next
      viewSaver.push({ ...v, config: next })
    } else allCfg.value = next
  },
})
const columns = computed(() => cfg.value.columns.filter((c) => !cfg.value.hidden.includes(c) && c !== 'folder'))
const titleOf = (id: string) => lib.titleById.get(id)
const shown = computed(() => {
  if (!type.value) return []
  const base = applyView(rows.value, cfg.value, type.value, titleOf)
  const s = q.value.trim().toLowerCase()
  return s ? base.filter((r) => r.title.toLowerCase().includes(s)) : base
})

const viewSaver = debouncedSaver(async (v: ViewDef) => {
  try {
    await api.putView(JSON.parse(JSON.stringify(v)))
  } catch (e) {
    fail(e)
  }
}, 400)

function fail(e: unknown) {
  toast.error(e instanceof CryptApiError ? e.message : String(e))
}

async function load() {
  if (!type.value) return
  const list = await api.listRecords({ typeId: props.typeId })
  rows.value = await api.getRecords(list.map((r) => r.id))
  views.value = await api.listViews(props.typeId)
}
watch([() => props.typeId, () => lib.version, type], load, { immediate: true })
// The type may have changed elsewhere (fields removed): start from fresh definitions.
lib.refreshAll()

function fieldOf(col: string) {
  return type.value?.fields.find((f) => f.id === col)
}
function display(r: RecordView, col: string): string {
  if (col === 'title') return r.title || 'Без названия'
  if (col === 'tags') return r.tags.map((t) => `#${t}`).join(' ')
  const f = fieldOf(col)
  return f ? formatValue(r.fields[col], f.kind, { titleOf }) : ''
}

function toggleSort(col: string, e: MouseEvent) {
  const sort = [...cfg.value.sort]
  const i = sort.findIndex((s) => s.col === col)
  const cur = sort[i]
  if (!e.shiftKey) {
    const next = !cur ? 'asc' : cur.dir === 'asc' ? 'desc' : null
    cfg.value = { ...cfg.value, sort: next ? [{ col, dir: next }] : [] }
    return
  }
  if (!cur) sort.push({ col, dir: 'asc' })
  else if (cur.dir === 'asc') sort[i] = { col, dir: 'desc' }
  else sort.splice(i, 1)
  cfg.value = { ...cfg.value, sort }
}
const sortOf = (col: string) => cfg.value.sort.find((s) => s.col === col)?.dir

const OP_LABEL: Record<FilterOp, string> = { contains: 'содержит', eq: 'равно', empty: 'пусто', notEmpty: 'не пусто' }
function addFilter() {
  const f = { ...newFilter.value }
  if ((f.op === 'eq' || f.op === 'contains') && !f.value?.trim()) return
  if (f.op === 'empty' || f.op === 'notEmpty') delete f.value
  cfg.value = { ...cfg.value, filters: [...cfg.value.filters, f] }
  filterOpen.value = false
  newFilter.value = { col: 'title', op: 'contains', value: '' }
}
function removeFilter(i: number) {
  cfg.value = { ...cfg.value, filters: cfg.value.filters.filter((_, k) => k !== i) }
}
function filterText(f: Filter) {
  const label = type.value ? columnLabel(f.col, type.value) : f.col
  return f.value === undefined ? `${label}: ${OP_LABEL[f.op]}` : `${label} ${OP_LABEL[f.op]} «${f.value}»`
}

async function saveAsView() {
  const name = `Вид ${views.value.length + 1}`
  const v: ViewDef = { id: ulid(), type_id: props.typeId, name, config: JSON.parse(JSON.stringify(cfg.value)) }
  try {
    await api.putView(v)
    views.value = [...views.value, v]
    activeView.value = v.id
  } catch (e) {
    fail(e)
  }
}
async function renameView(v: ViewDef, ev: Event) {
  const name = (ev.target as HTMLInputElement).value.trim()
  if (!name || name === v.name) return
  v.name = name
  viewSaver.push({ ...v })
}
async function deleteView(v: ViewDef) {
  try {
    await api.deleteView(v.id)
    views.value = views.value.filter((x) => x.id !== v.id)
    activeView.value = null
  } catch (e) {
    fail(e)
  }
}

async function setCell(r: RecordView, col: string, v: FieldValue | null) {
  try {
    await api.setField(r.id, col, v)
    const fresh = await api.getRecord(r.id)
    rows.value = rows.value.map((x) => (x.id === r.id ? fresh : x))
  } catch (e) {
    fail(e)
  }
}
async function setTitle(r: RecordView, ev: Event) {
  const t = (ev.target as HTMLInputElement).value.trim()
  editing.value = null
  if (t === r.title) return
  try {
    await api.setTitle(r.id, t)
    await lib.changed()
  } catch (e) {
    fail(e)
  }
}
async function newRow() {
  const id = await lib.createRecord({ type_id: props.typeId })
  await load()
  freshId.value = id
  setTimeout(() => {
    if (freshId.value === id) freshId.value = null
  }, 900)
  editing.value = { row: id, col: 'title' }
}
function startEdit(r: RecordView, col: string) {
  const f = fieldOf(col)
  if (col === 'title' || (f && f.kind.kind !== 'RecordLink' && f.kind.kind !== 'SecretLink'))
    editing.value = { row: r.id, col }
}
const isEditing = (r: RecordView, col: string) => editing.value?.row === r.id && editing.value.col === col
const options = (col: string) => {
  const f = fieldOf(col)
  return f ? optionsOf(f.kind) : []
}
const selectIds = (v: FieldValue | undefined) =>
  v?.kind === 'Select' ? [v.value] : v?.kind === 'MultiSelect' ? v.value : []
</script>

<template>
  <div v-if="type" class="flex h-full flex-col">
    <header data-tauri-drag-region class="shrink-0 border-b pr-5 pl-7">
      <div class="flex h-16 flex-wrap items-center gap-3">
        <KindTile :icon="typeIcon(type.icon)" :tone="toneForType(lib.types, type.id)" size="lg" />
        <h1 class="text-[22px] font-semibold tracking-[-.02em]">{{ type.name }}</h1>
        <span class="text-muted-foreground text-[13px]">{{ shown.length }} из {{ rows.length }}</span>
        <div class="flex-1" />
        <Button variant="outline" size="sm" @click="router.push(`/types/${type.id}`)">Поля типа</Button>
        <Button size="sm" @click="newRow"><Plus class="size-4" /> Новая запись</Button>
      </div>

      <div class="flex flex-wrap items-end gap-2">
        <div ref="tabs" role="tablist" aria-label="Виды" class="relative flex flex-wrap gap-1">
          <span
            aria-hidden="true"
            class="bg-primary absolute -bottom-px h-0.5 rounded-full transition-[left,width] duration-350 ease-[cubic-bezier(.3,1.2,.5,1)]"
            :style="{ left: `${underline.left}px`, width: `${underline.width}px` }"
          />
          <button
            role="tab"
            :aria-selected="!activeView"
            class="h-[38px] px-2.5 text-[13px] transition-colors"
            :class="
              !activeView ? 'text-foreground font-semibold' : 'text-muted-foreground hover:text-foreground font-medium'
            "
            @click="activeView = null"
          >
            Все
          </button>
          <div v-for="v in views" :key="v.id" class="flex items-center">
            <button
              v-if="activeView !== v.id"
              role="tab"
              :aria-selected="false"
              class="text-muted-foreground hover:text-foreground h-[38px] px-2.5 text-[13px] font-medium transition-colors"
              @click="activeView = v.id"
            >
              {{ v.name }}
            </button>
            <span
              v-else
              role="tab"
              aria-selected="true"
              class="flex h-[38px] items-center gap-1 pr-1 pl-2.5 font-semibold"
            >
              <input
                :value="v.name"
                class="w-[7em] bg-transparent text-[13px] outline-none"
                aria-label="Название вида"
                @change="renameView(v, $event)"
              />
              <button
                class="text-muted-foreground hover:text-foreground"
                aria-label="Удалить вид"
                @click="deleteView(v)"
              >
                <X class="size-3" />
              </button>
            </span>
          </div>
          <button
            class="text-muted-foreground hover:bg-foreground/[.08] mb-1 size-[30px] rounded-[7px] text-[15px] transition-colors"
            aria-label="Сохранить как вид"
            title="Сохранить текущие фильтры как вид"
            @click="saveAsView"
          >
            +
          </button>
        </div>
        <span class="bg-border mx-1 mb-2.5 h-5 w-px" />
        <span
          v-for="(f, i) in cfg.filters"
          :key="i"
          class="bg-primary-soft text-primary-soft-fg mb-1.5 flex h-[26px] items-center gap-1.5 rounded-[7px] pr-1.5 pl-2.5 text-xs font-medium"
        >
          {{ filterText(f) }}
          <button
            class="text-muted-foreground hover:text-foreground"
            aria-label="Убрать фильтр"
            @click="removeFilter(i)"
          >
            <X class="size-3" />
          </button>
        </span>
        <Popover v-model:open="filterOpen">
          <PopoverTrigger as-child>
            <button
              class="text-muted-foreground hover:text-foreground border-border-strong mb-1.5 flex h-[26px] items-center gap-1.5 rounded-[7px] border border-dashed px-2.5 text-xs"
            >
              <ListFilter class="size-3" /> Фильтр
            </button>
          </PopoverTrigger>
          <PopoverContent align="start" class="flex w-72 flex-col gap-2 p-3">
            <select v-model="newFilter.col" class="bg-card h-8 rounded-md border px-2 text-[13px]" aria-label="Колонка">
              <option v-for="c in cfg.columns.filter((c) => c !== 'folder')" :key="c" :value="c">
                {{ columnLabel(c, type) }}
              </option>
            </select>
            <select v-model="newFilter.op" class="bg-card h-8 rounded-md border px-2 text-[13px]" aria-label="Условие">
              <option v-for="(label, op) in OP_LABEL" :key="op" :value="op">{{ label }}</option>
            </select>
            <input
              v-if="newFilter.op === 'eq' || newFilter.op === 'contains'"
              v-model="newFilter.value"
              class="h-8 rounded-md border bg-transparent px-2 text-[13px] outline-none"
              placeholder="Значение"
              aria-label="Значение"
              @keydown.enter="addFilter"
            />
            <Button size="sm" @click="addFilter">Добавить</Button>
          </PopoverContent>
        </Popover>
        <div class="flex-1" />
        <label
          class="bg-background text-muted-foreground mb-1 flex h-[30px] items-center gap-2 rounded-lg border px-2.5 text-[13px]"
        >
          <Search class="size-3.5" />
          <input
            v-model="q"
            class="text-foreground w-36 bg-transparent outline-none"
            placeholder="Найти в таблице"
            aria-label="Найти в таблице"
          />
        </label>
      </div>
    </header>

    <div class="min-h-0 flex-1 overflow-auto pr-5 pl-4">
      <table class="w-full min-w-[760px] border-collapse text-[13px]">
        <thead class="bg-background/85 sticky top-0 z-10 backdrop-blur">
          <tr class="text-muted-foreground h-9 border-b">
            <th v-for="(c, i) in columns" :key="c" class="px-3 text-left font-medium" :class="i === 0 && 'pl-6'">
              <button
                class="hover:text-foreground flex items-center gap-1.5 transition-colors"
                :class="sortOf(c) && 'text-foreground'"
                :title="'Сортировка (Shift — добавить)'"
                @click="toggleSort(c, $event)"
              >
                {{ columnLabel(c, type) }}
                <ArrowDown
                  class="size-3 transition-[transform,opacity] duration-300 ease-[cubic-bezier(.3,1.3,.5,1)]"
                  :class="[sortOf(c) ? 'opacity-100' : 'opacity-0', sortOf(c) === 'asc' && 'rotate-180']"
                />
              </button>
            </th>
          </tr>
        </thead>
        <tbody>
          <RecordContextMenu v-for="r in shown" :key="r.id" :record="r"
            ><tr
              class="group/row hover:bg-foreground/[.035] h-11 border-b transition-colors"
              :class="freshId === r.id && 'anim-fresh'"
            >
              <td
                v-for="(c, i) in columns"
                :key="c"
                class="max-w-[320px] px-3"
                :class="i === 0 && 'pl-6'"
                @dblclick="startEdit(r, c)"
              >
                <template v-if="c === 'title'">
                  <input
                    v-if="isEditing(r, c)"
                    :ref="(el) => (el as HTMLInputElement | null)?.focus()"
                    :value="r.title"
                    class="border-primary h-8 w-full rounded-md border bg-transparent px-1.5 font-medium outline-none"
                    aria-label="Название"
                    @blur="setTitle(r, $event)"
                    @keydown.enter="($event.target as HTMLInputElement).blur()"
                    @keydown.esc="editing = null"
                  />
                  <RouterLink
                    v-else
                    :to="`/r/${r.id}`"
                    class="flex items-center gap-1.5 font-medium"
                    :class="!r.title && 'text-muted-foreground'"
                    ><span class="truncate">{{ display(r, c) }}</span
                    ><ArrowUpRight
                      class="text-muted-foreground size-3.5 shrink-0 -translate-x-1 opacity-0 transition group-hover/row:translate-x-0 group-hover/row:opacity-100"
                  /></RouterLink>
                </template>
                <div
                  v-else-if="isEditing(r, c) && fieldOf(c)"
                  class="border-primary rounded-md border"
                  @focusout="editing = null"
                >
                  <FieldEditor
                    :field="fieldOf(c)!"
                    :value="r.fields[c]"
                    :record-id="r.id"
                    autofocus
                    @change="(v) => setCell(r, c, v)"
                    @done="editing = null"
                  />
                </div>
                <template v-else-if="fieldOf(c)?.kind.kind === 'SecretLink' && r.fields[c]?.kind === 'Links'">
                  <span
                    v-for="id in (r.fields[c] as { value: string[] }).value"
                    :key="id"
                    class="bg-secret-bg text-secret-fg mr-1 inline-flex h-6 items-center gap-1.5 rounded-md px-2 text-xs font-medium"
                  >
                    <KeyRound class="size-3" /> {{ titleOf(id) ?? 'удалено' }}
                  </span>
                </template>
                <template v-else-if="options(c).length">
                  <span
                    v-for="o in options(c).filter((o) => selectIds(r.fields[c]).includes(o.id))"
                    :key="o.id"
                    class="mr-1 rounded-full px-2 py-0.5 text-xs"
                    :class="optionClass(o.color)"
                    >{{ o.label }}</span
                  >
                </template>
                <span
                  v-else
                  class="block truncate"
                  :class="fieldOf(c)?.kind.kind === 'Number' ? 'tabular-nums' : ''"
                  tabindex="0"
                  @keydown.enter="startEdit(r, c)"
                  >{{ display(r, c) || ' ' }}</span
                >
              </td>
            </tr></RecordContextMenu
          >
          <tr class="h-10">
            <td :colspan="columns.length" class="pl-6">
              <button
                class="text-muted-foreground hover:text-foreground flex items-center gap-1.5 transition-colors"
                @click="newRow"
              >
                <Plus class="size-3.5" /> Новая строка
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
  <div v-else class="text-muted-foreground flex h-full items-center justify-center">Тип не найден</div>
</template>
