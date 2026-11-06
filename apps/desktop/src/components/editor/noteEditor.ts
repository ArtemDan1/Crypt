import { Crepe } from '@milkdown/crepe'
import { setCodeMirrorTheme } from '@milkdown/crepe/feature/code-mirror'
import { editorViewCtx, serializerCtx } from '@milkdown/kit/core'
import { Plugin } from '@milkdown/kit/prose/state'
import { $prose, replaceAll } from '@milkdown/kit/utils'
import { cryptLinks, linkQuery, refreshLinkLabels, wikilinkNode, type NoteLinks } from './milkdownLinks'
import { codeTheme } from './codeTheme'

export interface LinkQuery {
  query: string
  from: number
  to: number
  left: number
  top: number
}

interface NoteEditorOptions {
  root: HTMLElement
  markdown: string
  links: NoteLinks
  onChange(markdown: string): void
  onQuery?(query: LinkQuery | null): void
  onKeyDown?(event: KeyboardEvent): boolean
  openExternal?(url: string): void
}

function imageData(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result))
    reader.onerror = () => reject(reader.error)
    reader.readAsDataURL(file)
  })
}

export function createNoteEditor(options: NoteEditorOptions) {
  let ready = false
  let disposed = false
  let replacing = false
  let desired = options.markdown
  let pending: Promise<void> | undefined
  const crepe = new Crepe({
    root: options.root,
    defaultValue: options.markdown,
    features: { [Crepe.Feature.TopBar]: true },
    featureConfigs: {
      [Crepe.Feature.Placeholder]: { text: 'Пишите здесь. / — вставить блок, [[ — ссылка на запись' },
      [Crepe.Feature.CodeMirror]: {
        searchPlaceholder: 'Язык кода',
        noResultText: 'Язык не найден',
        copyText: 'Копировать',
        previewToggleText: (hidden) => (hidden ? 'Редактировать' : 'Скрыть код'),
      },
      [Crepe.Feature.TopBar]: {
        headingOptions: [
          { label: 'Текст', level: null },
          ...[1, 2, 3, 4, 5, 6].map((level) => ({ label: `Заголовок ${level}`, level })),
        ],
      },
      [Crepe.Feature.ImageBlock]: {
        onUpload: imageData,
        inlineUploadButton: 'Выбрать изображение',
        blockUploadButton: 'Выбрать изображение',
        inlineUploadPlaceholderText: 'или вставьте ссылку',
        blockUploadPlaceholderText: 'или вставьте ссылку',
        blockConfirmButton: 'Добавить',
        blockCaptionPlaceholderText: 'Подпись изображения',
      },
      [Crepe.Feature.LinkTooltip]: {
        inputPlaceholder: 'Адрес ссылки',
      },
      [Crepe.Feature.BlockEdit]: {
        textGroup: {
          label: 'Текст',
          text: { label: 'Обычный текст' },
          h1: { label: 'Заголовок 1' },
          h2: { label: 'Заголовок 2' },
          h3: { label: 'Заголовок 3' },
          h4: { label: 'Заголовок 4' },
          h5: { label: 'Заголовок 5' },
          h6: { label: 'Заголовок 6' },
          quote: { label: 'Цитата' },
          divider: { label: 'Разделитель' },
        },
        listGroup: {
          label: 'Списки',
          bulletList: { label: 'Маркированный список' },
          orderedList: { label: 'Нумерованный список' },
          taskList: { label: 'Задачи' },
        },
        advancedGroup: {
          label: 'Блоки',
          codeBlock: { label: 'Блок кода' },
          table: { label: 'Таблица' },
          image: { label: 'Изображение' },
          math: { label: 'Формула' },
        },
      },
    },
  })
  crepe.editor.use(cryptLinks(options.links)).use(
    $prose(
      (ctx) =>
        new Plugin({
          props: {
            attributes: { 'aria-label': 'Заметка', spellcheck: 'true', lang: 'ru' },
            handleKeyDown: (_view, event) => options.onKeyDown?.(event) ?? false,
            handleDOMEvents: {
              click: (_view, event) => {
                const anchor = (event.target as HTMLElement).closest<HTMLAnchorElement>('a[href]')
                if (!anchor || anchor.hasAttribute('data-crypt-link')) return false
                event.preventDefault()
                const href = anchor.getAttribute('href')!
                if (/^(https?:|mailto:)/i.test(href)) options.openExternal?.(href)
                return true
              },
            },
          },
          view: () => ({
            update: (view, previous) => {
              if (!ready || disposed || replacing) return
              if (!previous.doc.eq(view.state.doc)) {
                desired = ctx.get(serializerCtx)(view.state.doc)
                options.onChange(desired)
              }
              if (!options.onQuery) return
              const query = view.state.selection.empty ? linkQuery(view.state.doc, view.state.selection.from) : null
              if (query) {
                const rect = view.coordsAtPos(query.to)
                options.onQuery({ ...query, left: rect.left, top: rect.bottom + 6 })
              } else options.onQuery(null)
            },
          }),
        }),
    ),
  )

  function replace(markdown: string) {
    desired = markdown
    if (!ready || disposed) return
    replacing = true
    try {
      crepe.editor.action(replaceAll(markdown, true))
    } finally {
      replacing = false
    }
  }

  return {
    crepe,
    create() {
      pending ??= crepe.create().then(() => {
        if (disposed) return
        crepe.editor.action(setCodeMirrorTheme(codeTheme))
        ready = true
        if (desired !== options.markdown) replace(desired)
      })
      return pending
    },
    destroy() {
      disposed = true
      ready = false
      return (pending ?? Promise.resolve()).then(() => crepe.destroy()).then(() => {})
    },
    replace,
    focus() {
      if (ready && !disposed) crepe.editor.action((ctx) => ctx.get(editorViewCtx).focus())
    },
    refreshLinks() {
      if (ready && !disposed) refreshLinkLabels(options.root, options.links)
    },
    insertLink(query: LinkQuery, id: string, title: string) {
      if (!ready || disposed) return
      crepe.editor.action((ctx) => {
        const view = ctx.get(editorViewCtx)
        const current = linkQuery(view.state.doc, view.state.selection.from)
        if (!current || current.from !== query.from || current.to !== query.to || current.query !== query.query) return
        view.dispatch(
          view.state.tr
            .replaceWith(
              query.from,
              query.to,
              wikilinkNode.type(ctx).create({ id, alias: title.replace(/[\]|\n]/g, ' ') }),
            )
            .scrollIntoView(),
        )
        view.focus()
      })
    },
  }
}
