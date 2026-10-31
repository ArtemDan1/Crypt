<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { ChevronRight, FileText, Folder as FolderIcon, FolderOpen } from '@lucide/vue'
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
import RecordContextMenu from '@/components/RecordContextMenu.vue'
import { api, CryptApiError } from '@/lib/api'
import type { Folder, Id } from '@/lib/types'
import { useLibrary } from '@/stores/library'

const props = defineProps<{ parent: Id | null; depth: number; activeId?: string; activeRecordId?: string }>()
const lib = useLibrary()
const router = useRouter()
const children = computed(() =>
  lib.folders.filter((f) => f.parent === props.parent).sort((a, b) => a.name.localeCompare(b.name, 'ru')),
)
const open = ref<Set<Id>>(new Set())
const editing = ref<Id | null>(null)
const draft = ref('')
const inputs = ref<Record<Id, HTMLInputElement | null>>({})

const files = computed(() =>
  lib.records
    .filter((r) => r.folder === props.parent && !r.secret_kind)
    .sort((a, b) => a.title.localeCompare(b.title, 'ru')),
)
watch(
  () => [props.activeId, lib.folders] as const,
  () => {
    const ancestors = new Set<Id>()
    let id = props.activeId
    while (id && !ancestors.has(id)) {
      ancestors.add(id)
      id = lib.folders.find((f) => f.id === id)?.parent ?? undefined
    }
    open.value = new Set([...open.value, ...ancestors])
  },
  { immediate: true },
)
function toggle(id: Id) {
  const s = new Set(open.value)
  s.has(id) ? s.delete(id) : s.add(id)
  open.value = s
}
function descendants(id: Id): Set<Id> {
  const out = new Set<Id>([id])
  let grew = true
  while (grew) {
    grew = false
    for (const f of lib.folders)
      if (f.parent && out.has(f.parent) && !out.has(f.id)) {
        out.add(f.id)
        grew = true
      }
  }
  return out
}
function moveTargets(f: Folder) {
  const banned = descendants(f.id)
  return lib.folders.filter((x) => !banned.has(x.id) && x.id !== f.parent)
}
function report(e: unknown) {
  const msg = e instanceof CryptApiError ? e.message : String(e)
  toast.error(
    msg.includes('has records') ? 'В папке есть записи' : msg.includes('children') ? 'В папке есть подпапки' : msg,
  )
}
async function startRename(f: Folder) {
  editing.value = f.id
  draft.value = f.name
  await nextTick()
  inputs.value[f.id]?.select()
}
async function commitRename(f: Folder) {
  const name = draft.value.trim()
  editing.value = null
  if (!name || name === f.name) return
  try {
    await api.renameFolder(f.id, name)
    await lib.changed()
  } catch (e) {
    report(e)
  }
}
async function addChild(f: Folder) {
  try {
    const id = await api.createFolder('Новая папка', f.id)
    await lib.changed()
    open.value = new Set(open.value).add(f.id)
    await startRename(lib.folders.find((x) => x.id === id)!)
  } catch (e) {
    report(e)
  }
}
async function newRecord(f: Folder) {
  try {
    await router.push(`/r/${await lib.createRecord({ folder: f.id })}`)
  } catch (e) {
    report(e)
  }
}
async function move(f: Folder, parent: Id | null) {
  try {
    await api.moveFolder(f.id, parent)
    await lib.changed()
  } catch (e) {
    report(e)
  }
}
async function remove(f: Folder) {
  try {
    await api.deleteFolder(f.id)
    await lib.changed()
  } catch (e) {
    report(e)
  }
}
defineExpose({ startRename })
</script>

<template>
  <div>
    <div v-for="f in children" :key="f.id">
      <ContextMenu>
        <ContextMenuTrigger as-child>
          <div
            class="group flex h-[30px] items-center gap-1 rounded-[7px] pr-2 text-[13px] transition-colors"
            :class="
              activeId === f.id
                ? 'bg-sidebar-accent text-foreground font-semibold'
                : 'text-foreground hover:bg-foreground/[.06] font-medium'
            "
            :style="{ paddingLeft: `${4 + depth * 14}px` }"
          >
            <button
              class="text-muted-foreground flex size-4 items-center justify-center"
              :aria-expanded="open.has(f.id)"
              :aria-label="`${open.has(f.id) ? 'Свернуть' : 'Развернуть'} ${f.name}`"
              @click="toggle(f.id)"
            >
              <ChevronRight class="size-3.5 transition-transform" :class="open.has(f.id) && 'rotate-90'" />
            </button>
            <component
              :is="open.has(f.id) ? FolderOpen : FolderIcon"
              class="size-4 shrink-0"
              :class="activeId === f.id ? 'text-primary' : 'text-muted-foreground'"
            />
            <input
              v-if="editing === f.id"
              :ref="(el) => (inputs[f.id] = el as HTMLInputElement)"
              v-model="draft"
              class="bg-background min-w-0 flex-1 rounded border px-1 outline-none"
              @keydown.enter="commitRename(f)"
              @keydown.esc="editing = null"
              @blur="commitRename(f)"
            />
            <RouterLink
              v-else
              :to="`/folder/${f.id}`"
              class="min-w-0 flex-1 truncate"
              @click="open = new Set(open).add(f.id)"
              @dblclick="startRename(f)"
              >{{ f.name }}</RouterLink
            >
            <span class="text-muted-foreground text-xs">{{ lib.countByFolder.get(f.id) || '' }}</span>
          </div>
        </ContextMenuTrigger>
        <ContextMenuContent class="w-52">
          <ContextMenuItem @select="newRecord(f)">Новая запись</ContextMenuItem>
          <ContextMenuItem @select="addChild(f)">Новая подпапка</ContextMenuItem>
          <ContextMenuItem @select="startRename(f)">Переименовать</ContextMenuItem>
          <ContextMenuSub>
            <ContextMenuSubTrigger>Переместить в…</ContextMenuSubTrigger>
            <ContextMenuSubContent class="max-h-72 w-48 overflow-auto">
              <ContextMenuItem v-if="f.parent" @select="move(f, null)">В корень</ContextMenuItem>
              <ContextMenuItem v-for="t in moveTargets(f)" :key="t.id" @select="move(f, t.id)">{{
                t.name
              }}</ContextMenuItem>
            </ContextMenuSubContent>
          </ContextMenuSub>
          <ContextMenuSeparator />
          <ContextMenuItem variant="destructive" @select="remove(f)">Удалить</ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>
      <FolderTree
        v-if="open.has(f.id)"
        :parent="f.id"
        :depth="depth + 1"
        :active-id="activeId"
        :active-record-id="activeRecordId"
      />
    </div>
    <RecordContextMenu v-for="file in files" :key="file.id" :record="file"
      ><RouterLink
        :to="`/r/${file.id}`"
        :aria-current="activeRecordId === file.id ? 'page' : undefined"
        class="flex h-[30px] items-center gap-1 rounded-[7px] pr-2 text-[13px] transition-colors"
        :class="
          activeRecordId === file.id
            ? 'bg-sidebar-accent text-foreground font-semibold'
            : 'text-foreground hover:bg-foreground/[.06]'
        "
        :style="{ paddingLeft: `${24 + depth * 14}px` }"
      >
        <FileText class="text-muted-foreground size-4 shrink-0" />
        <span class="min-w-0 flex-1 truncate" :class="!file.title && 'text-muted-foreground'">{{
          file.title || 'Без названия'
        }}</span>
      </RouterLink></RecordContextMenu
    >
  </div>
</template>
