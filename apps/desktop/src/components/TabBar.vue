<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { Plus, X } from '@lucide/vue'
import { toast } from 'vue-sonner'
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuShortcut,
  ContextMenuTrigger,
} from '@/components/ui/context-menu'
import { pageInfo } from '@/lib/pageInfo'
import { useLibrary } from '@/stores/library'
import { useTabs, type Tab } from '@/stores/tabs'

const tabs = useTabs()
const lib = useLibrary()
const router = useRouter()
const info = (t: Tab) => pageInfo(router, tabs.pathOf(t), lib)

const dragging = ref<string | null>(null)
function drop(to: number) {
  if (dragging.value) tabs.move(dragging.value, to)
  dragging.value = null
}
async function toWindow(id: string) {
  try {
    await tabs.toWindow(id)
  } catch {
    toast.error('Не удалось открыть окно')
  }
}
</script>

<template>
  <div
    data-tauri-drag-region
    role="tablist"
    aria-label="Вкладки"
    class="bg-sidebar flex h-[38px] shrink-0 items-center gap-1 border-b px-2"
  >
    <ContextMenu v-for="(t, i) in tabs.tabs" :key="t.id">
      <ContextMenuTrigger as-child>
        <div
          role="tab"
          tabindex="0"
          draggable="true"
          :aria-selected="t.id === tabs.activeId"
          :title="info(t).title"
          class="group relative flex h-[28px] max-w-[220px] min-w-[72px] flex-1 cursor-default items-center gap-1.5 rounded-[7px] pr-1 pl-2.5 text-[12.5px] transition-colors select-none"
          :class="[
            t.id === tabs.activeId
              ? 'bg-background text-foreground font-medium shadow-[0_0_0_1px_var(--border),0_1px_2px_rgb(0_0_0/.04)]'
              : 'text-muted-foreground hover:bg-foreground/[.06] hover:text-foreground',
            dragging === t.id && 'opacity-50',
          ]"
          @mousedown.left="tabs.activate(t.id)"
          @auxclick.middle.prevent="tabs.close(t.id)"
          @keydown.enter="tabs.activate(t.id)"
          @dragstart="dragging = t.id"
          @dragend="dragging = null"
          @dragover.prevent
          @drop.prevent="drop(i)"
        >
          <component
            :is="info(t).icon"
            class="size-3.5 shrink-0"
            :class="info(t).secret ? 'text-secret' : t.id === tabs.activeId ? 'text-primary' : ''"
          />
          <span class="min-w-0 flex-1 truncate">{{ info(t).title }}</span>
          <button
            class="hover:bg-foreground/[.1] flex size-5 shrink-0 items-center justify-center rounded-[5px] opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
            :class="t.id === tabs.activeId && 'opacity-100'"
            :aria-label="`Закрыть вкладку ${info(t).title}`"
            title="Закрыть вкладку (⌘W)"
            @mousedown.stop
            @click.stop="tabs.close(t.id)"
          >
            <X class="size-3" />
          </button>
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent class="w-56">
        <ContextMenuItem @select="tabs.close(t.id)"
          >Закрыть<ContextMenuShortcut>⌘W</ContextMenuShortcut></ContextMenuItem
        >
        <ContextMenuItem :disabled="tabs.tabs.length < 2" @select="tabs.closeOthers(t.id)"
          >Закрыть остальные</ContextMenuItem
        >
        <ContextMenuSeparator />
        <ContextMenuItem @select="tabs.open(tabs.pathOf(t))">Дублировать</ContextMenuItem>
        <ContextMenuItem @select="toWindow(t.id)">Открыть в новом окне</ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
    <button
      class="text-muted-foreground hover:bg-foreground/[.06] hover:text-foreground flex size-7 shrink-0 items-center justify-center rounded-[7px] transition-colors"
      aria-label="Новая вкладка"
      title="Новая вкладка (⌘T)"
      @click="tabs.open()"
    >
      <Plus class="size-4" />
    </button>
    <div
      data-tauri-drag-region
      class="h-full min-w-6 flex-1"
      @drop.prevent="drop(tabs.tabs.length)"
      @dragover.prevent
    />
  </div>
</template>
