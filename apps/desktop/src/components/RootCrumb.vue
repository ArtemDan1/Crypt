<script setup lang="ts">
import { FolderRoot } from '@lucide/vue'
import { ROOT_TITLE } from '@/lib/pageInfo'

defineProps<{ current?: boolean }>()
</script>

<template>
  <RouterLink
    to="/"
    title="Корень хранилища"
    :aria-current="current ? 'page' : undefined"
    class="hover:text-foreground hover:bg-foreground/[.06] flex shrink-0 items-center gap-1.5 rounded-md px-1.5 py-1 transition-colors"
    :class="current && 'text-foreground font-semibold'"
  >
    <FolderRoot class="size-3.5" :class="current ? 'text-primary' : ''" />{{ ROOT_TITLE }}
  </RouterLink>
</template>
