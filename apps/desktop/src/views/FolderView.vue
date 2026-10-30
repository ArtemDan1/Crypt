<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { Folder, Plus } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import PageHeader from '@/components/PageHeader.vue'
import RecordList from '@/components/RecordList.vue'
import { useLibrary } from '@/stores/library'

const props = defineProps<{ folderId: string }>()
const lib = useLibrary()
const router = useRouter()
const folder = computed(() => lib.folders.find((f) => f.id === props.folderId))
const records = computed(() => lib.records.filter((r) => r.folder === props.folderId && !r.secret_kind))
async function create() {
  router.push(`/r/${await lib.createRecord({ folder: props.folderId })}`)
}
</script>

<template>
  <div class="flex h-full flex-col">
    <PageHeader :title="folder?.name ?? 'Папка'" :sub="`${records.length}`">
      <template #icon><Folder class="text-muted-foreground size-5" /></template>
      <Button size="sm" @click="create"><Plus class="size-4" /> Запись в папке</Button>
    </PageHeader>
    <div class="flex-1 overflow-auto p-3"><RecordList :records="records" /></div>
  </div>
</template>
