<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { Check, Copy } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Switch } from '@/components/ui/switch'
import LinkPicker from '@/components/record/LinkPicker.vue'
import { inputValue, optionClass, optionsOf, parseInput } from '@/lib/fields'
import type { FieldDef, FieldValue, Id } from '@/lib/types'

const props = defineProps<{ field: FieldDef; value?: FieldValue; recordId?: Id; autofocus?: boolean }>()
const emit = defineEmits<{ change: [v: FieldValue | null]; done: [] }>()

const draft = ref(inputValue(props.value))
async function copyValue() {
  try {
    await navigator.clipboard.writeText(draft.value)
    toast.success(`${props.field.name}: скопировано`)
  } catch {
    toast.error('Не удалось скопировать')
  }
}
const textInput = ref<HTMLInputElement | null>(null)
// `autofocus` only works on page load; focus explicitly when mounted inside a table cell.
onMounted(async () => {
  if (!props.autofocus) return
  await nextTick()
  textInput.value?.focus()
  textInput.value?.select()
})
watch(
  () => props.value,
  (v) => (draft.value = inputValue(v)),
)

const kind = computed(() => props.field.kind)
const isText = computed(() => ['Text', 'Url', 'Number'].includes(kind.value.kind))
const options = computed(() => optionsOf(kind.value))
const selected = computed<Id[]>(() =>
  props.value?.kind === 'Select' ? [props.value.value] : props.value?.kind === 'MultiSelect' ? props.value.value : [],
)
const linkIds = computed<Id[]>(() => (props.value?.kind === 'Links' ? props.value.value : []))

function commitText() {
  if (draft.value === inputValue(props.value)) return emit('done')
  const r = parseInput(draft.value, kind.value)
  if (r && 'error' in r) {
    toast.error(r.error)
    draft.value = inputValue(props.value)
    return
  }
  emit('change', r)
  emit('done')
}
function cancel() {
  draft.value = inputValue(props.value)
  emit('done')
}
function toggleOption(id: Id) {
  if (kind.value.kind === 'Select') {
    emit('change', selected.value[0] === id ? null : { kind: 'Select', value: id })
  } else {
    const next = selected.value.includes(id) ? selected.value.filter((x) => x !== id) : [...selected.value, id]
    emit('change', next.length ? { kind: 'MultiSelect', value: next } : null)
  }
}
</script>

<template>
  <div v-if="isText" class="group/field relative flex items-center">
    <input
      ref="textInput"
      v-model="draft"
      class="hover:bg-foreground/[.04] focus:bg-foreground/[.04] h-[34px] w-full rounded-lg bg-transparent px-2 pr-9 transition-colors outline-none"
      :class="kind.kind !== 'Text' && 'font-mono text-[13px]'"
      :inputmode="kind.kind === 'Number' ? 'decimal' : undefined"
      :aria-label="field.name"
      placeholder="Пусто"
      @keydown.enter.prevent="commitText"
      @keydown.esc="cancel"
      @blur="commitText"
    />
    <button
      v-if="draft && (kind.kind === 'Text' || kind.kind === 'Url')"
      type="button"
      class="text-muted-foreground hover:bg-foreground/[.08] absolute right-1 flex size-7 items-center justify-center rounded-md opacity-0 transition-opacity group-hover/field:opacity-100 focus-visible:opacity-100"
      :aria-label="`Скопировать ${field.name}`"
      @click="copyValue"
    >
      <Copy class="size-3.5" />
    </button>
  </div>
  <input
    v-else-if="kind.kind === 'Date'"
    ref="textInput"
    v-model="draft"
    type="date"
    :aria-label="field.name"
    class="hover:bg-accent/50 h-[30px] rounded-md bg-transparent px-2 font-mono text-[13px] outline-none dark:[color-scheme:dark]"
    @change="commitText"
    @keydown.esc="cancel"
  />
  <div v-else-if="kind.kind === 'Checkbox'" class="flex h-[30px] items-center px-2">
    <Switch
      :model-value="value?.kind === 'Checkbox' && value.value"
      :aria-label="field.name"
      @update:model-value="(v: boolean) => emit('change', { kind: 'Checkbox', value: v })"
    />
  </div>
  <Popover v-else-if="kind.kind === 'Select' || kind.kind === 'MultiSelect'">
    <PopoverTrigger as-child>
      <button
        class="hover:bg-accent/50 flex min-h-[30px] w-full flex-wrap items-center gap-1 rounded-md px-2 text-left"
      >
        <span
          v-for="o in options.filter((o) => selected.includes(o.id))"
          :key="o.id"
          class="rounded-full px-2 py-0.5 text-xs"
          :class="optionClass(o.color)"
          >{{ o.label }}</span
        >
        <span v-if="!selected.length" class="text-muted-foreground">Пусто</span>
      </button>
    </PopoverTrigger>
    <PopoverContent align="start" class="w-56 p-1">
      <button
        v-for="o in options"
        :key="o.id"
        class="hover:bg-accent flex w-full items-center gap-2 rounded px-2 py-1.5 text-left"
        @click="toggleOption(o.id)"
      >
        <span class="rounded-full px-2 py-0.5 text-xs" :class="optionClass(o.color)">{{ o.label }}</span>
        <span class="flex-1" />
        <Check v-if="selected.includes(o.id)" class="size-3.5" />
      </button>
      <p v-if="!options.length" class="text-muted-foreground px-2 py-1.5 text-xs">
        Варианты задаются в настройках типа
      </p>
    </PopoverContent>
  </Popover>
  <LinkPicker
    v-else-if="kind.kind === 'RecordLink' || kind.kind === 'SecretLink'"
    :value="linkIds"
    :multiple="kind.value.multiple"
    :secrets="kind.kind === 'SecretLink'"
    :target-type="kind.kind === 'RecordLink' ? kind.value.target_type : null"
    :self-id="recordId"
    @change="(ids) => emit('change', ids.length ? { kind: 'Links', value: ids } : null)"
  />
</template>
