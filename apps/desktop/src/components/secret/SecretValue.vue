<script setup lang="ts">
import { computed, nextTick, ref, watch, onBeforeUnmount } from 'vue'
import { Copy, Eye, EyeOff, Check, X } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import PasswordGenerator from './PasswordGenerator.vue'
import SecretHistory from './SecretHistory.vue'
import type { SecretFieldMeta } from '@/lib/secretFields'
import { api } from '@/lib/api'
import { useRevealTimer } from '@/lib/reveal'
import { useSession } from '@/stores/session'
import { passwordStrength } from '@/lib/strength'
const props = defineProps<{
    recordId: string
    meta: SecretFieldMeta
    filled: boolean
    publicValue?: string
    historyCount?: number
    readOnly?: boolean
  }>(),
  emit = defineEmits<{ saved: [] }>()
const editing = ref(false),
  draft = ref(''),
  visible = ref(false),
  busy = ref(false),
  historyOpen = ref(false),
  field = ref<HTMLInputElement | HTMLTextAreaElement | null>(null),
  { revealed, show, hide } = useRevealTimer()
let request = 0,
  active = true,
  inputTimer: ReturnType<typeof setTimeout> | undefined
watch(
  () => props.publicValue,
  (value) => {
    if (!editing.value) draft.value = value ?? ''
  },
  { immediate: true },
)
function cancel() {
  editing.value = false
  draft.value = ''
  visible.value = false
  if (inputTimer) clearTimeout(inputTimer)
  inputTimer = undefined
}
function toggleInput() {
  visible.value = !visible.value
  if (inputTimer) clearTimeout(inputTimer)
  if (visible.value)
    inputTimer = setTimeout(() => {
      visible.value = false
      inputTimer = undefined
    }, 30000)
}
const unhook = useSession().beforeLock(() => {
  ++request
  hide()
  cancel()
})
onBeforeUnmount(() => {
  active = false
  ++request
  unhook()
  cancel()
})
async function edit() {
  if (props.readOnly || editing.value) return
  draft.value = props.meta.sensitive ? (revealed.value ?? '') : (props.publicValue ?? '')
  editing.value = true
  visible.value = false
  hide()
  await nextTick()
  field.value?.focus()
}
const empty = computed(() => (props.meta.sensitive ? !props.filled : !props.publicValue))
const shown = computed(() =>
  empty.value ? 'не задано' : props.meta.sensitive ? (revealed.value ?? '••••••••') : props.publicValue,
)
// Strength is shown only for a value already on screen (revealed or being typed/generated).
const strength = computed(() => {
  if (props.meta.name !== 'password') return null
  const v = editing.value ? draft.value : revealed.value
  return v ? passwordStrength(v) : null
})
// Never let the generator overwrite an existing value: the field has to be cleared first.
const hasValue = computed(() => (editing.value ? draft.value.length > 0 : !empty.value))
const strengthTone = ['bg-destructive', 'bg-secret', 'bg-primary', 'bg-primary']
// Sensitive and multiline drafts are saved explicitly; plain values save on blur.
const explicit = computed(() => props.meta.sensitive || props.meta.multiline)
async function reveal() {
  const seq = ++request
  if (revealed.value !== null) {
    hide()
    return
  }
  try {
    const value = await api.revealSecret(props.recordId, props.meta.name)
    if (active && seq === request) show(value)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось показать значение')
  }
}
async function publicBlur(event: FocusEvent) {
  if (props.meta.sensitive || !editing.value) return
  const target = event.relatedTarget as HTMLElement | null
  if (target?.closest('[data-secret-edit]')) return
  await save()
}
async function save() {
  if (busy.value) return
  busy.value = true
  try {
    await api.setSecret(props.recordId, props.meta.name, draft.value || null)
    cancel()
    hide()
    emit('saved')
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось сохранить')
  } finally {
    busy.value = false
  }
}
async function copy() {
  try {
    await api.copySecret(props.recordId, props.meta.name)
    const settings = await api.getSettings()
    toast.success(`${props.meta.label}: скопировано · очистится через ${settings.clipboardClearSeconds} с`)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось скопировать')
  }
}
async function generated(value: string) {
  draft.value = value
  editing.value = true
  visible.value = false
  hide()
  await nextTick()
  field.value?.focus()
}
</script>
<template>
  <div class="border-b py-3 last:border-b-0">
    <div class="mb-1 flex items-center justify-between gap-2">
      <label class="text-xs text-muted-foreground">{{ meta.label }}</label
      ><button
        v-if="historyCount"
        class="text-xs text-muted-foreground hover:text-foreground"
        @click="historyOpen = true"
      >
        История ({{ historyCount }})
      </button>
    </div>
    <div
      class="flex min-h-9 gap-1"
      :class="editing && meta.multiline ? 'items-start' : 'items-center'"
      :data-secret-edit="editing || undefined"
    >
      <template v-if="editing">
        <textarea
          v-if="meta.multiline"
          ref="field"
          v-model="draft"
          rows="4"
          :aria-label="meta.label"
          class="min-w-0 flex-1 rounded-lg bg-transparent px-2.5 py-1.5 text-sm outline-none ring-1 ring-primary/60 ring-inset"
          :class="meta.mono ? 'font-mono' : ''"
          @keydown.esc.prevent="cancel"
          @keydown.meta.enter.prevent="save"
          @blur="publicBlur"
        /><input
          v-else
          ref="field"
          v-model="draft"
          :type="meta.sensitive && !visible ? 'password' : 'text'"
          :aria-label="meta.label"
          :placeholder="meta.sensitive && filled ? 'Новое значение' : 'не задано'"
          class="h-9 min-w-0 flex-1 rounded-lg bg-transparent px-2.5 text-sm outline-none ring-1 ring-primary/60 ring-inset placeholder:font-sans"
          :class="meta.mono || meta.sensitive ? 'font-mono' : ''"
          autocomplete="off"
          @keydown.enter.prevent="save"
          @keydown.esc.prevent="cancel"
          @blur="publicBlur"
        /><Button
          v-if="meta.sensitive && !meta.multiline"
          variant="ghost"
          size="sm"
          aria-label="Видимость ввода"
          @click="toggleInput"
          ><component :is="visible ? EyeOff : Eye" class="size-4" /></Button
        ><template v-if="explicit"
          ><Button variant="ghost" size="sm" aria-label="Сохранить" :disabled="busy" @click="save"
            ><Check class="size-4 text-primary" /></Button
          ><Button variant="ghost" size="sm" aria-label="Отмена" @click="cancel"><X class="size-4" /></Button
        ></template>
      </template>
      <template v-else>
        <button
          v-if="!readOnly"
          type="button"
          :aria-label="`Изменить ${meta.label}`"
          :title="explicit ? 'Нажмите, чтобы изменить' : undefined"
          class="min-h-8 min-w-0 flex-1 cursor-text rounded-lg px-2.5 py-1.5 text-left text-sm break-all whitespace-pre-wrap transition-colors"
          :class="[
            empty ? 'text-muted-foreground' : meta.mono || meta.sensitive ? 'font-mono' : '',
            meta.sensitive && !empty
              ? 'bg-secret-bg text-secret-fg hover:brightness-[.97]'
              : 'hover:bg-foreground/[.05]',
          ]"
          @click="edit"
        >
          <span
            :key="revealed ?? 'mask'"
            :class="
              meta.sensitive && revealed !== null ? 'anim-reveal' : meta.sensitive && !empty ? 'tracking-[.14em]' : ''
            "
            >{{ shown }}</span
          ></button
        ><span
          v-else
          class="min-w-0 flex-1 rounded-lg px-2.5 py-1.5 text-sm break-all whitespace-pre-wrap"
          :class="[
            empty ? 'text-muted-foreground' : meta.mono || meta.sensitive ? 'font-mono' : '',
            meta.sensitive && !empty ? 'bg-secret-bg text-secret-fg' : '',
          ]"
          ><span :key="revealed ?? 'mask'" :class="meta.sensitive && revealed !== null && 'anim-reveal'">{{
            shown
          }}</span></span
        ><Button
          v-if="meta.sensitive && filled && meta.name !== 'totp_seed'"
          variant="ghost"
          size="sm"
          :aria-label="`${revealed === null ? 'Показать' : 'Скрыть'} ${meta.label}`"
          @click="reveal"
          ><component :is="revealed === null ? Eye : EyeOff" class="size-4" /></Button
        ><Button
          v-if="(filled || publicValue) && meta.name !== 'totp_seed'"
          variant="ghost"
          size="sm"
          :aria-label="`Копировать ${meta.label}`"
          @click="copy"
          ><Copy class="size-4"
        /></Button>
      </template>
      <PasswordGenerator v-if="meta.generator && !readOnly" :disabled="hasValue" @use="generated" />
    </div>
    <div v-if="strength" class="mt-1.5 flex items-center gap-2">
      <span class="bg-muted h-1 w-28 overflow-hidden rounded-full"
        ><span
          class="block h-1 rounded-full transition-[width] duration-500 ease-[cubic-bezier(.2,.8,.2,1)]"
          :class="strengthTone[strength.score]"
          :style="{ width: `${(strength.score + 1) * 25}%` }"
      /></span>
      <span class="text-muted-foreground text-[11px]">{{ strength.label }}</span>
    </div>
    <SecretHistory :record-id="recordId" :name="meta.name" :label="meta.label" v-model:open="historyOpen" />
  </div>
</template>
