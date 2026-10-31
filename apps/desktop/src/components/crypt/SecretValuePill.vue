<script setup lang="ts">
// Amber plate for a secret value: mask while hidden, the value (unblurring in) once revealed.
const props = withDefaults(defineProps<{ value: string | null; mono?: boolean }>(), { mono: true })
</script>

<template>
  <span
    class="bg-secret-bg text-secret-fg flex min-h-8 min-w-0 items-center overflow-hidden rounded-lg px-2.5 text-sm"
    :class="props.mono && 'font-mono'"
  >
    <span v-if="props.value !== null" :key="props.value" class="anim-reveal selectable truncate">{{
      props.value
    }}</span>
    <span v-else class="tracking-[.14em]" aria-label="Скрыто">••••••••••••</span>
  </span>
</template>
