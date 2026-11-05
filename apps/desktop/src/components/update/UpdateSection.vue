<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { LoaderCircle, RefreshCw } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import UpdateCard from './UpdateCard.vue'
import { useUpdates } from '@/stores/updates'

const u = useUpdates()
const version = ref('')
const checked = ref(false)
onMounted(async () => {
  version.value = await getVersion().catch(() => '')
})
async function check() {
  try {
    await u.check()
    checked.value = true
  } catch (e) {
    toast.error(`Не удалось проверить обновления: ${e instanceof Error ? e.message : e}`)
  }
}
</script>

<template>
  <section class="bg-card flex flex-col gap-3 rounded-[14px] border px-[18px] py-4">
    <div class="flex items-center gap-3">
      <span class="flex flex-1 flex-col gap-0.5">
        <span class="text-[14px] font-medium">Версия {{ version || '—' }}</span>
        <span class="text-muted-foreground text-xs">Обновления приходят из релизов на GitHub</span>
      </span>
      <Button variant="outline" size="sm" :disabled="u.checking" @click="check">
        <component :is="u.checking ? LoaderCircle : RefreshCw" class="size-4" :class="u.checking && 'animate-spin'" />
        {{ u.checking ? 'Проверка…' : 'Проверить обновления' }}
      </Button>
    </div>
    <UpdateCard v-if="u.available" inline />
    <p v-else-if="checked && !u.checking" class="text-muted-foreground text-[13px]">Установлена последняя версия</p>
  </section>
</template>
