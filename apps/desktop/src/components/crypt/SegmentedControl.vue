<script setup lang="ts" generic="T extends string | number">
import { nextTick, onMounted, ref, watch } from 'vue'
import type { Component } from 'vue'

export interface SegmentOption<V> {
  value: V
  label: string
  icon?: Component
}

const props = withDefaults(
  defineProps<{
    modelValue: T
    options: SegmentOption<T>[]
    ariaLabel: string
    size?: 'sm' | 'md'
    iconOnly?: boolean
  }>(),
  { size: 'md', iconOnly: false },
)
const emit = defineEmits<{ 'update:modelValue': [value: T] }>()

const root = ref<HTMLElement | null>(null)
const thumb = ref({ left: 0, width: 0 })

function measure() {
  const el = root.value?.querySelector<HTMLElement>('[aria-pressed="true"]')
  if (el) thumb.value = { left: el.offsetLeft, width: el.offsetWidth }
}
onMounted(measure)
watch(
  () => [props.modelValue, props.options.length],
  () => nextTick(measure),
)
</script>

<template>
  <div
    ref="root"
    role="group"
    :aria-label="props.ariaLabel"
    class="bg-muted relative inline-flex rounded-[9px] p-[3px]"
    :class="props.size === 'sm' ? 'h-8' : 'h-9'"
  >
    <span
      aria-hidden="true"
      class="seg-thumb bg-card absolute top-[3px] bottom-[3px] rounded-[7px] shadow-[0_1px_2px_rgba(0,0,0,.12)]"
      :style="{ left: `${thumb.left}px`, width: `${thumb.width}px` }"
    />
    <button
      v-for="o in props.options"
      :key="String(o.value)"
      type="button"
      :aria-pressed="o.value === props.modelValue ? 'true' : 'false'"
      :aria-label="props.iconOnly ? o.label : undefined"
      :title="props.iconOnly ? o.label : undefined"
      class="relative flex items-center justify-center gap-1.5 rounded-[7px] text-[13px] transition-colors"
      :class="[
        props.iconOnly ? 'w-[34px]' : 'px-3',
        o.value === props.modelValue
          ? 'text-foreground font-semibold'
          : 'text-muted-foreground hover:text-foreground font-medium',
      ]"
      @click="emit('update:modelValue', o.value)"
    >
      <component :is="o.icon" v-if="o.icon" class="size-[15px]" />
      <span v-if="!props.iconOnly">{{ o.label }}</span>
    </button>
  </div>
</template>

<style scoped>
.seg-thumb {
  transition:
    left 0.35s cubic-bezier(0.3, 1.3, 0.5, 1),
    width 0.35s cubic-bezier(0.3, 1.3, 0.5, 1),
    background-color 0.35s ease;
}
@media (prefers-reduced-motion: reduce) {
  .seg-thumb {
    transition: none;
  }
}
</style>
