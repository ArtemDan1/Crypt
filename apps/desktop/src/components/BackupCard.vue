<script setup lang="ts">
import { computed, onMounted, ref, onBeforeUnmount } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { api } from '@/lib/api'
import { backupReminder } from '@/lib/backupReminder'
import type { BackupExport } from '@/lib/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useSession } from '@/stores/session'
const last = ref<number | null>(null),
  confirming = ref(false),
  pin = ref(''),
  hasPin = ref(false)
const busy = ref(false),
  error = ref(''),
  result = ref<BackupExport | null>(null)
const reminder = computed(() => backupReminder(last.value, Date.now()))
onMounted(async () => {
  last.value = (await api.getSettings()).lastBackupAt ?? null
  hasPin.value = (await api.unlockMethods()).pin
})
const unhook = useSession().beforeLock(() => {
  pin.value = ''
  confirming.value = false
  result.value = null
})
onBeforeUnmount(unhook)
function cancel() {
  confirming.value = false
  pin.value = ''
}
async function exportBackup() {
  if (busy.value) return
  busy.value = true
  error.value = ''
  result.value = null
  try {
    const name = `Crypt-${new Date().toISOString().slice(0, 10)}.cryptbackup`
    const path = isTauri()
      ? await save({ defaultPath: name, filters: [{ name: 'Бэкап Crypt', extensions: ['cryptbackup'] }] })
      : `/Downloads/${name}`
    if (!path) return
    result.value = await api.exportBackup(path, pin.value || undefined)
    confirming.value = false
    last.value = result.value.header.created_at
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    pin.value = ''
    busy.value = false
  }
}
async function reveal() {
  if (result.value)
    try {
      await revealItemInDir(result.value.path)
    } catch (e) {
      error.value = String(e)
    }
}
</script>
<template>
  <section class="space-y-4 rounded-[14px] border bg-card p-5">
    <div>
      <h2 class="font-semibold">Бэкап всей базы</h2>
      <p class="mt-1 text-sm text-muted-foreground">
        Записи, секреты, история и снимки в одном зашифрованном файле. Для восстановления нужна сид-фраза.
      </p>
    </div>
    <p class="text-sm" :class="reminder === 'none' ? 'text-muted-foreground' : 'text-secret-fg'">
      {{
        last
          ? `Последний бэкап: ${new Date(last).toLocaleDateString('ru-RU')}`
          : 'Бэкапа пока нет — сохраните копию хранилища'
      }}<span v-if="reminder === 'stale'"> · Пора обновить копию</span>
    </p>
    <Button v-if="!confirming" @click="confirming = true">Экспорт бэкапа…</Button>
    <form v-else class="space-y-3" @submit.prevent="exportBackup">
      <p class="text-sm">Подтвердите экспорт с Touch ID / паролем Mac<span v-if="hasPin"> или введите PIN</span>.</p>
      <Input
        v-if="hasPin"
        v-model="pin"
        type="password"
        inputmode="numeric"
        autocomplete="off"
        aria-label="PIN для экспорта бэкапа"
      />
      <div class="flex gap-2">
        <Button :disabled="busy" type="submit">{{ busy ? 'Упаковываем…' : 'Продолжить' }}</Button
        ><Button variant="outline" :disabled="busy" @click="cancel">Отмена</Button>
      </div>
    </form>
    <progress v-if="busy" class="h-1 w-full accent-primary" max="1" aria-label="Упаковка бэкапа" />
    <div v-if="result" role="status" class="rounded-lg bg-primary-soft p-3 text-sm text-primary-soft-fg">
      <p>Бэкап сохранён · {{ (result.bytes / 1024).toFixed(1) }} КБ</p>
      <p class="mt-1 truncate">{{ result.path.split('/').pop() }}</p>
      <button class="mt-2 underline" @click="reveal">Показать в Finder</button>
    </div>
    <p v-if="error" role="alert" class="text-sm text-destructive">{{ error }}</p>
  </section>
</template>
