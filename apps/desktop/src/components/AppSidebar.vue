<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Clock, FolderRoot, Hash, KeyRound, Lock, Plus, Search, Settings, Trash2 } from '@lucide/vue'
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from '@/components/ui/dropdown-menu'
import { Button } from '@/components/ui/button'
import FolderTree from '@/components/FolderTree.vue'
import { api } from '@/lib/api'
import { typeIcon } from '@/lib/icons'
import { useLibrary } from '@/stores/library'
import { useSession } from '@/stores/session'

const emit = defineEmits<{ search: [] }>()
const lib = useLibrary()
const session = useSession()
const route = useRoute()
const router = useRouter()

const activeRecord = computed(() => (route.path.startsWith('/r/') ? String(route.params.id) : undefined))
const activeFolder = computed(() =>
  route.path.startsWith('/folder/')
    ? String(route.params.folderId)
    : (lib.records.find((r) => r.id === activeRecord.value)?.folder ?? undefined),
)
const topTags = computed(() => lib.tags.slice(0, 10))
const total = computed(() => lib.records.length)
const isActive = (p: string) => route.path === p

async function newRecord() {
  const id = await lib.createRecord({ folder: activeFolder.value ?? null })
  router.push(`/r/${id}`)
}
async function newRootRecord() {
  router.push(`/r/${await lib.createRecord({ folder: null })}`)
}
const tree = ref<InstanceType<typeof FolderTree> | null>(null)
async function newFolder() {
  const id = await api.createFolder('Новая папка', null)
  await lib.changed()
  await nextTick()
  const f = lib.folders.find((x) => x.id === id)
  if (f) tree.value?.startRename(f)
}
defineExpose({ newFolder })
</script>

