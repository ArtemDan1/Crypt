<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { GripVertical, KeyRound, Plus, Trash2, X } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Switch } from '@/components/ui/switch'
import PageHeader from '@/components/PageHeader.vue'
import { api, CryptApiError } from '@/lib/api'
import { debouncedSaver } from '@/lib/debounce'
import { OPTION_COLORS, optionClass } from '@/lib/fields'
import { FIELD_KINDS, kindMeta } from '@/lib/fieldKinds'
import KindTile from '@/components/crypt/KindTile.vue'
import { toneForType } from '@/lib/tones'
import { TYPE_ICONS, typeIcon } from '@/lib/icons'
import type { FieldDef, FieldKind, FieldKindName, TypeDef } from '@/lib/types'
import { useLibrary } from '@/stores/library'

const props = defineProps<{ typeId?: string }>()
const lib = useLibrary()
const route = useRoute()
const router = useRouter()

const ulid = () => {
  const a = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'
  let t = Date.now()
  let time = ''
  for (let i = 0; i < 10; i++) {
    time = a[t % 32] + time
    t = Math.floor(t / 32)
  }
  let rnd = ''
  const bytes = crypto.getRandomValues(new Uint8Array(16))
  for (let i = 0; i < 16; i++) rnd += a[bytes[i]! % 32]
  return time + rnd
}

// Vue proxies cannot go through structuredClone.
const clone = <T,>(x: T): T => JSON.parse(JSON.stringify(x))

const KINDS = FIELD_KINDS
const freshField = ref<string | null>(null)
const SAMPLE: Partial<Record<FieldKindName, string>> = {
  Text: 'Фрэнк Герберт',
  Number: '5',
  Date: '12 сентября',
  Url: 'example.com',
  Checkbox: 'Да',
  Select: 'Читаю',
  MultiSelect: 'Фантастика, Классика',
  RecordLink: 'Пикник на обочине',
  SecretLink: '•••• секрет',
}

function makeKind(k: FieldKindName): FieldKind {
  if (k === 'Select' || k === 'MultiSelect') return { kind: k, value: { options: [] } }
  if (k === 'RecordLink') return { kind: k, value: { target_type: null, multiple: true } }
  if (k === 'SecretLink') return { kind: k, value: { multiple: true } }
  return { kind: k } as FieldKind
}

const draft = ref<TypeDef | null>(null)
const selectedId = computed(() => props.typeId ?? lib.types[0]?.id)
const recordCount = computed(() => (draft.value ? (lib.countByType.get(draft.value.id) ?? 0) : 0))
const addOpen = ref(false)
const dragFrom = ref<number | null>(null)

const saver = debouncedSaver(async (t: TypeDef) => {
  try {
    await api.putType(t)
    await lib.changed()
  } catch (e) {
    toast.error(e instanceof CryptApiError ? e.message : String(e))
  }
}, 400)

function load() {
  const t = lib.types.find((x) => x.id === selectedId.value)
  draft.value = t ? clone(t) : null
}
watch(selectedId, async () => {
  await saver.flush()
  load()
})
watch(
  () => lib.types,
  () => {
    if (!draft.value || draft.value.id !== selectedId.value) load()
  },
)

function save() {
  if (draft.value && draft.value.name.trim()) saver.push(clone(draft.value))
}

async function createType() {
  await saver.flush()
  const t: TypeDef = {
    id: ulid(),
    name: 'Новый тип',
    icon: 'file-text',
    fields: [{ id: ulid(), name: 'Описание', kind: { kind: 'Text' } }],
  }
  await api.putType(t)
  await lib.changed()
  router.replace(`/types/${t.id}`)
}

onMounted(async () => {
  if (!lib.types.length) await lib.refreshAll()
  load()
  if (route.query.new) await createType()
})

