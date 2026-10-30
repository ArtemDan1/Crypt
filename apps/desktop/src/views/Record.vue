<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { onBeforeRouteLeave, useRouter } from 'vue-router'
import { ChevronRight, History, KeyRound, MoreHorizontal, Plus } from '@lucide/vue'
import { toast } from 'vue-sonner'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import SecretPanel from '@/components/secret/SecretPanel.vue'
import FieldEditor from '@/components/record/FieldEditor.vue'
import TagEditor from '@/components/record/TagEditor.vue'
import RecordHistory from '@/components/record/RecordHistory.vue'
import MarkdownEditor from '@/components/editor/MarkdownEditor.vue'
import { api, CryptApiError } from '@/lib/api'
import { debouncedSaver } from '@/lib/debounce'
import { formatValue } from '@/lib/fields'
import { typeIcon } from '@/lib/icons'
import type { FieldValue, Id, RecordSummary, RecordView } from '@/lib/types'
import { useLibrary } from '@/stores/library'
import { useSession } from '@/stores/session'

const props = defineProps<{ id: string }>()
const lib = useLibrary()
const session = useSession()
const router = useRouter()

const rec = ref<RecordView | null>(null)
const missing = ref(false)
const backlinks = ref<RecordSummary[]>([])
const title = ref('')
const body = ref('')
const saving = ref(false)
const savedAt = ref<number | null>(null)
const showHidden = ref(false)
const historyOpen = ref(false)

const type = computed(() => (rec.value?.type_id ? lib.typeById.get(rec.value.type_id) : undefined))
const folderPath = computed(() => {
  const out: { id: string; name: string }[] = []
  const seen = new Set<string>()
  let id = rec.value?.folder ?? null
  while (id && !seen.has(id)) {
    seen.add(id)
    const f = lib.folders.find((x) => x.id === id)
    if (!f) break
    out.unshift({ id: f.id, name: f.name })
    id = f.parent
  }
  return out
})
const hiddenEntries = computed(() => Object.entries(rec.value?.hidden_fields ?? {}))
const timeFmt = new Intl.DateTimeFormat('ru', { hour: '2-digit', minute: '2-digit' })
const dateFmt = new Intl.DateTimeFormat('ru', {
  day: 'numeric',
  month: 'long',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})

function fail(e: unknown) {
  toast.error(e instanceof CryptApiError ? e.message : String(e))
}

async function load() {
  try {
    const r = await api.getRecord(props.id)
    rec.value = r
    title.value = r.title
    body.value = r.body
    savedAt.value = r.updated_at
    backlinks.value = await api.backlinks(props.id)
    // Link labels read titles from the library; make sure they are current.
    await lib.refreshAll()
  } catch (e) {
    if (e instanceof CryptApiError && e.code === 'NotFound') missing.value = true
    else fail(e)
  }
}
load()
watch(
  () => lib.version,
  async () => {
    // Titles of linked records may have changed elsewhere.
    if (rec.value) backlinks.value = await api.backlinks(props.id).catch(() => backlinks.value)
  },
)

const bodySaver = debouncedSaver(async (text: string) => {
  saving.value = true
  try {
    await api.setBody(props.id, text)
    savedAt.value = Date.now()
  } catch (e) {
    fail(e)
  } finally {
    saving.value = false
  }
}, 600)
watch(body, (b) => {
  if (rec.value && b !== rec.value.body) {
    rec.value.body = b
    bodySaver.push(b)
  }
})
const unhook = session.beforeLock(() => bodySaver.flush())
onBeforeRouteLeave(() => bodySaver.flush())
onBeforeUnmount(() => {
  bodySaver.flush()
  unhook()
})

async function mutate(fn: () => Promise<void>) {
  saving.value = true
  try {
    await fn()
    savedAt.value = Date.now()
    await lib.changed()
    const r = await api.getRecord(props.id)
    // Keep the editor's own text: it may be ahead of the store.
    rec.value = { ...r, body: body.value }
  } catch (e) {
    fail(e)
  } finally {
    saving.value = false
  }
}

