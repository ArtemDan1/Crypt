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

const KINDS: { kind: FieldKindName; label: string; glyph: string }[] = [
  { kind: 'Text', label: 'Текст', glyph: 'T' },
  { kind: 'Number', label: 'Число', glyph: '#' },
  { kind: 'Date', label: 'Дата', glyph: '▦' },
  { kind: 'Select', label: 'Выбор', glyph: '◉' },
  { kind: 'MultiSelect', label: 'Мультивыбор', glyph: '☰' },
  { kind: 'Checkbox', label: 'Флажок', glyph: '☑' },
  { kind: 'Url', label: 'URL', glyph: '↗' },
  { kind: 'RecordLink', label: 'Ссылка на запись', glyph: '→' },
  { kind: 'SecretLink', label: 'Секрет', glyph: '✱' },
]
const kindMeta = (k: FieldKindName) => KINDS.find((x) => x.kind === k)!

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
  draft.value.fields.push({ id: ulid(), name: kindMeta(k).label, kind: makeKind(k) })
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
      <section aria-label="Список типов" class="bg-card/30 w-60 shrink-0 overflow-auto border-r p-2">
        <RouterLink
          v-for="t in lib.types"
          :key="t.id"
          :to="`/types/${t.id}`"
          class="flex h-10 items-center gap-2.5 rounded-lg px-2.5"
          :class="t.id === selectedId ? 'bg-accent' : 'hover:bg-accent/60'"
        >
          <span class="bg-muted flex size-[26px] items-center justify-center rounded-md"
            ><component :is="typeIcon(t.icon)" class="size-4"
          /></span>
          <span class="flex-1 truncate font-medium">{{ t.name }}</span>
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

      <main v-if="draft" class="min-w-0 flex-1 overflow-auto px-10 py-7">
        <div class="mx-auto max-w-[760px]">
          <div class="flex flex-wrap items-center gap-3.5">
            <Popover>
              <PopoverTrigger as-child>
                <button
                  aria-label="Иконка типа"
                  class="bg-card hover:bg-accent flex size-12 items-center justify-center rounded-xl border"
                >
                  <component :is="typeIcon(draft.icon)" class="size-6" />
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
                class="w-full bg-transparent text-[26px] font-semibold outline-none"
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

          <h3 class="text-muted-foreground mt-7 mb-2.5 text-[13px] font-medium">Поля</h3>
          <div class="overflow-hidden rounded-xl border">
            <div
              v-for="(f, i) in draft.fields"
              :key="f.id"
              class="border-b last:border-b-0"
              :class="dragFrom === i && 'opacity-50'"
              @dragover.prevent
              @drop="onDrop(i)"
            >
              <div class="flex min-h-12 items-center gap-3 px-3">
                <span
                  draggable="true"
                  class="text-muted-foreground cursor-grab"
                  aria-label="Перетащить"
                  @dragstart="dragFrom = i"
                >
                  <GripVertical class="size-4" />
                </span>
                <span
                  class="bg-muted flex size-[26px] shrink-0 items-center justify-center rounded-md text-[13px]"
                  :class="f.kind.kind === 'SecretLink' ? 'text-secret' : 'text-foreground/80'"
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
                  class="text-muted-foreground hover:text-destructive rounded p-1"
                  :aria-label="`Удалить поле ${f.name}`"
                  @click="removeField(f)"
                >
                  <Trash2 class="size-3.5" />
                </button>
              </div>
              <div
                v-if="f.kind.kind === 'Select' || f.kind.kind === 'MultiSelect'"
                class="flex flex-wrap items-center gap-1.5 px-12 pb-3"
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
                <button class="text-muted-foreground hover:text-foreground text-xs" @click="addOption(f)">
                  + вариант
                </button>
              </div>
            </div>
            <div class="p-2.5">
              <Popover v-model:open="addOpen">
                <PopoverTrigger as-child>
                  <Button variant="secondary" size="sm"><Plus class="size-4" /> Добавить поле</Button>
                </PopoverTrigger>
                <PopoverContent align="start" class="w-64 p-1.5">
                  <div class="text-muted-foreground px-2 py-1 text-[11px] tracking-wider uppercase">Вид поля</div>
                  <button
                    v-for="k in KINDS"
                    :key="k.kind"
                    class="hover:bg-accent flex h-8 w-full items-center gap-2.5 rounded-md px-2 text-[13px]"
                    @click="addField(k.kind)"
                  >
                    <span
                      class="w-[18px] text-center"
                      :class="k.kind === 'SecretLink' ? 'text-secret' : 'text-muted-foreground'"
                      >{{ k.glyph }}</span
                    >
                    {{ k.label }}
                  </button>
                </PopoverContent>
              </Popover>
            </div>
          </div>

          <div class="mt-8 flex items-center gap-3 border-t pt-5">
            <Button variant="outline" size="sm" class="text-destructive" :disabled="recordCount > 0" @click="deleteType"
              >Удалить тип</Button
            >
            <span v-if="recordCount > 0" class="text-muted-foreground text-xs"
              >Сначала смените тип или удалите записи этого типа ({{ recordCount }})</span
            >
          </div>
          <p class="text-muted-foreground mt-4 flex items-center gap-1.5 text-xs">
            <KeyRound class="text-secret size-3.5" /> Поле «Секрет» ссылается на запись из хранилища паролей — значение
            не показывается в карточке.
          </p>
        </div>
      </main>
      <div v-else class="text-muted-foreground flex flex-1 items-center justify-center text-[13px]">
        Создайте первый тип
      </div>
    </div>
  </div>
</template>