function addField(k: FieldKindName) {
  if (!draft.value) return
  const id = ulid()
  draft.value.fields.push({ id, name: kindMeta(k).label, kind: makeKind(k) })
  freshField.value = id
  setTimeout(() => freshField.value === id && (freshField.value = null), 900)
  addOpen.value = false
  save()
}
function removeField(f: FieldDef) {
  if (!draft.value) return
  draft.value.fields = draft.value.fields.filter((x) => x.id !== f.id)
  save()
  toast('Поле удалено. Значения в записях сохранятся скрытыми.')
}
function addOption(f: FieldDef) {
  if (f.kind.kind !== 'Select' && f.kind.kind !== 'MultiSelect') return
  const colors = Object.keys(OPTION_COLORS)
  f.kind.value.options.push({
    id: ulid(),
    label: 'Вариант',
    color: colors[f.kind.value.options.length % colors.length]!,
  })
  save()
}
function setIcon(name: string) {
  if (!draft.value) return
  draft.value.icon = name
  save()
}
function removeOption(kind: { options: { id: string }[] }, id: string) {
  kind.options = kind.options.filter((x) => x.id !== id)
  save()
}
function cycleColor(o: { color: string | null }) {
  const colors = Object.keys(OPTION_COLORS)
  o.color = colors[(colors.indexOf(o.color ?? 'zinc') + 1) % colors.length]!
  save()
}
function onDrop(to: number) {
  if (!draft.value || dragFrom.value === null || dragFrom.value === to) return
  const [moved] = draft.value.fields.splice(dragFrom.value, 1)
  draft.value.fields.splice(to, 0, moved!)
  dragFrom.value = null
  save()
}
async function deleteType() {
  if (!draft.value) return
  try {
    await api.deleteType(draft.value.id)
    await lib.changed()
    router.replace('/types')
  } catch (e) {
    toast.error(e instanceof CryptApiError ? e.message : String(e))
  }
}
async function addStarter() {
  for (const t of await api.starterTypes()) await api.putType(t)
  await lib.changed()
}
</script>

