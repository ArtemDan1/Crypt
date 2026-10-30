<script setup lang="ts">
import { ref, watch } from 'vue'
import { Eye, Copy } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { api } from '@/lib/api'
import type { SecretRevisionMeta } from '@/lib/types'
import { useRevealTimer } from '@/lib/reveal'
const props = defineProps<{ recordId: string; name: string; label: string; open: boolean }>(),
  emit = defineEmits<{ 'update:open': [value: boolean] }>()
const items = ref<SecretRevisionMeta[]>([]),
  selected = ref<number | null>(null),
  { revealed, show, hide } = useRevealTimer()
let seq = 0
watch(
  () => props.open,
  async (on) => {
    const n = ++seq
    hide()
    items.value = []
    if (on)
      try {
        const h = await api.secretHistory(props.recordId, props.name)
        if (n === seq) items.value = h
      } catch (e) {
        toast.error(e instanceof Error ? e.message : 'Не удалось загрузить историю')
      }
  },
)
async function reveal(index: number) {
  const n = ++seq
  hide()
  try {
    const value = await api.revealSecretRevision(props.recordId, props.name, index)
    if (n === seq && props.open) {
      selected.value = index
      show(value)
    }
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось показать версию')
  }
}
async function copy(index: number) {
  try {
    await api.copySecretRevision(props.recordId, props.name, index)
    const settings = await api.getSettings()
    toast(`Скопировано, буфер очистится через ${settings.clipboardClearSeconds} с`)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось скопировать')
  }
}
const date = new Intl.DateTimeFormat('ru', { dateStyle: 'medium', timeStyle: 'short' })
</script>
<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)"
    ><DialogContent class="max-h-[80vh] overflow-auto"
      ><DialogHeader
        ><DialogTitle>История · {{ label }}</DialogTitle
        ><DialogDescription>Предыдущие значения хранятся в зашифрованном хранилище.</DialogDescription></DialogHeader
      >
      <div v-for="r in items" :key="r.index" class="space-y-2 border-b py-3">
        <div class="flex items-center justify-between gap-2">
          <span class="text-sm">Заменено {{ date.format(r.replacedAt) }}</span>
          <div v-if="name !== 'totp_seed'" class="flex">
            <Button variant="ghost" size="sm" aria-label="Показать версию" @click="reveal(r.index)"
              ><Eye class="size-4" /></Button
            ><Button variant="ghost" size="sm" aria-label="Копировать версию" @click="copy(r.index)"
              ><Copy class="size-4"
            /></Button>
          </div>
        </div>
        <pre
          v-if="selected === r.index && revealed !== null"
          class="whitespace-pre-wrap break-all font-mono text-sm text-secret"
          >{{ revealed }}</pre>
      </div>
      <p v-if="!items.length" class="py-4 text-sm text-muted-foreground">Предыдущих значений нет</p></DialogContent
    ></Dialog
  >
</template>
