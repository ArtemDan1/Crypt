<script setup lang="ts">
import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { Download } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { api, CryptApiError } from '@/lib/api'
const busy = ref(false),
  includeSecrets = ref(false)
function fail(e: unknown) {
  toast.error(e instanceof CryptApiError ? e.message : String(e))
}
async function exportMd() {
  const path = await open({ directory: true, multiple: false, title: 'Куда сохранить экспорт' })
  if (typeof path !== 'string') return
  busy.value = true
  try {
    const r = await api.exportMarkdown(path, includeSecrets.value)
    toast(`Экспортировано заметок: ${r.notes}, секретов: ${r.secrets}`, {
      description: r.path,
      action: { label: 'Показать', onClick: () => revealItemInDir(r.path).catch(() => {}) },
    })
  } catch (e) {
    fail(e)
  } finally {
    busy.value = false
  }
}
</script>
<template>
  <section class="rounded-xl border bg-card">
    <div class="space-y-3 px-4 py-3.5">
      <div class="flex items-center gap-3">
        <Download class="text-muted-foreground size-5" />
        <div class="flex-1">
          <div class="font-medium">Экспорт в Markdown</div>
          <div class="text-muted-foreground text-xs">
            Папки и заметки в .md, поля — во фронтматтере. Создаётся новая папка, ничего не перезаписывается.
          </div>
        </div>
        <Button variant="outline" size="sm" :disabled="busy" @click="exportMd"
          ><Download class="size-3.5" />Экспортировать…</Button
        >
      </div>
      <label class="flex items-center gap-3 text-sm">
        <span class="flex-1"
          >Включить значения секретов
          <span class="text-muted-foreground block text-xs" :class="includeSecrets && 'text-destructive'">{{
            includeSecrets
              ? 'Пароли и ключи будут записаны открытым текстом — храните экспорт осторожно'
              : 'По умолчанию у секретов выгружаются только название и вид'
          }}</span></span
        ><Switch v-model="includeSecrets" aria-label="Включить значения секретов" />
      </label>
    </div>
  </section>
</template>
