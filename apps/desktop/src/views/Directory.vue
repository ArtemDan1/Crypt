<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ChevronRight, FolderPlus, LayoutGrid, List, Plus } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import RecordList from '@/components/RecordList.vue'
import RecordCard from '@/components/RecordCard.vue'
import SegmentedControl from '@/components/crypt/SegmentedControl.vue'
import { useViewPref } from '@/lib/viewPref'
import { api } from '@/lib/api'
import { useLibrary } from '@/stores/library'

const props = defineProps<{ folderId?: string }>()
const lib = useLibrary()
const router = useRouter()
const parent = computed(() => props.folderId ?? null)
const folder = computed(() => lib.folders.find((f) => f.id === props.folderId))
const missing = computed(() => !!props.folderId && !folder.value)
const children = computed(() =>
  lib.folders.filter((f) => f.parent === parent.value).sort((a, b) => a.name.localeCompare(b.name, 'ru')),
)
const records = computed(() =>
  lib.records
    .filter((r) => r.folder === parent.value && !r.secret_kind)
    .sort((a, b) => a.title.localeCompare(b.title, 'ru')),
)
const crumbs = computed(() => {
  const result = []
  const seen = new Set<string>()
  let id = parent.value
  while (id && !seen.has(id)) {
    seen.add(id)
    const f = lib.folders.find((f) => f.id === id)
    if (!f) break
    result.unshift(f)
    id = f.parent
  }
  return result
})
const view = useViewPref('directory', 'list')
const viewOptions = [
  { value: 'list' as const, label: 'Списком', icon: List },
  { value: 'grid' as const, label: 'Карточками', icon: LayoutGrid },
]
const recent = computed(() =>
  lib.records
    .filter((r) => !r.secret_kind)
    .sort((a, b) => b.updated_at - a.updated_at)
    .slice(0, 4),
)
const countIn = (id: string) => {
  const folders = lib.folders.filter((f) => f.parent === id).length
  const recs = lib.countByFolder.get(id) ?? 0
  return [
    folders && `${folders} ${plural(folders, 'папка', 'папки', 'папок')}`,
    `${recs} ${plural(recs, 'запись', 'записи', 'записей')}`,
  ]
    .filter(Boolean)
    .join(' · ')
}
function plural(n: number, one: string, few: string, many: string) {
  const m10 = n % 10,
    m100 = n % 100
  if (m10 === 1 && m100 !== 11) return one
  if (m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14)) return few
  return many
}
const naming = ref(false)
const name = ref('')
const busy = ref(false)
async function createFolder() {
  if (!name.value.trim() || busy.value) return
  busy.value = true
  try {
    await api.createFolder(name.value.trim(), parent.value)
    await lib.changed()
    name.value = ''
    naming.value = false
  } catch (e) {
    toast.error(String(e))
  } finally {
    busy.value = false
  }
}
async function create() {
  try {
    await router.push(`/r/${await lib.createRecord({ folder: parent.value })}`)
  } catch (e) {
    toast.error(String(e))
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <header data-tauri-drag-region class="flex h-[52px] shrink-0 items-center gap-3 border-b pr-5 pl-7">
      <nav aria-label="Путь к папке" class="text-muted-foreground flex min-w-0 flex-1 items-center gap-1.5 text-[13px]">
        <RouterLink to="/" class="hover:text-foreground rounded-md px-1.5 py-1 font-mono">/</RouterLink>
        <template v-for="f in crumbs" :key="f.id">
          <ChevronRight class="size-3 shrink-0" />
          <RouterLink
            :to="`/folder/${f.id}`"
            :aria-current="f.id === folderId ? 'page' : undefined"
            class="hover:text-foreground truncate rounded-md px-1.5 py-1"
            :class="f.id === folderId && 'text-foreground font-semibold'"
            >{{ f.name }}</RouterLink
          >
        </template>
      </nav>
      <SegmentedControl v-model="view" :options="viewOptions" label="Вид" size="sm" icon-only />
      <Button variant="outline" size="sm" :disabled="missing" @click="naming = !naming"
        ><FolderPlus class="size-4" /> Папка</Button
      >
      <Button size="sm" :disabled="missing" @click="create"><Plus class="size-4" /> Запись</Button>
    </header>
    <div v-if="missing" class="text-muted-foreground p-8">Папка не найдена</div>
    <div v-else :key="parent ?? 'root'" class="anim-in flex flex-1 flex-col gap-7 overflow-auto px-9 pt-7 pb-10">
      <div class="flex items-baseline gap-3">
        <h1 class="text-[28px] font-semibold tracking-[-.025em]">{{ folder?.name ?? 'Файлы' }}</h1>
        <span class="text-muted-foreground text-[13px]">{{ folder ? countIn(folder.id) : 'корень хранилища' }}</span>
      </div>
      <form v-if="naming" class="bg-card flex items-center gap-2 rounded-xl border p-3" @submit.prevent="createFolder">
        <Input
          v-model="name"
          aria-label="Название папки"
          placeholder="Название папки"
          autofocus
          @keydown.esc="naming = false"
        />
        <Button size="sm" :disabled="!name.trim() || busy">Создать</Button>
        <Button type="button" size="sm" variant="ghost" @click="naming = false">Отмена</Button>
      </form>
      <section v-if="!folder && recent.length" class="flex flex-col gap-3">
        <h2 class="text-muted-foreground text-xs font-semibold tracking-wider uppercase">Недавние</h2>
        <div class="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3">
          <RecordCard v-for="r in recent" :key="r.id" :record="r" />
        </div>
      </section>
      <section v-if="children.length" class="flex flex-col gap-3">
        <h2 class="text-muted-foreground text-xs font-semibold tracking-wider uppercase">Папки</h2>
        <div class="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3">
          <RouterLink
            v-for="f in children"
            :key="f.id"
            :to="`/folder/${f.id}`"
            class="bg-card flex items-center gap-3 rounded-xl border p-3.5 transition-[transform,box-shadow] duration-300 ease-[cubic-bezier(.2,.8,.2,1)] hover:-translate-y-[3px] hover:shadow-[0_14px_28px_-16px_rgba(0,0,0,.28)]"
          >
            <svg width="34" height="34" viewBox="0 0 34 34" aria-hidden="true" class="shrink-0">
              <path
                d="M3 9a3 3 0 0 1 3-3h7l3 3h12a3 3 0 0 1 3 3v13a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3z"
                class="fill-[#D9D5CC] dark:fill-[#3A3832]"
              />
              <path d="M3 13h28v12a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3z" class="fill-[#E8E5DE] dark:fill-[#4A4740]" />
            </svg>
            <span class="flex min-w-0 flex-col gap-0.5">
              <span class="truncate text-[14px] font-semibold">{{ f.name }}</span>
              <span class="text-muted-foreground text-xs">{{ countIn(f.id) }}</span>
            </span>
          </RouterLink>
        </div>
      </section>
      <section class="flex flex-col gap-2">
        <h2 class="text-muted-foreground text-xs font-semibold tracking-wider uppercase">Записи</h2>
        <RecordList
          v-if="view === 'list'"
          :records="records"
          :empty="children.length ? 'Записей здесь нет' : 'Папка пуста — создайте запись или подпапку'"
        />
        <div v-else-if="records.length" class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3">
          <RecordCard v-for="r in records" :key="r.id" :record="r" />
        </div>
        <p v-else class="text-muted-foreground px-1 py-6 text-[13px]">Записей здесь нет</p>
      </section>
    </div>
  </div>
</template>
