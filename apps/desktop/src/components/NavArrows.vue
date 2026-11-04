<script setup lang="ts">
import { ChevronLeft, ChevronRight } from '@lucide/vue'
import { useTabs } from '@/stores/tabs'

const tabs = useTabs()
const btn =
  'text-muted-foreground hover:bg-foreground/[.08] hover:text-foreground flex size-7 items-center justify-center rounded-[7px] transition-colors disabled:pointer-events-none disabled:opacity-35'
</script>

<template>
  <div class="-ml-1.5 flex shrink-0 items-center">
    <button :class="btn" :disabled="!tabs.canBack" aria-label="Назад" title="Назад (⌘[)" @click="tabs.back()">
      <ChevronLeft class="size-4" />
    </button>
    <button :class="btn" :disabled="!tabs.canForward" aria-label="Вперёд" title="Вперёд (⌘])" @click="tabs.forward()">
      <ChevronRight class="size-4" />
    </button>
  </div>
</template>
