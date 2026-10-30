<script setup lang="ts">
import RecordContextMenu from '@/components/RecordContextMenu.vue'
import { typeIcon } from '@/lib/icons'
import type { RecordSummary } from '@/lib/types'
import { useLibrary } from '@/stores/library'

defineProps<{ records: RecordSummary[]; empty?: string }>()
const lib = useLibrary()
const fmt = new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })
</script>

<template>
  <div>
    <RecordContextMenu v-for="r in records" :key="r.id" :record="r"><RouterLink
      :to="`/r/${r.id}`"
      class="hover:bg-accent/60 flex h-11 items-center gap-3 rounded-lg px-3"
    >
      <component
        :is="typeIcon(lib.typeById.get(r.type_id ?? '')?.icon)"
        class="text-muted-foreground size-4 shrink-0"
      />
      <span class="min-w-0 flex-1 truncate font-medium" :class="!r.title && 'text-muted-foreground'">{{
        r.title || 'Без названия'
      }}</span>
      <span v-for="t in r.tags.slice(0, 3)" :key="t" class="bg-muted rounded px-1.5 py-0.5 text-xs">#{{ t }}</span>
      <span class="text-muted-foreground w-28 text-right text-xs">{{ fmt.format(r.updated_at) }}</span>
    </RouterLink></RecordContextMenu>
    <p v-if="!records.length" class="text-muted-foreground px-3 py-6 text-[13px]">{{ empty ?? 'Здесь пока пусто' }}</p>
  </div>
</template>
