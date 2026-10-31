<script setup lang="ts">
import RecordContextMenu from '@/components/RecordContextMenu.vue'
import KindTile from '@/components/crypt/KindTile.vue'
import { typeIcon } from '@/lib/icons'
import { toneForType } from '@/lib/tones'
import type { RecordSummary } from '@/lib/types'
import { useLibrary } from '@/stores/library'

defineProps<{ records: RecordSummary[]; empty?: string }>()
const lib = useLibrary()
const fmt = new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })
</script>

<template>
  <div>
    <div v-if="records.length" class="bg-card overflow-hidden rounded-xl border">
      <RecordContextMenu v-for="r in records" :key="r.id" :record="r"
        ><RouterLink
          :to="`/r/${r.id}`"
          class="hover:bg-foreground/[.04] flex min-h-12 items-center gap-3 border-t px-4 transition-colors first:border-t-0"
        >
          <KindTile
            :icon="typeIcon(lib.typeById.get(r.type_id ?? '')?.icon)"
            :tone="toneForType(lib.types, r.type_id)"
          />
          <span class="min-w-0 flex-1 truncate text-[14px] font-medium" :class="!r.title && 'text-muted-foreground'">{{
            r.title || 'Без названия'
          }}</span>
          <span
            v-for="t in r.tags.slice(0, 3)"
            :key="t"
            class="bg-foreground/[.07] text-foreground-2 rounded-full px-2 py-0.5 text-xs"
            >#{{ t }}</span
          >
          <span class="text-muted-foreground w-28 text-right text-xs">{{ fmt.format(r.updated_at) }}</span>
        </RouterLink></RecordContextMenu
      >
    </div>
    <p v-else class="text-muted-foreground px-1 py-6 text-[13px]">{{ empty ?? 'Здесь пока пусто' }}</p>
  </div>
</template>
