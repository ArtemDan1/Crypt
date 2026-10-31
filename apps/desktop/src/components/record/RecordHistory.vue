<script setup lang="ts">
import { VERSION_KIND as KIND } from '@/lib/versionKinds'
import { computed, ref, watch } from 'vue'
import { History, Monitor, RotateCcw, Smartphone } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { api, CryptApiError } from '@/lib/api'
import { diffLines } from '@/lib/diff'
import { formatValue } from '@/lib/fields'
import type { FieldValue, RecordVersion, RecordView } from '@/lib/types'
import { useLibrary } from '@/stores/library'

const props = defineProps<{ current: RecordView }>()
const open = defineModel<boolean>('open', { required: true })
const emit = defineEmits<{ reverted: [] }>()
const lib = useLibrary()

const fmt = new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })

const versions = ref<RecordVersion[]>([])
const selected = ref<RecordVersion | null>(null)
const old = ref<RecordView | null>(null)
const confirming = ref(false)
const busy = ref(false)
let seq = 0

function fail(e: unknown) {
  toast.error(e instanceof CryptApiError ? e.message : String(e))
}
async function refresh() {
  try {
    versions.value = await api.recordVersions(props.current.id)
    await pick(versions.value[1] ?? versions.value[0] ?? null)
  } catch (e) {
    fail(e)
  }
}
watch(open, (on) => {
  if (on) refresh()
  else {
    selected.value = null
    old.value = null
  }
})
async function pick(v: RecordVersion | null) {
  const n = ++seq
  selected.value = v
  confirming.value = false
  if (!v) return
  try {
    const view = await api.recordAtVersion(props.current.id, v.id)
    if (n === seq) old.value = view
  } catch (e) {
    fail(e)
  }
}
const isLatest = computed(() => selected.value?.id === versions.value[0]?.id)

// What "Вернуть" would change: current → selected version.
const typeOf = computed(() => (old.value?.type_id ? lib.typeById.get(old.value.type_id) : undefined))
const show = (v: FieldValue | undefined, id: string) => {
  if (!v) return '—'
  const field =
    typeOf.value?.fields.find((f) => f.id === id) ??
    lib.typeById.get(props.current.type_id ?? '')?.fields.find((f) => f.id === id)
  return (field ? formatValue(v, field.kind, { titleOf: (x) => lib.titleById.get(x) }) : JSON.stringify(v.value)) || '—'
}
const fieldName = (id: string) =>
  [typeOf.value, lib.typeById.get(props.current.type_id ?? '')].flatMap((t) => t?.fields ?? []).find((f) => f.id === id)
    ?.name ?? 'Удалённое поле'
const changes = computed(() => {
  const o = old.value
  const c = props.current
  if (!o) return []
  const out: { label: string; now: string; then: string }[] = []
  if (o.title !== c.title)
    out.push({ label: 'Название', now: c.title || 'Без названия', then: o.title || 'Без названия' })
  if (o.type_id !== c.type_id)
    out.push({
      label: 'Тип',
      now: lib.typeById.get(c.type_id ?? '')?.name ?? 'Без типа',
      then: lib.typeById.get(o.type_id ?? '')?.name ?? (o.type_id ? 'Удалённый тип' : 'Без типа'),
    })
  if (o.tags.join() !== c.tags.join())
    out.push({
      label: 'Теги',
      now: c.tags.map((t) => `#${t}`).join(' ') || '—',
      then: o.tags.map((t) => `#${t}`).join(' ') || '—',
    })
  const nowFields = { ...c.hidden_fields, ...c.fields }
  const oldFields = { ...o.hidden_fields, ...o.fields }
  for (const id of new Set([...Object.keys(nowFields), ...Object.keys(oldFields)]))
    if (JSON.stringify(nowFields[id]) !== JSON.stringify(oldFields[id]))
      out.push({ label: fieldName(id), now: show(nowFields[id], id), then: show(oldFields[id], id) })
  return out
})
const body = computed(() => (old.value ? diffLines(props.current.body, old.value.body) : []))
const bodyChanged = computed(() => body.value.some((p) => p.op !== 'same'))

