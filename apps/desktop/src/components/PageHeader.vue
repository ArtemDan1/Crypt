<script setup lang="ts">
defineProps<{ title: string; sub?: string }>()
</script>

<template>
  <header data-tauri-drag-region class="flex min-h-[52px] flex-wrap items-center gap-3 border-b px-6 py-2">
    <slot name="icon" />
    <h1 class="text-[17px] font-semibold tracking-tight">{{ title }}</h1>
    <span v-if="sub" class="text-muted-foreground text-[13px]">{{ sub }}</span>
    <div class="flex-1" />
    <slot />
  </header>
</template>
