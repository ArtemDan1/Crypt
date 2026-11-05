<script setup lang="ts">
import { ArrowUpRight, Download, RotateCw, Sparkles } from '@lucide/vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Button } from '@/components/ui/button'
import { useUpdates } from '@/stores/updates'

/** `inline`: inside Settings, full width, without «Позже» and the shadow. */
defineProps<{ inline?: boolean }>()
const u = useUpdates()
const open = (url: string) => openUrl(url).catch(() => {})
</script>

<template>
  <div
    v-if="u.available"
    class="flex flex-col gap-3.5 rounded-2xl border p-4"
    :class="
      inline
        ? 'bg-primary-soft border-primary/25'
        : 'bg-card w-[360px] shadow-[0_12px_32px_rgb(0_0_0/.14)] dark:shadow-[0_12px_32px_rgb(0_0_0/.45)]'
    "
    role="status"
  >
    <div class="flex items-start gap-3">
      <span
        class="bg-primary-soft border-primary/25 flex size-9 shrink-0 items-center justify-center rounded-[10px] border"
      >
        <Sparkles class="text-primary size-[18px]" />
      </span>
      <div class="flex min-w-0 flex-col gap-0.5">
        <span class="text-[15px] font-semibold">Доступна версия {{ u.available.version }}</span>
        <span class="text-muted-foreground text-[13px]">{{
          u.installing
            ? 'Запускаем установщик — Crypt закроется и обновится'
            : u.progress !== null
              ? 'Загружаем обновление…'
              : 'Новая версия Crypt готова к установке'
        }}</span>
      </div>
    </div>
    <div v-if="u.progress !== null || u.installing" class="flex items-center gap-2.5">
      <span class="bg-foreground/[.08] h-1.5 flex-1 overflow-hidden rounded-full">
        <span
          class="bg-primary block h-full rounded-full transition-[width] duration-200"
          :class="u.installing && 'animate-pulse'"
          :style="{ width: `${u.installing ? 100 : Math.round((u.progress ?? 0) * 100)}%` }"
        />
      </span>
      <span v-if="!u.installing" class="text-muted-foreground w-9 text-right text-xs tabular-nums"
        >{{ Math.round((u.progress ?? 0) * 100) }}%</span
      >
    </div>
    <template v-else>
      <p v-if="u.error" class="bg-destructive/10 text-destructive rounded-[10px] px-2.5 py-2 text-xs">
        Не удалось скачать: {{ u.error }}
      </p>
      <div class="flex items-center gap-1.5">
        <Button size="sm" @click="u.install()">
          <component :is="u.error ? RotateCw : Download" class="size-4" />{{ u.error ? 'Повторить' : 'Обновить' }}
        </Button>
        <Button v-if="!inline" size="sm" variant="ghost" @click="u.dismiss()">Позже</Button>
      </div>
      <div class="-mt-1 flex flex-wrap gap-4 text-[13px]">
        <button
          v-if="u.available.releaseUrl"
          class="text-primary flex items-center gap-0.5 hover:underline"
          @click="open(u.available.releaseUrl)"
        >
          Что нового <ArrowUpRight class="size-3.5" />
        </button>
        <button v-if="!inline" class="text-muted-foreground hover:text-foreground" @click="u.skip()">
          Пропустить эту версию
        </button>
      </div>
    </template>
  </div>
</template>
