<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open } from '@tauri-apps/plugin-dialog'
import CryptMark from '@/components/crypt/CryptMark.vue'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { api, CryptApiError } from '@/lib/api'
import type { BackupInspection } from '@/lib/types'
import { useSession } from '@/stores/session'
const router = useRouter(),
  session = useSession()
const path = ref(''),
  header = ref<BackupInspection | null>(null),
  text = ref(''),
  busy = ref(false),
  error = ref(''),
  shake = ref(0)
const words = computed(() => text.value.trim().toLowerCase().split(/\s+/).filter(Boolean))
let unlisten: (() => void) | undefined
let active = true,
  request = 0
async function inspect(file: string) {
  if (busy.value) return
  const seq = ++request
  error.value = ''
  header.value = null
  path.value = ''
  try {
    const h = await api.inspectBackup(file)
    if (active && seq === request) {
      header.value = h
      path.value = file
    }
  } catch (e) {
    if (active && seq === request) error.value = e instanceof Error ? e.message : String(e)
  }
}
async function pick() {
  try {
    const file = isTauri()
      ? await open({ multiple: false, filters: [{ name: 'Бэкап Crypt', extensions: ['cryptbackup'] }] })
      : '/Downloads/Crypt-demo.cryptbackup'
    if (typeof file === 'string') await inspect(file)
  } catch (e) {
    error.value = String(e)
  }
}
async function restore() {
  if (!header.value || words.value.length !== 24 || busy.value) return
  busy.value = true
  error.value = ''
  try {
    await session.restoreBackup(path.value, words.value)
    text.value = ''
    await router.replace('/onboarding?step=4')
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
    if (e instanceof CryptApiError && e.code === 'BackupSeedMismatch') ++shake.value
  } finally {
    busy.value = false
  }
}
onMounted(async () => {
  if (isTauri()) {
    const off = await getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === 'drop' && e.payload.paths[0]) void inspect(e.payload.paths[0])
    })
    if (active) unlisten = off
    else off()
  }
})
onBeforeUnmount(() => {
  active = false
  ++request
  text.value = ''
  unlisten?.()
})
</script>
<template>
  <div class="flex h-full overflow-auto bg-background">
    <aside class="hidden w-[300px] shrink-0 flex-col bg-[#1c1b19] p-9 text-[#edebe6] sm:flex">
      <div data-tauri-drag-region class="h-8" />
      <CryptMark :size="64" />
      <h2 class="mt-6 text-2xl font-semibold">Снова под печатью.</h2>
      <ol class="mt-12 space-y-6 border-l border-white/20 pl-5 text-sm">
        <li :class="header ? 'text-white/70' : 'font-semibold'">1 · Файл бэкапа</li>
        <li :class="header && !busy ? 'font-semibold' : 'text-white/70'">2 · Сид-фраза</li>
        <li :class="busy ? 'font-semibold' : 'text-white/70'">3 · Восстановление</li>
      </ol>
    </aside>
    <main class="mx-auto w-full max-w-[680px] space-y-6 px-8 pt-16 pb-8 anim-in">
      <h1 class="text-[28px] font-semibold tracking-tight">Восстановить из бэкапа</h1>
      <p class="text-sm text-muted-foreground">Выберите .cryptbackup и введите сид-фразу этого хранилища.</p>
      <div class="rounded-xl border border-dashed bg-card p-6">
        <Button variant="outline" :disabled="busy" @click="pick">{{
          path ? 'Выбрать другой файл' : 'Выбрать файл…'
        }}</Button>
        <p class="mt-3 text-sm text-muted-foreground">Или перетащите файл в окно</p>
        <div v-if="header" class="mt-4 text-sm">
          <p class="truncate font-medium">{{ path.split('/').pop() }}</p>
          <p class="text-muted-foreground">
            {{ new Date(header.created_at).toLocaleDateString('ru-RU') }} · {{ (header.bytes / 1024).toFixed(1) }} КБ ·
            объектов: {{ header.objects }}
          </p>
        </div>
      </div>
      <form v-if="header" :key="shake" class="space-y-4" :class="shake ? 'anim-shake' : ''" @submit.prevent="restore">
        <label for="restore-seed" class="text-sm font-medium">Сид-фраза</label
        ><Textarea
          id="restore-seed"
          v-model="text"
          :disabled="busy"
          rows="6"
          class="font-mono"
          autocomplete="off"
          spellcheck="false"
          autocapitalize="off"
        />
        <p class="text-xs text-muted-foreground">{{ words.length }} / 24 слова</p>
        <Button :disabled="words.length !== 24 || busy" type="submit">{{
          busy ? 'Проверяем и восстанавливаем…' : 'Восстановить'
        }}</Button
        ><progress v-if="busy" aria-label="Восстановление" max="1" class="block h-1 w-full accent-primary" />
      </form>
      <p v-if="error" role="alert" class="text-sm text-destructive">{{ error }}</p>
      <Button variant="ghost" :disabled="busy" @click="router.push('/onboarding')">Назад</Button>
    </main>
  </div>
</template>
