<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { markdown, markdownLanguage } from '@codemirror/lang-markdown'
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
import { EditorState } from '@codemirror/state'
import { EditorView, keymap, placeholder } from '@codemirror/view'
import { api } from '@/lib/api'
import { useLibrary } from '@/stores/library'
import { livePreview, previewTheme, refreshLinks } from './livePreview'
import { wikilinkCompletion } from './wikilinkExtension'

const model = defineModel<string>({ required: true })
const props = defineProps<{ recordId: string }>()
const lib = useLibrary()
const router = useRouter()
const host = ref<HTMLDivElement | null>(null)
let view: EditorView | null = null

onMounted(() => {
  view = new EditorView({
    parent: host.value!,
    state: EditorState.create({
      doc: model.value,
      extensions: [
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
        markdown({ base: markdownLanguage }),
        EditorView.lineWrapping,
        placeholder('Пишите здесь. [[ — ссылка на запись'),
        livePreview({
          titleOf: (id) => lib.titleById.get(id),
          open: (id) => router.push(`/r/${id}`),
        }),
        wikilinkCompletion({
          async find(q) {
            const typeName = (t: string | null) => (t ? lib.typeById.get(t)?.name : undefined)
            if (!q.trim()) {
              return lib.records
                .filter((r) => r.id !== props.recordId && !r.secret_kind)
                .slice(0, 10)
                .map((r) => ({ id: r.id, title: r.title, detail: typeName(r.type_id) }))
            }
            const hits = await api.search(q, 10).catch(() => [])
            return hits
              .filter((h) => h.id !== props.recordId)
              .map((h) => ({
                id: h.id,
                title: h.title,
                detail: typeName(lib.records.find((r) => r.id === h.id)?.type_id ?? null),
              }))
          },
          async create(title) {
            const id = await api.createRecord({ title })
            await lib.changed()
            return id
          },
        }),
        previewTheme,
        EditorView.contentAttributes.of({ 'aria-label': 'Заметка', spellcheck: 'true', lang: 'ru' }),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) model.value = u.state.doc.toString()
        }),
      ],
    }),
  })
})

// Titles changed somewhere: re-render link labels.
watch(
  () => lib.version,
  () => view?.dispatch({ effects: refreshLinks.of(null) }),
)

// External replacement (rare): keep the editor in sync without fighting the user.
watch(model, (v) => {
  if (view && v !== view.state.doc.toString()) {
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: v } })
  }
})

onBeforeUnmount(() => view?.destroy())
defineExpose({ focus: () => view?.focus() })
</script>

<template>
  <div ref="host" class="min-h-[300px] cursor-text" @click.self="view?.focus()" />
</template>
