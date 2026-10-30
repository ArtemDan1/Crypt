<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
  ContextMenuTrigger,
} from '@/components/ui/context-menu'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { api } from '@/lib/api'
import type { RecordSummary } from '@/lib/types'
import { makeLink } from '@/lib/wikilinks'
import { useLibrary } from '@/stores/library'

const props = defineProps<{ record: Pick<RecordSummary, 'id' | 'title' | 'folder' | 'secret_kind'> }>()
const lib = useLibrary()
const router = useRouter()
const renaming = ref(false)
const draft = ref('')
const folders = computed(() =>
  lib.folders.filter((f) => f.id !== props.record.folder).sort((a, b) => a.name.localeCompare(b.name, 'ru')),
)

async function run(action: () => Promise<unknown>, fail: string) {
  try {
    await action()
    await lib.changed()
  } catch (e) {
    toast.error(e instanceof Error ? e.message : fail)
  }
}
function startRename() {
  draft.value = props.record.title
  renaming.value = true
}
async function rename() {
  renaming.value = false
  const title = draft.value.trim()
  if (title !== props.record.title) await run(() => api.setTitle(props.record.id, title), 'Не удалось переименовать')
}
const move = (folder: string | null) => run(() => api.setFolder(props.record.id, folder), 'Не удалось переместить')
async function copyLink() {
  try {
    await navigator.clipboard.writeText(makeLink(props.record.id, props.record.title || 'Без названия'))
    toast('Ссылка скопирована')
  } catch {
    toast.error('Не удалось скопировать ссылку')
  }
}
async function trash() {
  await run(() => api.deleteRecord(props.record.id), 'Не удалось удалить')
  if (router.currentRoute.value.path === `/r/${props.record.id}`) router.push('/')
}
</script>

<template>
  <ContextMenu>
    <ContextMenuTrigger as-child><slot /></ContextMenuTrigger>
    <ContextMenuContent class="w-56">
      <ContextMenuItem @select="router.push(`/r/${record.id}`)">Открыть</ContextMenuItem>
      <ContextMenuItem @select="startRename">Переименовать</ContextMenuItem>
      <ContextMenuSub v-if="!record.secret_kind">
        <ContextMenuSubTrigger>Переместить в…</ContextMenuSubTrigger>
        <ContextMenuSubContent class="max-h-72 w-48 overflow-auto">
          <ContextMenuItem v-if="record.folder" @select="move(null)">В корень</ContextMenuItem>
          <ContextMenuItem v-for="f in folders" :key="f.id" @select="move(f.id)">{{ f.name }}</ContextMenuItem>
        </ContextMenuSubContent>
      </ContextMenuSub>
      <ContextMenuItem @select="copyLink">Копировать ссылку [[…]]</ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem variant="destructive" @select="trash">В корзину</ContextMenuItem>
    </ContextMenuContent>
  </ContextMenu>
  <Dialog v-model:open="renaming">
    <DialogContent class="sm:max-w-sm">
      <DialogHeader><DialogTitle>Переименовать</DialogTitle></DialogHeader>
      <form class="contents" @submit.prevent="rename">
        <Input v-model="draft" aria-label="Название" placeholder="Без названия" autofocus />
        <DialogFooter>
          <Button type="button" variant="ghost" @click="renaming = false">Отмена</Button>
          <Button type="submit">Сохранить</Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
