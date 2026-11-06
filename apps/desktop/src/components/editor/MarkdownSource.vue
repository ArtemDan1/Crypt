<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { markdown, markdownLanguage } from '@codemirror/lang-markdown'
import { indentWithTab } from '@codemirror/commands'
import { basicSetup } from 'codemirror'
import { EditorState } from '@codemirror/state'
import { EditorView, keymap, placeholder } from '@codemirror/view'
import { api } from '@/lib/api'
import { useLibrary } from '@/stores/library'
import { previewTheme } from './livePreview'
import { wikilinkCompletion } from './wikilinkExtension'
import { codeTheme } from './codeTheme'

const model = defineModel<string>({ required: true })
const props = defineProps<{ recordId: string }>()
const lib = useLibrary()
const host = ref<HTMLDivElement | null>(null)
let view: EditorView | null = null

onMounted(() => {
  view = new EditorView({
    parent: host.value!,
    state: EditorState.create({
      doc: model.value,
      extensions: [
        basicSetup,
        keymap.of([indentWithTab]),
        markdown({ base: markdownLanguage }),
        EditorView.lineWrapping,
        placeholder('Пишите здесь. [[ — ссылка на запись'),
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
        codeTheme,
        EditorView.theme({
          '.cm-scroller': { fontFamily: 'var(--font-mono)', fontSize: '13px', overflow: 'auto' },
          '.cm-gutters': {
            background: 'var(--muted)',
            color: 'var(--muted-foreground)',
            borderColor: 'var(--border)',
            marginRight: '10px',
          },
          '.cm-search': { background: 'var(--muted)', color: 'var(--foreground)' },
          '.cm-activeLine, .cm-activeLineGutter': { background: 'var(--accent)' },
        }),
        EditorView.contentAttributes.of({ 'aria-label': 'Исходный Markdown', spellcheck: 'false' }),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) model.value = u.state.doc.toString()
        }),
      ],
    }),
  })
})

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
