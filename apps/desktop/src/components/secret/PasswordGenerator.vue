<script setup lang="ts">
import { ref, watch } from 'vue'
import { WandSparkles, RefreshCw } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Switch } from '@/components/ui/switch'
import { Button } from '@/components/ui/button'
import { api } from '@/lib/api'
import { useRevealTimer } from '@/lib/reveal'
import type { PasswordOptions } from '@/lib/types'
const props = defineProps<{ disabled?: boolean }>()
const emit = defineEmits<{ use: [value: string] }>()
const defaults: PasswordOptions = {
  length: 24,
  lower: true,
  upper: true,
  digits: true,
  symbols: true,
  avoidAmbiguous: false,
}
function read(): PasswordOptions {
  const result = { ...defaults }
  try {
    const raw = JSON.parse(localStorage.getItem('crypt.pwgen') ?? '{}')
    if (Number.isInteger(raw.length) && raw.length >= 8 && raw.length <= 128) result.length = raw.length
    for (const key of ['lower', 'upper', 'digits', 'symbols', 'avoidAmbiguous'] as const)
      if (typeof raw[key] === 'boolean') result[key] = raw[key]
  } catch {}
  return result
}
const options = ref(read()),
  open = ref(false),
  busy = ref(false),
  { revealed, show, hide } = useRevealTimer()
let request = 0
async function generate() {
  const seq = ++request
  hide()
  busy.value = true
  try {
    const value = await api.generatePassword({ ...options.value })
    if (seq === request && open.value) show(value)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось создать пароль')
  } finally {
    if (seq === request) busy.value = false
  }
}
watch(open, (on) => {
  if (on) generate()
  else {
    ++request
    hide()
  }
})
watch(
  options,
  () => {
    try {
      localStorage.setItem('crypt.pwgen', JSON.stringify(options.value))
    } catch {}
    if (open.value) generate()
  },
  { deep: true },
)
function use() {
  if (revealed.value) {
    emit('use', revealed.value)
    hide()
    open.value = false
  }
}
const classes = [
  ['lower', 'Строчные a–z'],
  ['upper', 'Заглавные A–Z'],
  ['digits', 'Цифры 0–9'],
  ['symbols', 'Символы'],
] as const
</script>
<template>
  <span
    v-if="props.disabled"
    class="inline-flex"
    title="Пароль уже задан. Чтобы сгенерировать новый, сначала очистите поле — так старый не затрётся случайно"
    ><Button variant="ghost" size="sm" aria-label="Генератор пароля недоступен: пароль уже задан" disabled
      ><WandSparkles class="size-4" /></Button
  ></span>
  <Popover v-else v-model:open="open"
    ><PopoverTrigger as-child
      ><Button variant="ghost" size="sm" aria-label="Генератор пароля"
        ><WandSparkles class="size-4 text-secret" /></Button></PopoverTrigger
    ><PopoverContent
      class="max-h-(--reka-popover-content-available-height) w-80 gap-3 overflow-y-auto"
      align="end"
      :collision-padding="12"
      ><p class="text-sm font-medium">Генератор пароля</p>
      <output class="block min-h-12 break-all rounded-md border bg-muted/30 p-2.5 font-mono text-sm text-secret">{{
        revealed ?? 'Нажмите «Обновить»'
      }}</output>
      <div class="flex justify-between gap-2">
        <Button variant="outline" size="sm" :disabled="busy" @click="generate"
          ><RefreshCw class="size-3.5" />Обновить</Button
        ><Button size="sm" :disabled="!revealed || busy" @click="use">Использовать</Button>
      </div>
      <div class="space-y-1 border-t pt-3">
        <label class="flex justify-between text-xs"
          >Длина <span class="font-mono">{{ options.length }}</span></label
        ><input
          v-model.number="options.length"
          type="range"
          min="8"
          max="128"
          class="w-full accent-amber-400"
          aria-label="Длина пароля"
        />
      </div>
      <div class="grid grid-cols-2 gap-x-4 gap-y-2">
        <label v-for="[key, label] in classes" :key="key" class="flex items-center justify-between gap-2 text-xs"
          >{{ label }}<Switch :model-value="options[key]" @update:model-value="options[key] = $event"
        /></label>
      </div>
      <label class="flex items-center justify-between text-xs"
        >Без похожих 0O1lI<Switch v-model="options.avoidAmbiguous" /></label></PopoverContent
  ></Popover>
</template>
