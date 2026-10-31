<script setup lang="ts">
import { computed } from 'vue'

// Thin ring showing how much time is left before auto-lock.
const props = withDefaults(defineProps<{ left: number | null; total: number | null; size?: number }>(), {
  size: 28,
})
const r = computed(() => props.size / 2 - 2)
const circ = computed(() => 2 * Math.PI * r.value)
const show = computed(() => props.left !== null && props.total !== null && props.total > 0)
const offset = computed(() => {
  if (!show.value) return 0
  const frac = Math.min(1, Math.max(0, props.left! / props.total!))
  return circ.value * (1 - frac)
})
</script>

<template>
  <svg
    v-if="show"
    :width="props.size"
    :height="props.size"
    :viewBox="`0 0 ${props.size} ${props.size}`"
    aria-hidden="true"
    class="pointer-events-none absolute -rotate-90"
  >
    <circle :cx="props.size / 2" :cy="props.size / 2" :r="r" fill="none" class="stroke-border" stroke-width="2" />
    <circle
      :cx="props.size / 2"
      :cy="props.size / 2"
      :r="r"
      fill="none"
      class="stroke-primary ring-arc"
      stroke-width="2"
      stroke-linecap="round"
      :stroke-dasharray="circ"
      :stroke-dashoffset="offset"
    />
  </svg>
</template>

<style scoped>
.ring-arc {
  transition: stroke-dashoffset 1s linear;
}
</style>
