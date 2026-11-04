import type { Component } from 'vue'
import type { Router } from 'vue-router'
import {
  Clock,
  Dices,
  FileText,
  Folder,
  FolderRoot,
  Hash,
  Import,
  KeyRound,
  Shapes,
  SlidersHorizontal,
  Trash2,
} from '@lucide/vue'
import { typeIcon } from './icons'
import type { Folder as FolderT, RecordSummary, TypeDef } from './types'

export interface PageInfo {
  title: string
  icon: Component
  secret?: boolean
}

export interface PageLookup {
  records: RecordSummary[]
  folders: FolderT[]
  typeById: Map<string, TypeDef>
}

export const ROOT_TITLE = 'Все файлы'

/** Tab title and icon for an in-app path. */
export function pageInfo(router: Router, path: string, lib: PageLookup): PageInfo {
  const to = router.resolve(path)
  const param = (k: string) => String(to.params[k] ?? '')
  const pattern = to.matched.at(-1)?.path ?? ''
  switch (pattern) {
    case '/':
      return { title: ROOT_TITLE, icon: FolderRoot }
    case '/recent':
      return { title: 'Недавние', icon: Clock }
    case '/r/:id': {
      const r = lib.records.find((r) => r.id === param('id'))
      const type = r?.type_id ? lib.typeById.get(r.type_id) : undefined
      if (r?.secret_kind) return { title: r.title || 'Без названия', icon: KeyRound, secret: true }
      return { title: r ? r.title || 'Без названия' : 'Запись', icon: type ? typeIcon(type.icon) : FileText }
    }
    case '/t/:typeId': {
      const t = lib.typeById.get(param('typeId'))
      return { title: t?.name ?? 'Тип', icon: typeIcon(t?.icon) }
    }
    case '/types/:typeId?':
      return { title: 'Типы', icon: Shapes }
    case '/folder/:folderId':
      return { title: lib.folders.find((f) => f.id === param('folderId'))?.name ?? 'Папка', icon: Folder }
    case '/tag/:tag':
      return { title: `#${param('tag')}`, icon: Hash }
    case '/import':
      return { title: 'Импорт', icon: Import }
    case '/secrets':
      return { title: 'Секреты', icon: KeyRound, secret: true }
    case '/generator':
      return { title: 'Генератор паролей', icon: Dices }
    case '/trash':
      return { title: 'Корзина', icon: Trash2 }
    case '/settings':
      return { title: 'Настройки', icon: SlidersHorizontal }
    default:
      return { title: 'Crypt', icon: FileText }
  }
}
