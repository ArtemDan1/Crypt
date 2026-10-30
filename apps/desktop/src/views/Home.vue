<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { Plus } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import PageHeader from '@/components/PageHeader.vue'
import RecordList from '@/components/RecordList.vue'
import { api } from '@/lib/api'
import { useLibrary } from '@/stores/library'

const lib = useLibrary()
const router = useRouter()
const recent = computed(() => lib.records.filter((r) => !r.secret_kind).sort((a, b) => b.updated_at - a.updated_at))

async function addStarter() {
  for (const t of await api.starterTypes()) await api.putType(t)
  await lib.changed()
}
async function create() {
  router.push(`/r/${await lib.createRecord()}`)
}
</script>

<template>
  <div class="flex h-full flex-col">
    <PageHeader title="Недавние">
      <Button size="sm" @click="create"><Plus class="size-4" /> Новая запись</Button>
    </PageHeader>
    <div class="flex-1 overflow-auto px-3 py-3">
      <div
        v-if="!lib.types.length"
        class="mx-3 mb-4 flex items-center gap-3 rounded-xl border border-dashed p-4 text-[13px]"
      >
        <span class="flex-1 text-muted-foreground"
          >Типы задают поля записей: серверы, книги, подписки — что угодно.</span
        >
        <Button size="sm" variant="outline" @click="addStarter">Добавить тип «Сервер»</Button>
        <Button size="sm" variant="outline" @click="router.push('/types?new=1')">Создать свой</Button>
      </div>
      <RecordList :records="recent" empty="Записей пока нет — нажмите ⌘N" />
    </div>
  </div>
</template>
