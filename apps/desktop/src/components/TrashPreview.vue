<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { RotateCcw } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { api, CryptApiError } from '@/lib/api'
import { formatValue } from '@/lib/fields'
import { WIKILINK_RE, linkLabel } from '@/lib/wikilinks'
import type { RecordSummary, RecordView } from '@/lib/types'
import { useLibrary } from '@/stores/library'

// Read-only look at a trashed record, so it can be checked before restoring or purging.
const props = defineProps<{ record: RecordSummary | null }>()
const emit = defineEmits<{ restore: [RecordSummary] }>()
const open = defineModel<boolean>('open', { required: true })
const lib = useLibrary()
const view = ref<RecordView | null>(null)
const fmt = new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'long', hour: '2-digit', minute: '2-digit' })

watch(
  () => [open.value, props.record?.id] as const,
  async ([on, id]) => {
    view.value = null
    if (!on || !id) return
    try {
      const v = await api.getRecord(id)
      if (props.record?.id === id) view.value = v
    } catch (e) {
      toast.error(e instanceof CryptApiError ? e.message : String(e))
    }
  },
  { immediate: true },
)

const type = computed(() => (view.value?.type_id ? lib.typeById.get(view.value.type_id) : undefined))
const fields = computed(() => {
  const v = view.value
  if (!v || !type.value) return []
  return type.value.fields
    .map((f) => ({
      name: f.name,
      value: formatValue(v.fields[f.id], f.kind, { titleOf: (x) => lib.titleById.get(x) }),
    }))
    .filter((f) => f.value)
})
// Wikilinks read as their titles, not raw ids.
const body = computed(() =>
  (view.value?.body ?? '').replace(
    WIKILINK_RE,
    (_, id: string, alias?: string) => linkLabel(id, alias, (x) => lib.titleById.get(x)).text,
  ),
)
const folder = computed(() =>
  view.value?.folder ? lib.folders.find((f) => f.id === view.value!.folder)?.name : undefined,
)
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="flex max-h-[80vh] flex-col gap-0 p-0 sm:max-w-2xl">
      <DialogHeader class="border-b px-5 py-4 pr-12">
        <DialogTitle class="truncate">{{ record?.title || 'Без названия' }}</DialogTitle>
        <DialogDescription>
          В корзине{{ folder ? ` · из «${folder}»` : '' }}{{ type ? ` · ${type.name}` : '' }}
          <template v-if="view?.deleted_at"> · удалено {{ fmt.format(view.deleted_at) }}</template>
        </DialogDescription>
      </DialogHeader>
      <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
        <div v-if="!view" class="text-muted-foreground py-10 text-center text-[13px]">Загрузка…</div>
        <template v-else>
          <dl v-if="fields.length" class="mb-4 grid grid-cols-[minmax(100px,auto)_1fr] gap-x-4 gap-y-1.5 text-[13px]">
            <template v-for="f in fields" :key="f.name">
              <dt class="text-muted-foreground truncate">{{ f.name }}</dt>
              <dd class="min-w-0 break-words">{{ f.value }}</dd>
            </template>
          </dl>
          <div v-if="view.tags.length" class="mb-4 flex flex-wrap gap-1">
            <span v-for="t in view.tags" :key="t" class="bg-accent rounded-full px-2 py-0.5 text-xs">#{{ t }}</span>
          </div>
          <p
            v-if="body.trim()"
            class="text-foreground-2 text-[13.5px] leading-relaxed break-words whitespace-pre-wrap"
            v-text="body"
          />
          <p v-else-if="!fields.length" class="text-muted-foreground py-6 text-center text-[13px]">Запись пустая</p>
        </template>
      </div>
      <div class="flex justify-end gap-2 border-t px-5 py-3">
        <Button variant="ghost" size="sm" @click="open = false">Закрыть</Button>
        <Button v-if="record" size="sm" @click="emit('restore', record)"
          ><RotateCcw class="size-3.5" /> Восстановить</Button
        >
      </div>
    </DialogContent>
  </Dialog>
</template>
