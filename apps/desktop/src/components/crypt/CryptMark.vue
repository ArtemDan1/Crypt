<script setup lang="ts">
// The «Seed» mark: an emerald seed on a graphite tile. Below 40px the veins are dropped
// so the silhouette stays readable.
const props = withDefaults(defineProps<{ size?: number; animated?: boolean; decorative?: boolean }>(), {
  size: 32,
  animated: false,
  decorative: false,
})
</script>

<template>
  <svg
    :width="props.size"
    :height="props.size"
    viewBox="0 0 1024 1024"
    :role="props.decorative ? undefined : 'img'"
    :aria-label="props.decorative ? undefined : 'Crypt'"
    :aria-hidden="props.decorative ? 'true' : undefined"
    class="shrink-0"
  >
    <rect x="100" y="100" width="824" height="824" rx="185" class="fill-[#1F1E1B] dark:fill-[#33322E]" />
    <g :class="props.animated && 'crypt-mark-sprout'">
      <template v-if="props.size >= 40">
        <path d="M512 196C716 336 716 660 512 828 308 660 308 336 512 196Z" fill="#12855F" />
        <path d="M512 300V724" stroke="#0A3B2C" stroke-width="30" stroke-linecap="round" />
        <path
          d="M512 470L598 400M512 590L604 516M512 520L430 456"
          stroke="#0A3B2C"
          stroke-width="22"
          stroke-linecap="round"
        />
      </template>
      <template v-else>
        <path d="M512 180C736 330 736 670 512 844 288 670 288 330 512 180Z" fill="#12855F" />
        <path d="M512 300V724" stroke="#0A3B2C" stroke-width="44" stroke-linecap="round" />
      </template>
    </g>
  </svg>
</template>

<style scoped>
.crypt-mark-sprout {
  transform-box: view-box;
  transform-origin: 512px 828px;
  animation: crypt-sprout 0.65s cubic-bezier(0.3, 1.4, 0.5, 1) both;
}
@keyframes crypt-sprout {
  from {
    transform: scale(0.55);
  }
  to {
    transform: scale(1);
  }
}
@media (prefers-reduced-motion: reduce) {
  .crypt-mark-sprout {
    animation: none;
  }
}
</style>
