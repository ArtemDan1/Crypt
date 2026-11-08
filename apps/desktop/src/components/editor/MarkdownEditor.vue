<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { openUrl } from '@tauri-apps/plugin-opener'
import { toast } from 'vue-sonner'
import { api } from '@/lib/api'
import { useLibrary } from '@/stores/library'
import MarkdownSource from './MarkdownSource.vue'
import { createNoteEditor, type LinkQuery } from './noteEditor'
import '@milkdown/crepe/theme/common/style.css'
import '@milkdown/crepe/theme/frame.css'
import './milkdown.css'

const model = defineModel<string>({ required: true })
const props = defineProps<{ recordId: string }>()
const lib = useLibrary()
const router = useRouter()
const host = ref<HTMLDivElement | null>(null)
const sourceEditor = ref<InstanceType<typeof MarkdownSource> | null>(null)
// Raw Markdown is only a fallback for when the visual editor fails to load.
const source = ref(false)
const loading = ref(true)
const failed = ref(false)
const query = ref<LinkQuery | null>(null)
const matches = ref<{ id: string; title: string }[]>([])
const selected = ref(0)
const choosing = ref(false)
let editor: ReturnType<typeof createNoteEditor> | undefined
let alive = true
let request = 0
let rendered = model.value
const canCreate = computed(() => !!query.value?.query.trim())
const menuStyle = computed(() => ({
  left: `${Math.max(8, Math.min(query.value?.left ?? 0, window.innerWidth - 300))}px`,
  top: `${Math.max(8, Math.min(query.value?.top ?? 0, window.innerHeight - 300))}px`,
}))

async function showLinks(next: LinkQuery | null) {
  if (next?.query === query.value?.query && next?.from === query.value?.from && next?.to === query.value?.to) return
  query.value = next
  matches.value = []
  selected.value = 0
  const token = ++request
  if (!next) return
  const search = next.query.trim()
  const records = search ? await api.search(search, 10).catch(() => []) : lib.records
  if (!alive || token !== request) return
  matches.value = records
    .filter((r) => r.id !== props.recordId && !lib.records.find((item) => item.id === r.id)?.secret_kind)
    .slice(0, 10)
    .map((r) => ({ id: r.id, title: r.title || 'Без названия' }))
}

async function choose(index: number) {
  const current = query.value
  if (!current || choosing.value) return
  const recordId = props.recordId
  const token = request
  let match = matches.value[index]
  choosing.value = true
  try {
    if (!match && canCreate.value) {
      const title = current.query.trim()
      const id = await api.createRecord({ title })
      await lib.changed()
      match = { id, title }
    }
    if (match && alive && token === request && recordId === props.recordId) {
      editor?.insertLink(current, match.id, match.title)
      showLinks(null)
    }
  } catch (error) {
    toast.error(String(error))
  } finally {
    choosing.value = false
  }
}

function keydown(event: KeyboardEvent) {
  if (!query.value) return false
  const count = matches.value.length + Number(canCreate.value)
  if (event.key === 'Escape') {
    showLinks(null)
    return true
  }
  if (!count) return false
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    selected.value = (selected.value + (event.key === 'ArrowDown' ? 1 : -1) + count) % count
    return true
  }
  if (event.key === 'Enter' || event.key === 'Tab') {
    choose(selected.value)
    return true
  }
  return false
}

onMounted(async () => {
  editor = createNoteEditor({
    root: host.value!,
    markdown: model.value,
    links: {
      titleOf: (id) => lib.titleById.get(id),
      open: (id) => {
        router.push(`/r/${id}`)
      },
    },
    onChange: (text) => {
      if (!source.value && alive) {
        rendered = text
        model.value = text
      }
    },
    onQuery: (next) => {
      if (!source.value) showLinks(next)
    },
    onKeyDown: keydown,
    openExternal: (url) => {
      openUrl(url).catch((error) => toast.error(String(error)))
    },
  })
  try {
    await editor.create()
  } catch (error) {
    console.error('Markdown editor initialization failed', error)
    if (alive) {
      failed.value = true
      source.value = true
    }
  } finally {
    loading.value = false
  }
})
watch(model, (text) => {
  if (!source.value && text !== rendered) {
    rendered = text
    editor?.replace(text)
  }
})
watch(source, (value) => {
  showLinks(null)
  if (!value) {
    rendered = model.value
    editor?.replace(model.value)
  }
})
watch(
  () => lib.version,
  () => editor?.refreshLinks(),
)
onBeforeUnmount(() => {
  alive = false
  request++
  editor?.destroy().catch(() => {})
})
defineExpose({ focus: () => (source.value ? sourceEditor.value?.focus() : editor?.focus()) })
</script>

<template>
  <section class="crypt-note-editor min-w-0" aria-label="Редактор заметки">
    <p v-if="failed" class="mb-3 text-sm text-muted-foreground" role="status">
      Не удалось загрузить визуальный редактор. Заметка доступна в Markdown.
    </p>
    <p v-else-if="loading && !source" class="py-4 text-sm text-muted-foreground" role="status">Загрузка редактора…</p>
    <div v-show="!source" ref="host" class="crypt-note-visual min-h-[300px]" />
    <MarkdownSource v-if="source" ref="sourceEditor" v-model="model" :record-id="recordId" />
    <div class="crypt-editor-hint" aria-hidden="true">
      {{ source ? '⌘F — поиск · Tab — отступ' : '/ — вставить блок · [[ — ссылка · ⌘B — жирный · ⌘I — курсив' }}
    </div>
    <div
      v-if="query"
      class="crypt-link-menu fixed z-50 max-h-72 w-72 overflow-auto rounded-xl border border-border bg-popover p-1 text-sm text-popover-foreground shadow-lg"
      :style="menuStyle"
      role="listbox"
      aria-label="Ссылка на запись"
      @mousedown.prevent
    >
      <button
        v-for="(match, index) in matches"
        :key="match.id"
        type="button"
        role="option"
        :aria-selected="selected === index"
        class="block w-full truncate rounded-lg px-3 py-2 text-left hover:bg-accent"
        :class="selected === index && 'bg-accent'"
        :disabled="choosing"
        @click="choose(index)"
      >
        {{ match.title }}
      </button>
      <button
        v-if="canCreate"
        type="button"
        role="option"
        :aria-selected="selected === matches.length"
        class="block w-full truncate rounded-lg px-3 py-2 text-left text-primary hover:bg-accent"
        :class="selected === matches.length && 'bg-accent'"
        :disabled="choosing"
        @click="choose(matches.length)"
      >
        + Создать «{{ query.query.trim() }}»
      </button>
      <p v-if="!matches.length && !canCreate" class="px-3 py-2 text-muted-foreground">Начните вводить название</p>
    </div>
  </section>
</template>
