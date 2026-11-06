import { HighlightStyle, syntaxHighlighting } from '@codemirror/language'
import { EditorView } from '@codemirror/view'
import { tags } from '@lezer/highlight'

// Semantic app colors adapt to both themes without rebuilding code blocks.
export const codeTheme = [
  EditorView.theme({
    '&': { color: 'var(--foreground)', backgroundColor: 'var(--card)' },
    '.cm-content': { caretColor: 'var(--primary)' },
    '.cm-cursor': { borderLeftColor: 'var(--primary)' },
    '.cm-gutters': { color: 'var(--muted-foreground)', backgroundColor: 'var(--card)' },
    '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: 'var(--muted)' },
    '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': { backgroundColor: 'var(--primary-soft)' },
  }),
  syntaxHighlighting(
    HighlightStyle.define([
      { tag: [tags.keyword, tags.operator], color: 'var(--chart-2)' },
      { tag: [tags.string, tags.regexp], color: 'var(--primary)' },
      { tag: [tags.number, tags.bool, tags.null, tags.atom], color: 'var(--secret)' },
      { tag: [tags.function(tags.variableName), tags.typeName, tags.className], color: 'var(--chart-4)' },
      { tag: tags.comment, color: 'var(--muted-foreground)', fontStyle: 'italic' },
      { tag: [tags.heading, tags.strong], fontWeight: 'bold' },
      { tag: tags.emphasis, fontStyle: 'italic' },
      { tag: tags.invalid, color: 'var(--destructive)' },
    ]),
  ),
]
