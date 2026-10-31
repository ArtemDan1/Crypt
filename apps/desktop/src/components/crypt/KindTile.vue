<script setup lang="ts">
import type { Component } from 'vue'

// A small rounded tile holding an icon: record types, secret kinds, files.
export type TileTone = 'neutral' | 'primary' | 'secret' | 'sand' | 'slate' | 'rose' | 'plum'

const props = withDefaults(defineProps<{ icon?: Component; tone?: TileTone; size?: 'sm' | 'md' | 'lg' }>(), {
  tone: 'neutral',
  size: 'md',
})

const tones: Record<TileTone, string> = {
  neutral: 'bg-accent text-foreground-2',
  primary: 'bg-primary-soft text-primary-soft-fg',
  secret: 'bg-secret-bg text-secret',
  sand: 'bg-[#EEE7DA] text-[#6B4A1F] dark:bg-[#2E2719] dark:text-[#D9B985]',
  slate: 'bg-[#E4E8F1] text-[#33456B] dark:bg-[#1E2433] dark:text-[#A9B8DA]',
  rose: 'bg-[#F6E4E1] text-[#8A2F25] dark:bg-[#33201F] dark:text-[#E7A9A1]',
  plum: 'bg-[#F1E6F5] text-[#6A2F7F] dark:bg-[#2A1E31] dark:text-[#D3A8E2]',
}
const sizes = {
  sm: 'size-[18px] rounded-[5px] [&_svg]:size-3',
  md: 'size-[22px] rounded-[6px] [&_svg]:size-[13px]',
  lg: 'size-[34px] rounded-[9px] [&_svg]:size-[17px]',
}
</script>

<template>
  <span class="inline-flex shrink-0 items-center justify-center" :class="[tones[props.tone], sizes[props.size]]">
    <component :is="props.icon" v-if="props.icon" :stroke-width="2.2" />
    <slot />
  </span>
</template>