async function revert() {
  if (!selected.value) return
  if (!confirming.value) {
    confirming.value = true
    return
  }
  busy.value = true
  try {
    await api.revertRecord(props.current.id, selected.value.id)
    toast('Версия возвращена — прежнее состояние осталось в истории')
    open.value = false
    emit('reverted')
  } catch (e) {
    fail(e)
  } finally {
    busy.value = false
    confirming.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="flex h-[80vh] max-h-[720px] flex-col gap-0 p-0 sm:max-w-4xl">
      <DialogHeader class="border-b px-5 py-4">
        <DialogTitle class="flex items-center gap-2"><History class="size-4" /> История версий</DialogTitle>
        <DialogDescription>Возврат создаёт новую версию — ничего не теряется.</DialogDescription>
      </DialogHeader>
      <div class="flex min-h-0 flex-1">
        <ol class="w-60 shrink-0 overflow-y-auto border-r p-2" aria-label="Версии">
          <li v-for="(v, i) in versions" :key="v.id">
            <button
              class="flex w-full flex-col items-start rounded-md px-2.5 py-2 text-left text-[13px]"
              :class="selected?.id === v.id ? 'bg-accent' : 'hover:bg-accent/60'"
              :aria-current="selected?.id === v.id ? 'true' : undefined"
              @click="pick(v)"
            >
              <span class="font-medium"
                >{{ fmt.format(v.at) }}<span v-if="i === 0" class="text-secret"> · сейчас</span></span
              >
              <span class="text-muted-foreground flex items-center gap-1 text-xs">
                <component :is="v.this_device ? Monitor : Smartphone" class="size-3" />
                {{ KIND[v.kind] ?? 'Изменения' }}
              </span>
            </button>
          </li>
        </ol>
        <section class="flex min-w-0 flex-1 flex-col">
          <div v-if="!selected || !old" class="text-muted-foreground m-auto text-[13px]">Выберите версию</div>
          <template v-else>
            <div class="flex-1 space-y-5 overflow-y-auto p-5">
              <p v-if="isLatest" class="text-muted-foreground text-[13px]">Это текущее состояние записи.</p>
              <p v-else-if="!changes.length && !bodyChanged" class="text-muted-foreground text-[13px]">
                Видимое содержимое не отличается от текущего{{
                  selected.kind === 'secret'
                    ? ': менялись секретные поля, их прошлые значения — в «Истории» самого поля'
                    : ''
                }}.
              </p>
              <section v-if="!isLatest && changes.length">
                <h4 class="text-muted-foreground mb-2 text-xs font-medium tracking-wider uppercase">Поля</h4>
                <dl class="grid grid-cols-[140px_1fr] gap-x-3 gap-y-2 text-[13px]">
                  <template v-for="c in changes" :key="c.label">
                    <dt class="text-muted-foreground">{{ c.label }}</dt>
                    <dd class="min-w-0 break-words">
                      <span class="bg-red-500/10 text-red-700 line-through decoration-red-400/60 dark:text-red-300">{{
                        c.now
                      }}</span>
                      →
                      <span class="bg-emerald-500/10 text-emerald-700 dark:text-emerald-300">{{ c.then }}</span>
                    </dd>
                  </template>
                </dl>
              </section>
              <section v-if="!isLatest && bodyChanged">
                <h4 class="text-muted-foreground mb-2 text-xs font-medium tracking-wider uppercase">Текст</h4>
                <pre class="overflow-x-auto rounded-md border font-sans text-[13px] leading-relaxed"><div
                  v-for="(p, i) in body"
                  :key="i"
                  class="px-3 whitespace-pre-wrap"
                  :class="
                    p.op === 'add'
                      ? 'bg-emerald-500/10 text-emerald-800 dark:text-emerald-200'
                      : p.op === 'del'
                        ? 'bg-red-500/10 text-red-800 line-through decoration-red-400/50 dark:text-red-200'
                        : 'text-muted-foreground'
                  "
                ><span class="select-none opacity-60">{{ p.op === 'add' ? '+ ' : p.op === 'del' ? '− ' : '  ' }}</span>{{ p.text || ' ' }}</div></pre>
                <p class="text-muted-foreground mt-1.5 text-xs">− сейчас · + в этой версии</p>
              </section>
            </div>
            <footer class="flex items-center justify-end gap-2 border-t px-5 py-3">
              <span v-if="confirming" class="text-muted-foreground mr-auto text-[13px]"
                >Текущее содержимое заменится этой версией.</span
              >
              <Button v-if="confirming" variant="ghost" size="sm" @click="confirming = false">Отмена</Button>
              <Button size="sm" :disabled="isLatest || busy" @click="revert"
                ><RotateCcw class="size-3.5" />{{ confirming ? 'Да, вернуть' : 'Вернуть эту версию' }}</Button
              >
            </footer>
          </template>
        </section>
      </div>
    </DialogContent>
  </Dialog>
</template>
