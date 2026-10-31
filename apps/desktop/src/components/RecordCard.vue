<script setup lang="ts">
import { computed } from 'vue'
import KindTile from '@/components/crypt/KindTile.vue'
import RecordContextMenu from '@/components/RecordContextMenu.vue'
import { typeIcon } from '@/lib/icons'
import { toneForType } from '@/lib/tones'
import type { RecordSummary } from '@/lib/types'
import { useLibrary } from '@/stores/library'

const props = defineProps<{ record: RecordSummary }>()
const lib = useLibrary()
const type = computed(() => lib.typeById.get(props.record.type_id ?? ''))
const fmt = new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'short' })
</script>

<template>
  <RecordContextMenu :record="props.record"
    ><RouterLink
      :to="`/r/${props.record.id}`"
      class="bg-card flex min-h-[118px] flex-col gap-2.5 rounded-xl border p-3.5 transition-[transform,box-shadow] duration-300 ease-[cubic-bezier(.2,.8,.2,1)] hover:-translate-y-[3px] hover:shadow-[0_14px_28px_-16px_rgba(0,0,0,.28)] active:translate-y-[-1px] active:scale-[.99]"
    >
      <span class="flex items-center gap-2">
        <KindTile :icon="typeIcon(type?.icon)" :tone="toneForType(lib.types, props.record.type_id)" />
        <span class="text-muted-foreground flex-1 truncate text-xs">{{ type?.name ?? 'Заметка' }}</span>
        <span class="text-muted-foreground text-xs">{{ fmt.format(props.record.updated_at) }}</span>
      </span>
      <span class="text-[14px] leading-snug font-semibold" :class="!props.record.title && 'text-muted-foreground'">{{
        props.record.title || 'Без названия'
      }}</span>
      <span v-if="props.record.tags.length" class="mt-auto flex flex-wrap gap-1">
        <span
          v-for="t in props.record.tags.slice(0, 3)"
          :key="t"
          class="bg-foreground/[.07] text-foreground-2 rounded-full px-2 py-0.5 text-xs"
          >#{{ t }}</span
        >
      </span>
    </RouterLink></RecordContextMenu
  >
</template>
