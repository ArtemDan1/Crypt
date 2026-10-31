<script setup lang="ts">
import { computed } from 'vue'
import { Eye, EyeOff } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { api } from '@/lib/api'
import { useRevealTimer } from '@/lib/reveal'

// A bank card drawn as a card. The number stays masked until the person asks to see it.
const props = defineProps<{ recordId: string; filled: boolean; holder?: string; expiry?: string }>()
const { revealed, show, hide } = useRevealTimer()
const number = computed(() =>
  revealed.value ? revealed.value.replace(/\s+/g, '').replace(/(.{4})(?=.)/g, '$1 ') : '•••• •••• •••• ••••',
)
async function toggle() {
  if (revealed.value !== null) return hide()
  try {
    show(await api.revealSecret(props.recordId, 'number'))
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось показать номер')
  }
}
</script>

<template>
  <div class="mb-4 flex items-end gap-3">
    <div
      class="relative flex h-[200px] w-[320px] flex-col justify-between overflow-hidden rounded-[18px] bg-[#1F1E1B] p-5 text-[#F2F0EB] shadow-[0_20px_40px_-20px_rgba(0,0,0,.5)] transition-transform duration-500 ease-[cubic-bezier(.2,.8,.2,1)] hover:-translate-y-[3px] hover:-rotate-[1.5deg]"
    >
      <svg width="220" height="220" viewBox="0 0 220 220" aria-hidden="true" class="absolute -top-12 -right-14">
        <circle cx="110" cy="110" r="100" fill="none" stroke="#12855F" stroke-width="2" />
        <circle cx="110" cy="110" r="74" fill="none" stroke="#12855F" stroke-width="2" opacity=".6" />
        <circle cx="110" cy="110" r="48" fill="none" stroke="#12855F" stroke-width="2" opacity=".35" />
      </svg>
      <span class="text-[13px] opacity-75">Карта</span>
      <span :key="number" class="font-mono text-[19px] tracking-[.1em]" :class="revealed && 'anim-reveal selectable'">{{
        number
      }}</span>
      <span class="flex justify-between text-xs opacity-85">
        <span class="uppercase">{{ props.holder || ' ' }}</span>
        <span class="font-mono">{{ props.expiry || '' }}</span>
      </span>
    </div>
    <button
      v-if="props.filled"
      class="text-muted-foreground hover:bg-foreground/[.08] flex size-9 items-center justify-center rounded-lg transition-colors"
      :aria-label="revealed === null ? 'Показать номер' : 'Скрыть номер'"
      @click="toggle"
    >
      <component :is="revealed === null ? Eye : EyeOff" class="size-4" />
    </button>
  </div>
</template>
