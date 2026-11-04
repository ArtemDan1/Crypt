<script setup lang="ts">
import NavArrows from '@/components/NavArrows.vue'
defineProps<{ title: string; sub?: string }>()
</script>

<template>
  <header data-tauri-drag-region class="flex min-h-[52px] flex-wrap items-center gap-3 border-b pr-5 pl-5 py-2">
    <NavArrows />
    <slot name="icon" />
    <h1 class="text-[17px] font-semibold tracking-[-.01em]">{{ title }}</h1>
    <span v-if="sub" class="text-muted-foreground text-[13px]">{{ sub }}</span>
    <div class="flex-1" />
    <slot />
  </header>
</template>
