<script setup lang="ts">
import { computed } from 'vue'
import { KeyRound, Copy } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { useLibrary } from '@/stores/library'
import { api } from '@/lib/api'
const props = defineProps<{ id: string }>(),
  lib = useLibrary()
const record = computed(() => lib.records.find((r) => r.id === props.id && r.secret_kind))
const copies = computed(() =>
  record.value?.secret_kind === 'login'
    ? [
        ['username', 'логин'],
        ['password', 'пароль'],
      ]
    : record.value?.secret_kind === 'sshkey'
      ? [['public_key', 'публичный ключ']]
      : record.value?.secret_kind === 'token'
        ? [['token', 'токен']]
        : [],
)
async function copy(name: string) {
  try {
    await api.copySecret(props.id, name)
    const s = await api.getSettings()
    toast(`Скопировано, буфер очистится через ${s.clipboardClearSeconds} с`)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось скопировать')
  }
}
</script>
<template>
  <span class="inline-flex min-w-0 items-center gap-1.5 text-[13px]"
    ><KeyRound class="size-3.5 shrink-0 text-secret" /><RouterLink
      v-if="record"
      :to="`/r/${id}`"
      class="truncate hover:underline"
      >{{ record.title || 'Без названия' }}</RouterLink
    ><span v-else class="text-muted-foreground line-through">удалено</span
    ><button
      v-for="pair in copies"
      :key="pair[0]"
      type="button"
      class="rounded p-1 text-muted-foreground hover:bg-accent hover:text-secret"
      :aria-label="`Копировать ${pair[1]}`"
      :title="`Копировать ${pair[1]}`"
      @click.stop="copy(pair[0]!)"
    >
      <Copy class="size-3" /></button
  ></span>
</template>
