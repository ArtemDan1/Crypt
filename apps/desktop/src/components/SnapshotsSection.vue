<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { Camera, RotateCcw, Trash2 } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { api, CryptApiError } from '@/lib/api'
import type { SnapshotInfo } from '@/lib/types'
import { useLibrary } from '@/stores/library'

const lib = useLibrary()
const list = ref<SnapshotInfo[]>([])
const name = ref('')
const busy = ref(false)
/** Snapshot id awaiting a second click, with the action it confirms. */
const confirm = ref<{ id: string; action: 'restore' | 'delete' } | null>(null)
const fmt = new Intl.DateTimeFormat('ru', {
  day: 'numeric',
  month: 'long',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})

async function run<T>(fn: () => Promise<T>): Promise<T | undefined> {
  busy.value = true
  try {
    return await fn()
  } catch (e) {
    toast.error(e instanceof CryptApiError ? e.message : String(e))
  } finally {
    busy.value = false
    confirm.value = null
  }
}
const refresh = () => run(async () => (list.value = await api.listSnapshots()))
onMounted(refresh)

async function create() {
  const n = name.value.trim()
  if (!n) return
  await run(async () => {
    await api.createSnapshot(n)
    name.value = ''
    toast(`Снимок «${n}» создан`)
    list.value = await api.listSnapshots()
  })
}
async function act(s: SnapshotInfo, action: 'restore' | 'delete') {
  if (confirm.value?.id !== s.id || confirm.value.action !== action) {
    confirm.value = { id: s.id, action }
    return
  }
  if (action === 'delete')
    await run(async () => {
      await api.deleteSnapshot(s.id)
      list.value = await api.listSnapshots()
    })
  else
    await run(async () => {
      const r = await api.restoreSnapshot(s.id)
      await lib.changed()
      const parts = [`возвращено записей: ${r.restored}`]
      if (r.trashed) parts.push(`новые в корзине: ${r.trashed}`)
      if (r.removed) parts.push(`удалено импортированных секретов: ${r.removed}`)
      if (r.missing) parts.push(`стёрты навсегда и не вернутся: ${r.missing}`)
      toast(`Хранилище откачено к «${s.name}»`, { description: parts.join(' · ') })
    })
}
const isConfirming = (s: SnapshotInfo, action: 'restore' | 'delete') =>
  confirm.value?.id === s.id && confirm.value.action === action
</script>

<template>
  <section class="divide-y rounded-xl border">
    <div class="px-4 py-3.5">
      <div class="flex items-center gap-3">
        <Camera class="text-muted-foreground size-5" />
        <div class="flex-1">
          <div class="font-medium">Снимки хранилища</div>
          <div class="text-muted-foreground text-xs">
            Точка, к которой можно откатить все записи. Откат добавляет версии, а заметки, созданные позже, кладёт в
            корзину. Импортированные позже секреты, которые вы не меняли, удаляются; созданные вручную остаются.
            Значения секретов, типы и папки не откатываются.
          </div>
        </div>
      </div>
      <form class="mt-3 flex gap-2" @submit.prevent="create">
        <Input v-model="name" placeholder="Например: перед импортом" aria-label="Название снимка" maxlength="80" />
        <Button size="sm" variant="outline" :disabled="busy || !name.trim()">Создать снимок</Button>
      </form>
    </div>
    <div v-for="s in list" :key="s.id" class="flex flex-wrap items-center gap-2 px-4 py-3">
      <div class="min-w-0 flex-1">
        <div class="truncate text-sm font-medium">{{ s.name }}</div>
        <div class="text-muted-foreground text-xs">{{ fmt.format(s.created_at) }} · записей: {{ s.records }}</div>
      </div>
      <Button
        size="sm"
        :variant="isConfirming(s, 'restore') ? 'default' : 'outline'"
        :disabled="busy"
        @click="act(s, 'restore')"
        ><RotateCcw class="size-3.5" />{{ isConfirming(s, 'restore') ? 'Точно откатить?' : 'Откатить' }}</Button
      >
      <Button
        size="sm"
        :variant="isConfirming(s, 'delete') ? 'destructive' : 'ghost'"
        :disabled="busy"
        :aria-label="`Удалить снимок ${s.name}`"
        @click="act(s, 'delete')"
        ><Trash2 class="size-3.5" /><template v-if="isConfirming(s, 'delete')">Удалить?</template></Button
      >
    </div>
  </section>
</template>