function commitTitle() {
  const t = title.value.trim()
  if (rec.value && t !== rec.value.title) mutate(() => api.setTitle(props.id, t))
}
const setField = (field: Id, v: FieldValue | null) => mutate(() => api.setField(props.id, field, v))
const addTag = (t: string) => mutate(() => api.addTag(props.id, t))
const removeTag = (t: string) => mutate(() => api.removeTag(props.id, t))
const setType = (t: Id | null) => mutate(() => api.setType(props.id, t))
const setFolder = (f: Id | null) => mutate(() => api.setFolder(props.id, f))
async function openHistory() {
  // Pending typing must be a version before history is read.
  await bodySaver.flush()
  historyOpen.value = true
}
async function reverted() {
  await load()
}
async function remove() {
  await bodySaver.flush()
  try {
    await api.deleteRecord(props.id)
    await lib.changed()
    toast('Запись перемещена в корзину')
    router.push('/')
  } catch (e) {
    fail(e)
  }
}
</script>

<template>
  <div v-if="missing" class="text-muted-foreground flex h-full flex-col items-center justify-center gap-2">
    <p class="text-foreground font-medium">Запись не найдена</p>
    <p class="text-[13px]">Возможно, она удалена навсегда.</p>
  </div>
  <div v-else-if="rec" class="flex h-full flex-col">
    <header data-tauri-drag-region class="flex h-[52px] shrink-0 items-center gap-3 border-b px-5">
      <nav aria-label="Путь" class="text-muted-foreground flex min-w-0 items-center gap-1.5 text-[13px]">
        <RouterLink to="/" class="hover:text-foreground font-mono">/</RouterLink>
        <template v-for="p in folderPath" :key="p.id">
          <RouterLink :to="`/folder/${p.id}`" class="hover:text-foreground truncate">{{ p.name }}</RouterLink>
          <ChevronRight class="size-3.5 shrink-0" />
        </template>
        <span class="text-foreground truncate">{{ title || 'Без названия' }}</span>
      </nav>
      <div class="flex-1" />
      <span class="text-muted-foreground text-xs whitespace-nowrap">
        {{ saving ? 'Сохраняется…' : savedAt ? `Сохранено · ${timeFmt.format(savedAt)}` : '' }}
      </span>
      <DropdownMenu>
        <DropdownMenuTrigger as-child>
          <button
            class="hover:bg-accent flex size-8 items-center justify-center rounded-md border"
            aria-label="Действия с записью"
          >
            <MoreHorizontal class="size-4" />
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" class="w-56">
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>Переместить в папку</DropdownMenuSubTrigger>
            <DropdownMenuSubContent class="max-h-72 overflow-auto">
              <DropdownMenuItem @select="setFolder(null)">В корень /</DropdownMenuItem>
              <DropdownMenuItem v-for="f in lib.folders" :key="f.id" @select="setFolder(f.id)">{{
                f.name
              }}</DropdownMenuItem>
            </DropdownMenuSubContent>
          </DropdownMenuSub>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>Сменить тип</DropdownMenuSubTrigger>
            <DropdownMenuSubContent class="max-h-72 overflow-auto">
              <DropdownMenuItem @select="setType(null)">Без типа</DropdownMenuItem>
              <DropdownMenuItem v-for="t in lib.types" :key="t.id" @select="setType(t.id)">{{
                t.name
              }}</DropdownMenuItem>
            </DropdownMenuSubContent>
          </DropdownMenuSub>
          <DropdownMenuSeparator />
          <DropdownMenuItem @select="openHistory"><History class="size-4" />История версий</DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem variant="destructive" @select="remove">В корзину</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </header>

    <div class="flex min-h-0 flex-1">
      <article class="min-w-0 flex-1 overflow-auto px-12 pt-8 pb-24">
        <div class="mx-auto max-w-[760px]">
          <div class="mb-3 flex flex-wrap items-center gap-2">
            <RouterLink
              v-if="type"
              :to="`/t/${type.id}`"
              class="text-foreground/80 hover:bg-accent flex h-6 items-center gap-1.5 rounded-md border px-2 text-xs"
            >
              <component :is="typeIcon(type.icon)" class="size-3.5" /> {{ type.name }}
            </RouterLink>
            <span
              v-if="rec.secret_kind"
              class="text-secret flex h-6 items-center gap-1.5 rounded-md border px-2 text-xs"
            >
              <KeyRound class="size-3.5" /> Секрет
            </span>
            <DropdownMenu v-if="!type && !rec.secret_kind && lib.types.length">
              <DropdownMenuTrigger as-child>
                <button
                  class="text-muted-foreground hover:text-foreground flex h-6 items-center gap-1 rounded-md border border-dashed px-2 text-xs"
                >
                  <Plus class="size-3" /> Назначить тип
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start">
                <DropdownMenuItem v-for="t in lib.types" :key="t.id" @select="setType(t.id)">
                  <component :is="typeIcon(t.icon)" class="size-4" /> {{ t.name }}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <TagEditor :tags="rec.tags" @add="addTag" @remove="removeTag" />
          </div>

          <input
            v-model="title"
            placeholder="Без названия"
            aria-label="Заголовок"
            class="placeholder:text-muted-foreground/60 mb-6 w-full bg-transparent text-[32px] font-semibold tracking-tight outline-none"
            @blur="commitTitle"
            @keydown.enter.prevent="($event.target as HTMLInputElement).blur()"
          />

          <section
            v-if="type?.fields.length"
            aria-label="Поля"
            class="mb-6 grid grid-cols-[160px_minmax(0,1fr)] items-start gap-x-4 gap-y-0.5 border-b pb-6"
          >
            <template v-for="f in type.fields" :key="f.id">
              <div class="text-muted-foreground flex min-h-[30px] items-center truncate">{{ f.name }}</div>
              <FieldEditor
                :field="f"
                :value="rec.fields[f.id]"
                :record-id="rec.id"
                @change="(v) => setField(f.id, v)"
              />
            </template>
          </section>

          <section v-if="hiddenEntries.length" class="text-muted-foreground mb-6 text-[13px]">
            <button class="hover:text-foreground flex items-center gap-1" @click="showHidden = !showHidden">
              <ChevronRight class="size-3.5 transition-transform" :class="showHidden && 'rotate-90'" />
              Значения удалённых полей · {{ hiddenEntries.length }}
            </button>
            <ul v-if="showHidden" class="mt-2 space-y-1 pl-5">
              <li v-for="[fid, v] in hiddenEntries" :key="fid" class="font-mono text-xs selectable">
                {{
                  formatValue(v, { kind: v.kind === 'Links' ? 'RecordLink' : v.kind } as any, {
                    titleOf: (id) => lib.titleById.get(id),
                  }) || JSON.stringify(v.value)
                }}
              </li>
            </ul>
          </section>

          <SecretPanel v-if="rec?.secret_kind" :key="rec.id" :record-id="rec.id" />
          <MarkdownEditor v-model="body" :record-id="rec.id" />
        </div>
      </article>

      <aside
        aria-label="Связи"
        class="bg-card/30 hidden w-[300px] shrink-0 flex-col gap-6 overflow-auto border-l p-5 lg:flex"
      >
        <section>
          <h3 class="text-muted-foreground mb-2.5 text-xs font-medium tracking-wider uppercase">
            Бэклинки · {{ backlinks.length }}
          </h3>
          <div class="flex flex-col gap-1">
            <RouterLink
              v-for="b in backlinks"
              :key="b.id"
              :to="`/r/${b.id}`"
              class="bg-card hover:bg-accent rounded-md px-2.5 py-2"
            >
              <div class="font-medium">{{ b.title || 'Без названия' }}</div>
              <div v-if="b.type_id" class="text-muted-foreground mt-0.5 text-xs">
                {{ lib.typeById.get(b.type_id)?.name }}
              </div>
            </RouterLink>
            <p v-if="!backlinks.length" class="text-muted-foreground text-[13px]">Сюда пока никто не ссылается</p>
          </div>
        </section>
        <section class="text-[13px]">
          <h3 class="text-muted-foreground mb-2.5 text-xs font-medium tracking-wider uppercase">Информация</h3>
          <dl class="grid grid-cols-[80px_1fr] gap-y-1.5">
            <dt class="text-muted-foreground">Создано</dt>
            <dd>{{ dateFmt.format(rec.created_at) }}</dd>
            <dt class="text-muted-foreground">Изменено</dt>
            <dd>{{ dateFmt.format(rec.updated_at) }}</dd>
          </dl>
          <button
            class="text-muted-foreground hover:text-foreground mt-3 flex items-center gap-1.5 text-[13px]"
            @click="openHistory"
          >
            <History class="size-3.5" /> История версий
          </button>
        </section>
      </aside>
    </div>
    <RecordHistory v-if="rec" v-model:open="historyOpen" :current="rec" @reverted="reverted" />
  </div>
</template>
