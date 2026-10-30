<script setup lang="ts">
import { ref, onBeforeUnmount } from 'vue'
import { toast } from 'vue-sonner'
import { Copy } from '@lucide/vue'
import { api } from '@/lib/api'
import type { TotpCode as Code } from '@/lib/types'
import { useSession } from '@/stores/session'
const props = defineProps<{ recordId: string }>(),
  code = ref<Code | null>(null)
let active = true,
  request = 0,
  busy = false
async function load() {
  if (busy || !active) return
  busy = true
  const seq = ++request
  try {
    const value = await api.totpNow(props.recordId)
    if (active && seq === request) code.value = value
  } catch {
    code.value = null
  } finally {
    busy = false
  }
}
load()
const timer = setInterval(() => {
  if (!active) return
  if (code.value && code.value.remaining > 1) code.value.remaining--
  else {
    code.value = null
    load()
  }
}, 1000)
function clear() {
  active = false
  ++request
  code.value = null
  clearInterval(timer)
}
const unhook = useSession().beforeLock(clear)
onBeforeUnmount(() => {
  clear()
  unhook()
})
async function copy() {
  try {
    await api.copyTotp(props.recordId)
    const settings = await api.getSettings()
    toast(`Код скопирован, буфер очистится через ${settings.clipboardClearSeconds} с`)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось скопировать код')
  }
}
</script>
<template>
  <button
    v-if="code"
    class="flex w-full items-center gap-3 rounded-lg border bg-secret/5 p-3 text-left"
    aria-label="Копировать код TOTP"
    @click="copy"
  >
    <svg class="size-9 -rotate-90 text-secret" viewBox="0 0 36 36" aria-hidden="true">
      <circle cx="18" cy="18" r="14" fill="none" stroke="currentColor" stroke-width="2" opacity=".2" />
      <circle
        cx="18"
        cy="18"
        r="14"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-dasharray="88"
        :stroke-dashoffset="88 * (1 - code.remaining / code.period)"
      /></svg
    ><span class="flex-1"
      ><span class="block text-xs text-muted-foreground">Одноразовый код</span
      ><span class="font-mono text-xl tracking-wider text-secret">{{
        code.code.replace(/(.{3})(?=.)/g, '$1 ')
      }}</span></span
    ><span class="text-xs text-muted-foreground">{{ code.remaining }} с</span
    ><Copy class="size-4 text-muted-foreground" />
  </button>
</template>
