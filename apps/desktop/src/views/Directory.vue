<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowUp, Folder, FolderPlus, Plus } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import PageHeader from '@/components/PageHeader.vue'
import RecordList from '@/components/RecordList.vue'
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
    <PageHeader :title="folder?.name ?? (missing ? 'Папка не найдена' : 'Файлы')">
      <Button variant="outline" size="sm" :disabled="missing" @click="naming = !naming"
        ><FolderPlus class="size-4" /> Новая папка</Button
      >
      <Button size="sm" :disabled="missing" @click="create"><Plus class="size-4" /> Новая запись</Button>
    </PageHeader>
    <nav aria-label="Путь к папке" class="flex min-h-11 flex-wrap items-center gap-2 border-b px-6 py-2 text-[13px]">
      <Button
        v-if="folder"
        variant="ghost"
        size="icon-sm"
        aria-label="На уровень выше"
        @click="router.push(folder.parent ? `/folder/${folder.parent}` : '/')"
        ><ArrowUp class="size-4"
      /></Button>
      <RouterLink to="/" class="text-muted-foreground hover:text-foreground font-mono">/</RouterLink>
      <template v-for="f in crumbs" :key="f.id">
        <RouterLink
          :to="`/folder/${f.id}`"
          :aria-current="f.id === folderId ? 'page' : undefined"
          class="hover:underline"
          >{{ f.name }}</RouterLink
        >
        <span class="text-muted-foreground">/</span>
      </template>
    </nav>
    <div v-if="!missing" class="flex-1 overflow-auto p-3">
      <form v-if="naming" class="mb-3 flex items-center gap-2 rounded-lg border p-3" @submit.prevent="createFolder">
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
      <RouterLink
        v-for="f in children"
        :key="f.id"
        :to="`/folder/${f.id}`"
        class="hover:bg-accent/60 flex h-11 items-center gap-3 rounded-lg px-3"
      >
        <Folder class="text-muted-foreground size-4" /><span class="flex-1 truncate font-medium">{{ f.name }}</span>
        <span class="text-muted-foreground text-xs">Папка</span>
      </RouterLink>
      <RecordList :records="records" :empty="children.length ? ' ' : 'Папка пуста — создайте запись или подпапку'" />
    </div>
  </div>
</template>
