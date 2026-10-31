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
    toast.success(`Код скопирован · очистится через ${settings.clipboardClearSeconds} с`)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось скопировать код')
  }
}
</script>
<template>
  <button
    v-if="code"
    class="bg-background hover:bg-foreground/[.03] flex w-full items-center gap-4 rounded-xl border p-4 text-left transition-colors active:scale-[.99]"
    aria-label="Копировать код TOTP"
    @click="copy"
  >
    <svg class="size-11 -rotate-90" viewBox="0 0 44 44" aria-hidden="true">
      <circle cx="22" cy="22" r="18" fill="none" class="stroke-muted" stroke-width="4" />
      <circle
        cx="22"
        cy="22"
        r="18"
        fill="none"
        stroke-width="4"
        stroke-linecap="round"
        stroke-dasharray="113.1"
        class="totp-ring"
        :class="code.remaining <= 5 ? 'stroke-secret' : 'stroke-primary'"
        :stroke-dashoffset="113.1 * (1 - code.remaining / code.period)"
      /></svg
    ><span class="flex-1"
      ><span class="text-muted-foreground block text-xs">Одноразовый код · {{ code.remaining }} с</span
      ><span :key="code.code" class="anim-in block font-mono text-[26px] font-semibold tracking-[.06em]">{{
        code.code.replace(/(.{3})(?=.)/g, '$1 ')
      }}</span></span
    ><span class="text-muted-foreground flex items-center gap-1.5 text-xs"><Copy class="size-4" /> Скопировать</span>
  </button>
</template>

<style scoped>
.totp-ring {
  transition:
    stroke-dashoffset 1s linear,
    stroke 0.3s ease;
}
</style>
