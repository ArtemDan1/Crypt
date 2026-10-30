<script setup lang="ts">
import { computed, ref } from 'vue'
import { X } from '@lucide/vue'
import { useLibrary } from '@/stores/library'

const props = defineProps<{ tags: string[] }>()
const emit = defineEmits<{ add: [tag: string]; remove: [tag: string] }>()
const lib = useLibrary()
const draft = ref('')
const editing = ref(false)
const suggestions = computed(() => {
  const q = draft.value.trim().replace(/^#/, '').toLowerCase()
  return lib.tags
    .map(([t]) => t)
    .filter((t) => !props.tags.includes(t) && (!q || t.includes(q)))
    .slice(0, 6)
})
function add(t = draft.value) {
  const tag = t.trim()
  if (tag) emit('add', tag)
  draft.value = ''
}
function commit() {
  add()
  editing.value = false
}
</script>

<template>
  <div class="flex flex-wrap items-center gap-1.5">
    <span v-for="t in tags" :key="t" class="bg-muted group flex h-6 items-center gap-1 rounded-md pr-1 pl-2 text-xs">
      #{{ t }}
      <button
        class="text-muted-foreground hover:text-foreground opacity-0 group-hover:opacity-100"
        :aria-label="`Убрать тег ${t}`"
        @click="emit('remove', t)"
      >
        <X class="size-3" />
      </button>
    </span>
    <div class="relative">
      <input
        v-if="editing"
        v-model="draft"
        autofocus
        class="h-6 w-28 rounded-md border bg-transparent px-2 text-xs outline-none"
        placeholder="тег"
        aria-label="Новый тег"
        list="tag-suggestions"
        @keydown.enter.prevent="add()"
        @keydown.esc="editing = false"
        @blur="commit"
      />
      <button
        v-else
        class="text-muted-foreground hover:text-foreground h-6 rounded-md px-1.5 text-xs"
        @click="editing = true"
      >
        + тег
      </button>
      <datalist id="tag-suggestions"><option v-for="s in suggestions" :key="s" :value="s" /></datalist>
    </div>
  </div>
</template>
