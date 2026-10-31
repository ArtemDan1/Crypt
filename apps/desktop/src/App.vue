<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue'
import { Toaster } from '@/components/ui/sonner'
import { onThemeChange, resolvedTheme } from '@/lib/theme'

const theme = ref(resolvedTheme())
const off = onThemeChange((t) => (theme.value = t))
onBeforeUnmount(off)
</script>

<template>
  <RouterView />
  <Toaster position="bottom-right" :theme="theme" />
</template>
