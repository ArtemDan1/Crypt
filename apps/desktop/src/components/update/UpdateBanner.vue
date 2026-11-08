<script setup lang="ts">
import { computed } from 'vue'
import UpdateCard from './UpdateCard.vue'
import { useUpdates } from '@/stores/updates'

const u = useUpdates()
// A running download stays visible even if the version was put off.
const visible = computed(() => !!u.offer || ((u.progress !== null || u.installing) && !!u.available))
</script>

<template>
  <Transition
    enter-active-class="transition duration-250 ease-out"
    enter-from-class="translate-y-3 opacity-0"
    leave-active-class="transition duration-200 ease-in"
    leave-to-class="translate-y-3 opacity-0"
  >
    <div v-if="visible" class="fixed right-4 bottom-4 z-40"><UpdateCard /></div>
  </Transition>
</template>