<template>
  <div class="flex h-full flex-col">
    <PageHeader title="Типы" :sub="`${lib.types.length}`">
      <Button size="sm" @click="createType"><Plus class="size-4" /> Тип</Button>
    </PageHeader>

    <div class="flex min-h-0 flex-1">
      <section aria-label="Список типов" class="w-56 shrink-0 overflow-auto border-r px-2.5 py-4">
        <RouterLink
          v-for="t in lib.types"
          :key="t.id"
          :to="`/types/${t.id}`"
          class="flex h-9 items-center gap-2.5 rounded-lg px-2.5 transition-colors"
          :class="t.id === selectedId ? 'bg-accent font-semibold' : 'hover:bg-foreground/[.05] font-medium'"
        >
          <KindTile :icon="typeIcon(t.icon)" :tone="toneForType(lib.types, t.id)" />
          <span class="flex-1 truncate text-[14px]">{{ t.name }}</span>
          <span class="text-muted-foreground text-xs">{{ lib.countByType.get(t.id) || '' }}</span>
        </RouterLink>
        <button
          v-if="!lib.types.some((t) => t.name === 'Сервер')"
          class="text-muted-foreground hover:text-foreground mt-2 w-full rounded-lg border border-dashed px-2.5 py-2 text-left text-xs"
          @click="addStarter"
        >
          + Готовый тип «Сервер»
        </button>
      </section>

      <main v-if="draft" class="flex min-w-0 flex-1">
        <div :key="draft.id" class="anim-in min-w-0 flex-1 overflow-auto px-9 py-8">
          <div class="flex flex-wrap items-center gap-3.5">
            <Popover>
              <PopoverTrigger as-child>
                <button aria-label="Иконка типа" class="transition-transform hover:scale-105 active:scale-95">
                  <KindTile
                    :icon="typeIcon(draft.icon)"
                    :tone="toneForType(lib.types, draft.id)"
                    size="lg"
                    class="size-[52px]! rounded-[14px]! [&_svg]:size-6!"
                  />
                </button>
              </PopoverTrigger>
              <PopoverContent align="start" class="grid w-[264px] grid-cols-6 gap-1 p-2">
                <button
                  v-for="(icon, name) in TYPE_ICONS"
                  :key="name"
                  :aria-label="name"
                  class="hover:bg-accent flex size-9 items-center justify-center rounded-md"
                  :class="draft.icon === name && 'bg-accent'"
                  @click="setIcon(name)"
                >
                  <component :is="icon" class="size-4" />
                </button>
              </PopoverContent>
            </Popover>
            <div class="min-w-[200px] flex-1">
              <label for="tname" class="text-muted-foreground block text-xs">Название типа</label>
              <input
                id="tname"
                v-model="draft.name"
                class="w-full bg-transparent text-[28px] font-semibold tracking-[-.02em] outline-none"
                @input="save"
              />
            </div>
            <RouterLink
              :to="`/t/${draft.id}`"
              class="hover:bg-accent flex h-8 items-center rounded-md border px-3 text-[13px]"
            >
              Открыть таблицу · {{ recordCount }}
            </RouterLink>
          </div>

          <div class="mt-7 mb-2.5 flex items-center justify-between">
            <h3 class="text-muted-foreground text-xs font-semibold tracking-wider uppercase">Поля</h3>
            <span class="text-muted-foreground text-xs">Перетащите, чтобы поменять порядок</span>
          </div>
          <div class="flex flex-col gap-1.5">
            <div
              v-for="(f, i) in draft.fields"
              :key="f.id"
              class="group/f bg-card rounded-[11px] border transition-shadow hover:shadow-[0_6px_18px_-12px_rgba(0,0,0,.3)]"
              :class="[dragFrom === i && 'opacity-50', freshField === f.id && 'anim-fresh']"
              @dragover.prevent
              @drop="onDrop(i)"
            >
              <div class="flex min-h-12 items-center gap-2.5 pr-2.5 pl-1.5">
                <span
                  draggable="true"
                  class="text-muted-foreground/60 hover:text-muted-foreground cursor-grab"
                  aria-label="Перетащить"
                  @dragstart="dragFrom = i"
                >
                  <GripVertical class="size-4" />
                </span>
                <span
                  class="flex size-7 shrink-0 items-center justify-center rounded-[7px] font-mono text-xs font-medium"
                  :class="f.kind.kind === 'SecretLink' ? 'bg-secret-bg text-secret-fg' : 'bg-accent text-foreground-2'"
                  >{{ kindMeta(f.kind.kind).glyph }}</span
                >
                <input
                  v-model="f.name"
                  class="min-w-0 flex-1 bg-transparent font-medium outline-none"
                  :aria-label="`Название поля ${f.name}`"
                  @input="save"
                />
                <span class="text-muted-foreground text-[13px]">{{ kindMeta(f.kind.kind).label }}</span>
                <template v-if="f.kind.kind === 'RecordLink'">
                  <select
                    v-model="f.kind.value.target_type"
                    class="bg-card h-7 rounded-md border px-1.5 text-xs"
                    aria-label="Тип связанных записей"
                    @change="save"
                  >
                    <option :value="null">Любой тип</option>
                    <option v-for="t in lib.types" :key="t.id" :value="t.id">{{ t.name }}</option>
                  </select>
                </template>
                <label
                  v-if="f.kind.kind === 'RecordLink' || f.kind.kind === 'SecretLink'"
                  class="text-muted-foreground flex items-center gap-1.5 text-xs"
                >
                  <Switch v-model="f.kind.value.multiple" size="sm" @update:model-value="save" /> несколько
                </label>
                <button
                  class="text-muted-foreground hover:text-destructive hover:bg-foreground/[.06] rounded-md p-1.5 opacity-0 transition group-hover/f:opacity-100 focus-visible:opacity-100"
                  :aria-label="`Удалить поле ${f.name}`"
                  @click="removeField(f)"
                >
                  <Trash2 class="size-3.5" />
                </button>
              </div>
              <div
                v-if="f.kind.kind === 'Select' || f.kind.kind === 'MultiSelect'"
                class="flex flex-wrap items-center gap-1.5 pb-3 pl-[66px]"
              >
                <span
                  v-for="o in f.kind.value.options"
                  :key="o.id"
                  class="flex items-center gap-1 rounded-full py-0.5 pr-1 pl-1 text-xs"
                  :class="optionClass(o.color)"
                >
                  <button
                    class="size-3 rounded-full border border-current opacity-70"
                    aria-label="Сменить цвет"
                    @click="cycleColor(o)"
                  />
                  <input
                    v-model="o.label"
                    class="w-[7.5em] bg-transparent outline-none"
                    :aria-label="`Вариант ${o.label}`"
                    @input="save"
                  />
                  <button
                    aria-label="Удалить вариант"
                    class="opacity-70 hover:opacity-100"
                    @click="removeOption(f.kind.value, o.id)"
                  >
                    <X class="size-3" />
                  </button>
                </span>
                <button
                  class="text-muted-foreground hover:text-foreground border-border-strong h-6 rounded-md border border-dashed px-2 text-xs"
                  @click="addOption(f)"
                >
                  + вариант
                </button>
              </div>
            </div>
            <div class="pt-1">
              <Popover v-model:open="addOpen">
                <PopoverTrigger as-child>
                  <button
                    class="border-border-strong text-foreground-2 hover:bg-foreground/[.04] aria-expanded:bg-accent flex h-[38px] items-center gap-2 rounded-[9px] border border-dashed px-3.5 text-[13px] font-medium transition-colors"
                  >
                    <Plus class="size-3.5" :stroke-width="2.4" /> Добавить поле
                  </button>
                </PopoverTrigger>
                <PopoverContent align="start" class="grid w-[420px] grid-cols-3 gap-1.5 p-3">
                  <button
                    v-for="k in KINDS"
                    :key="k.kind"
                    class="bg-background hover:border-primary/50 flex flex-col items-start gap-2 rounded-[10px] border p-2.5 text-left transition-[transform,border-color] duration-200 ease-[cubic-bezier(.3,1.4,.5,1)] hover:-translate-y-0.5"
                    @click="addField(k.kind)"
                  >
                    <span
                      class="flex size-[26px] items-center justify-center rounded-[7px] font-mono text-xs"
                      :class="k.kind === 'SecretLink' ? 'bg-secret-bg text-secret-fg' : 'bg-accent text-foreground-2'"
                      >{{ k.glyph }}</span
                    >
                    <span class="text-[13px] font-medium">{{ k.label }}</span>
                  </button>
                </PopoverContent>
              </Popover>
            </div>
          </div>

          <div class="border-destructive/30 mt-8 flex items-center gap-3 rounded-xl border px-4 py-3.5">
            <span class="flex flex-1 flex-col gap-0.5">
              <span class="text-[14px] font-medium">Удалить тип</span>
              <span class="text-muted-foreground text-xs">{{
                recordCount > 0
                  ? `Сначала смените тип или удалите записи этого типа (${recordCount})`
                  : 'Записей этого типа нет, тип можно удалить'
              }}</span>
            </span>
            <Button variant="outline" size="sm" class="text-destructive" :disabled="recordCount > 0" @click="deleteType"
              >Удалить</Button
            >
          </div>
          <p class="text-muted-foreground mt-4 flex items-center gap-1.5 text-xs">
            <KeyRound class="text-secret size-3.5" /> Поле «Секрет» ссылается на запись из хранилища паролей — значение
            не показывается в карточке.
          </p>
        </div>
        <aside
          aria-label="Предпросмотр записи"
          class="bg-background hidden w-[300px] shrink-0 flex-col gap-3 border-l px-5 py-7 xl:flex"
        >
          <span class="text-muted-foreground text-[11px] font-semibold tracking-wider uppercase"
            >Так выглядит запись</span
          >
          <div
            class="bg-card flex flex-col gap-3 rounded-[14px] border p-[18px] shadow-[0_10px_30px_-20px_rgba(0,0,0,.25)]"
          >
            <span
              class="bg-primary-soft text-primary-soft-fg inline-flex h-[22px] items-center self-start rounded-md px-2 text-[11px] font-medium"
              >{{ draft.name || 'Без названия' }}</span
            >
            <span class="text-[19px] font-semibold tracking-[-.01em]">Пример записи</span>
            <TransitionGroup tag="div" name="pv" class="flex flex-col gap-2">
              <div
                v-for="f in draft.fields"
                :key="f.id"
                class="grid grid-cols-[90px_minmax(0,1fr)] items-center gap-2 text-xs"
              >
                <span class="text-muted-foreground truncate">{{ f.name }}</span>
                <span class="truncate" :class="f.kind.kind === 'SecretLink' && 'text-secret-fg'">{{
                  SAMPLE[f.kind.kind] ?? '—'
                }}</span>
              </div>
            </TransitionGroup>
          </div>
        </aside>
      </main>
      <div v-else class="text-muted-foreground flex flex-1 items-center justify-center text-[13px]">
        Создайте первый тип
      </div>
    </div>
  </div>
</template>

<style scoped>
.pv-enter-active {
  transition:
    opacity 0.35s ease,
    transform 0.35s ease;
}
.pv-enter-from {
  opacity: 0;
  transform: translateX(6px);
}
</style>
