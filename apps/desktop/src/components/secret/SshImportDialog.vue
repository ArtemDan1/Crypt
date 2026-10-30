<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount } from 'vue'
import { Check, LockKeyhole } from '@lucide/vue'
import { toast } from 'vue-sonner'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { api } from '@/lib/api'
import type { SshCandidate } from '@/lib/types'
import { defaultTitle, selectable } from '@/lib/sshImport'
import { useSession } from '@/stores/session'
import { useLibrary } from '@/stores/library'
const props = defineProps<{ open: boolean }>(),
  emit = defineEmits<{ 'update:open': [value: boolean]; imported: [] }>()
interface Row {
  candidate: SshCandidate
  selected: boolean
  title: string
  password: string
  error: string
  done: boolean
}
const rows = ref<Row[]>([]),
  loading = ref(false),
  busy = ref(false),
  lib = useLibrary()
let request = 0,
  active = true
const selected = computed(() => rows.value.filter((r) => r.selected && !r.done && selectable(r.candidate)))
function clear() {
  ++request
  for (const r of rows.value) r.password = ''
  rows.value = []
}
async function load() {
  clear()
  const n = request
  loading.value = true
  try {
    const candidates = await api.scanSshDir()
    if (active && props.open && n === request)
      rows.value = candidates.map((candidate) => ({
        candidate,
        selected: selectable(candidate),
        title: defaultTitle(candidate.fileName, candidate.info?.comment),
        password: '',
        error: '',
        done: false,
      }))
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось прочитать ~/.ssh')
  } finally {
    loading.value = false
  }
}
watch(
  () => props.open,
  (on) => {
    if (on) load()
    else clear()
  },
  { immediate: true },
)
const unhook = useSession().beforeLock(() => {
  clear()
  emit('update:open', false)
})
onBeforeUnmount(() => {
  active = false
  clear()
  unhook()
})
async function run() {
  if (busy.value || !selected.value.length) return
  busy.value = true
  const seq = request
  const current = [...selected.value]
  try {
    const result = await api.importSshKeys(
      current.map((r) => ({
        path: r.candidate.path,
        title: r.title.trim() || defaultTitle(r.candidate.fileName),
        passphrase: r.password || null,
      })),
    )
    if (active && props.open && seq === request)
      for (const item of result) {
        const row = rows.value.find((r) => r.candidate.path === item.path)
        if (row) {
          row.password = ''
          row.error = item.error?.message ?? ''
          row.done = !!item.recordId
          if (row.done) {
            row.selected = false
            row.candidate.alreadyImported = true
          }
        }
      }
    await lib.changed()
    emit('imported')
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось импортировать ключи')
  } finally {
    for (const r of current) r.password = ''
    busy.value = false
  }
}
</script>
<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)"
    ><DialogContent class="max-h-[85vh] overflow-auto sm:max-w-xl"
      ><DialogHeader
        ><DialogTitle>Импорт из ~/.ssh</DialogTitle
        ><DialogDescription>Выберите приватные ключи для зашифрованного хранилища.</DialogDescription></DialogHeader
      >
      <p v-if="loading" class="py-8 text-center text-sm text-muted-foreground">Поиск ключей…</p>
      <p v-else-if="!rows.length" class="py-8 text-center text-sm text-muted-foreground">Приватные ключи не найдены</p>
      <div
        v-for="row in rows"
        :key="row.candidate.path"
        class="space-y-2 border-b py-3"
        :class="row.candidate.alreadyImported && !row.done ? 'opacity-50' : ''"
      >
        <div class="flex items-center gap-3">
          <Checkbox
            v-model="row.selected"
            :disabled="busy || row.candidate.alreadyImported || row.done"
            :aria-label="`Импортировать ${row.candidate.fileName}`"
          /><span class="flex-1 font-mono text-sm">{{ row.candidate.fileName }}</span
          ><Check v-if="row.done" class="size-4 text-green-500" /><span
            v-else-if="row.candidate.alreadyImported"
            class="text-xs text-muted-foreground"
            >уже в Crypt</span
          ><LockKeyhole v-if="row.candidate.encrypted" class="size-3.5 text-secret" />
        </div>
        <template v-if="!row.candidate.alreadyImported && !row.done"
          ><input
            v-model="row.title"
            :disabled="busy"
            :aria-label="`Название ${row.candidate.fileName}`"
            class="w-full rounded-md border bg-transparent px-3 py-2 text-sm" /><input
            v-if="row.candidate.encrypted"
            v-model="row.password"
            :disabled="busy"
            type="password"
            autocomplete="off"
            :aria-label="`Пароль ${row.candidate.fileName}`"
            placeholder="Пароль ключа"
            class="w-full rounded-md border bg-transparent px-3 py-2 text-sm"
        /></template>
        <p v-if="row.candidate.info" class="truncate font-mono text-xs text-muted-foreground">
          {{ row.candidate.info.fingerprint }}
        </p>
        <p v-if="row.error" role="alert" class="text-xs text-destructive">{{ row.error }}</p>
      </div>
      <p class="text-xs leading-relaxed text-muted-foreground">
        Файлы в ~/.ssh не изменяются. Удалить их можно вручную, когда убедитесь, что агент Crypt работает.
      </p>
      <DialogFooter
        ><Button variant="ghost" :disabled="busy" @click="emit('update:open', false)">Закрыть</Button
        ><Button :disabled="busy || !selected.length" @click="run">{{
          busy ? 'Импортируем…' : `Импортировать (${selected.length})`
        }}</Button></DialogFooter
      ></DialogContent
    ></Dialog
  >
</template>