<template>
  <nav aria-label="Навигация" class="bg-sidebar flex h-full w-60 shrink-0 flex-col border-r text-[13px]">
    <div data-tauri-drag-region class="flex h-[52px] shrink-0 items-center justify-end px-3">
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label="Заблокировать (⌘L)"
        title="Заблокировать (⌘L)"
        @click="session.lock()"
      >
        <Lock class="size-4" />
      </Button>
    </div>

    <div class="flex flex-col gap-2 px-2.5 pb-2.5">
      <div class="flex items-center gap-2.5 px-1.5 py-1">
        <span class="bg-foreground text-background flex size-7 items-center justify-center rounded-[7px] font-bold"
          >C</span
        >
        <div class="flex-1 leading-tight">
          <div class="font-semibold">Личное</div>
          <div class="text-muted-foreground text-xs">{{ total }} записей</div>
        </div>
        <Button
          variant="secondary"
          size="icon-sm"
          aria-label="Новая запись (⌘N)"
          title="Новая запись (⌘N)"
          @click="newRecord"
        >
          <Plus class="size-3.5" />
        </Button>
      </div>
      <button
        class="bg-card text-muted-foreground hover:text-foreground flex h-8 items-center gap-2 rounded-md border px-2.5 text-left"
        @click="emit('search')"
      >
        <Search class="size-3.5" />
        <span class="flex-1">Поиск</span>
        <kbd class="rounded border px-1.5 text-[11px]">⌘K</kbd>
      </button>
    </div>

    <div class="flex min-h-0 flex-1 flex-col gap-[18px] overflow-auto px-2.5 py-1">
      <section>
        <RouterLink
          v-for="item in [{ path: '/recent', label: 'Недавние', icon: Clock }]"
          :key="item.path"
          :to="item.path"
          class="flex h-[30px] items-center gap-2 rounded-md px-2"
          :class="isActive(item.path) ? 'bg-accent text-foreground' : 'text-foreground/85 hover:bg-accent/60'"
        >
          <component :is="item.icon" class="text-muted-foreground size-4" />{{ item.label }}
        </RouterLink>
      </section>
      <section>
        <h4 class="text-muted-foreground mb-1 px-2 text-[11px] font-medium tracking-wider uppercase">Типы</h4>
        <RouterLink
          v-for="t in lib.types"
          :key="t.id"
          :to="`/t/${t.id}`"
          class="flex h-[30px] items-center gap-2 rounded-md px-2"
          :class="isActive(`/t/${t.id}`) ? 'bg-accent text-foreground' : 'text-foreground/85 hover:bg-accent/60'"
        >
          <component :is="typeIcon(t.icon)" class="text-muted-foreground size-4" />
          <span class="flex-1 truncate">{{ t.name }}</span>
          <span class="text-muted-foreground text-xs">{{ lib.countByType.get(t.id) || '' }}</span>
        </RouterLink>
        <RouterLink
          to="/types"
          class="text-muted-foreground hover:bg-accent/60 hover:text-foreground flex h-[30px] items-center gap-2 rounded-md px-2"
          :class="route.path.startsWith('/types') && 'bg-accent text-foreground'"
        >
          <Plus class="size-4" /> <span>{{ lib.types.length ? 'Типы…' : 'Новый тип…' }}</span>
        </RouterLink>
      </section>

      <section>
        <h4 class="text-muted-foreground mb-1 px-2 text-[11px] font-medium tracking-wider uppercase">Хранилище</h4>
        <RouterLink
          to="/secrets"
          class="flex h-[30px] items-center gap-2 rounded-md px-2"
          :class="isActive('/secrets') ? 'bg-accent text-foreground' : 'text-foreground/85 hover:bg-accent/60'"
        >
          <KeyRound class="text-secret size-4" />
          <span class="flex-1">Секреты</span>
          <span class="text-muted-foreground text-xs">{{ lib.secretsCount || '' }}</span>
        </RouterLink>
      </section>

      <section>
        <div class="mb-1 flex items-center px-2">
          <h4 class="text-muted-foreground flex-1 text-[11px] font-medium tracking-wider uppercase">Файлы</h4>
          <DropdownMenu>
            <DropdownMenuTrigger as-child>
              <button
                class="text-muted-foreground hover:text-foreground"
                aria-label="Создать в корне"
                title="Создать в корне"
              >
                <Plus class="size-3.5" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem @select="newRootRecord">Новый файл</DropdownMenuItem>
              <DropdownMenuItem @select="newFolder">Новая директория</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
        <RouterLink
          to="/"
          class="flex h-[30px] items-center gap-2 rounded-md px-2 font-mono"
          :class="isActive('/') ? 'bg-accent text-foreground' : 'text-foreground/85 hover:bg-accent/60'"
        >
          <FolderRoot class="text-muted-foreground size-4" /> /
        </RouterLink>
        <FolderTree ref="tree" :parent="null" :depth="0" :active-id="activeFolder" :active-record-id="activeRecord" />
        <p
          v-if="!lib.folders.length && !lib.records.some((r) => !r.secret_kind)"
          class="text-muted-foreground px-2 text-xs"
        >
          Здесь пока пусто
        </p>
      </section>

      <section v-if="topTags.length">
        <h4 class="text-muted-foreground mb-1 px-2 text-[11px] font-medium tracking-wider uppercase">Теги</h4>
        <RouterLink
          v-for="[tag, n] in topTags"
          :key="tag"
          :to="`/tag/${encodeURIComponent(tag)}`"
          class="flex h-[30px] items-center gap-2 rounded-md px-2"
          :class="
            route.path === `/tag/${encodeURIComponent(tag)}`
              ? 'bg-accent text-foreground'
              : 'text-foreground/85 hover:bg-accent/60'
          "
        >
          <Hash class="text-muted-foreground size-3.5" />
          <span class="flex-1 truncate">{{ tag }}</span>
          <span class="text-muted-foreground text-xs">{{ n }}</span>
        </RouterLink>
      </section>
    </div>

    <div class="flex shrink-0 flex-col border-t p-1.5">
      <RouterLink
        to="/trash"
        class="text-muted-foreground hover:bg-accent/60 hover:text-foreground flex h-[30px] items-center gap-2 rounded-md px-2"
        :class="isActive('/trash') && 'bg-accent text-foreground'"
      >
        <Trash2 class="size-4" /> <span class="flex-1">Корзина</span>
        <span class="text-xs">{{ lib.trashCount || '' }}</span>
      </RouterLink>
      <RouterLink
        to="/settings"
        class="text-muted-foreground hover:bg-accent/60 hover:text-foreground flex h-[30px] items-center gap-2 rounded-md px-2"
        :class="isActive('/settings') && 'bg-accent text-foreground'"
      >
        <Settings class="size-4" /> <span>Настройки</span>
      </RouterLink>
    </div>
  </nav>
</template>
