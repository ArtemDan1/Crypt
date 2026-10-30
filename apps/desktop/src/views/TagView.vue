<script setup lang="ts">
import { computed } from 'vue'
import { Hash } from '@lucide/vue'
import PageHeader from '@/components/PageHeader.vue'
import RecordList from '@/components/RecordList.vue'
import { useLibrary } from '@/stores/library'

const props = defineProps<{ tag: string }>()
const lib = useLibrary()
const records = computed(() => lib.records.filter((r) => r.tags.includes(props.tag)))
</script>

<template>
  <div class="flex h-full flex-col">
    <PageHeader :title="tag" :sub="`${records.length}`">
      <template #icon><Hash class="text-muted-foreground size-5" /></template>
    </PageHeader>
    <div class="flex-1 overflow-auto p-3"><RecordList :records="records" /></div>
  </div>
</template>
