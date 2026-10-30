<script setup lang="ts">
import { ref, watch } from 'vue'
import { Trash2 } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import PageHeader from '@/components/PageHeader.vue'
import { api, CryptApiError } from '@/lib/api'
import { typeIcon } from '@/lib/icons'
import type { RecordSummary } from '@/lib/types'
import { useLibrary } from '@/stores/library'

const lib = useLibrary()
const items = ref<RecordSummary[]>([])
const confirmId = ref<string | null>(null)
const confirmAll = ref(false)
const fmt = new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })

async function load() {
  items.value = await api.listRecords({ trashed: true })
}
watch(() => lib.version, load, { immediate: true })

const emptyAll = () => run(() => api.emptyTrash(), 'Корзина очищена')
const restore = (id: string) => run(() => api.restoreRecord(id), 'Запись восстановлена')
const purge = (id: string) => run(() => api.purgeRecord(id), 'Удалено навсегда')

async function run(fn: () => Promise<unknown>, msg: string) {
  try {
    await fn()
    await lib.changed()
    toast(msg)
  } catch (e) {
    toast.error(e instanceof CryptApiError ? e.message : String(e))
  } finally {
    confirmId.value = null
    confirmAll.value = false
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <PageHeader title="Корзина" :sub="`${items.length}`">
      <template #icon><Trash2 class="text-muted-foreground size-5" /></template>
      <template v-if="items.length">
        <Button v-if="!confirmAll" variant="outline" size="sm" @click="confirmAll = true"
          >Очистить корзину ({{ items.length }})</Button
        >
        <template v-else>
          <span class="text-muted-foreground text-[13px]">Удалить всё навсегда?</span>
          <Button variant="destructive" size="sm" @click="emptyAll">Удалить</Button>
          <Button variant="ghost" size="sm" @click="confirmAll = false">Отмена</Button>
        </template>
      </template>
    </PageHeader>
    <div class="flex-1 overflow-auto p-3">
      <div v-for="r in items" :key="r.id" class="hover:bg-card/60 flex h-12 items-center gap-3 rounded-lg px-3">
        <component :is="typeIcon(lib.typeById.get(r.type_id ?? '')?.icon)" class="text-muted-foreground size-4" />
        <span class="min-w-0 flex-1 truncate font-medium">{{ r.title || 'Без названия' }}</span>
        <span class="text-muted-foreground text-xs">{{ fmt.format(r.updated_at) }}</span>
        <Button variant="outline" size="sm" @click="restore(r.id)">Восстановить</Button>
        <Button v-if="confirmId !== r.id" variant="ghost" size="sm" class="text-destructive" @click="confirmId = r.id"
          >Удалить навсегда</Button
        >
        <Button v-else variant="destructive" size="sm" @click="purge(r.id)">Точно удалить</Button>
      </div>
      <p v-if="!items.length" class="text-muted-foreground px-3 py-6 text-[13px]">Корзина пуста</p>
    </div>
  </div>
</template>
