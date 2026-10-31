<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Clock, FolderRoot, KeyRound, Lock, Plus, Search, SlidersHorizontal, Trash2 } from '@lucide/vue'
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from '@/components/ui/dropdown-menu'
import FolderTree from '@/components/FolderTree.vue'
import KindTile from '@/components/crypt/KindTile.vue'
import { toneForType } from '@/lib/tones'
import AutolockRing from '@/components/crypt/AutolockRing.vue'
import { api } from '@/lib/api'
import { toast } from 'vue-sonner'
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
const plural = (n: number) => {
  const m10 = n % 10,
    m100 = n % 100
  if (m10 === 1 && m100 !== 11) return 'запись'
  if (m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14)) return 'записи'
  return 'записей'
}
// Vault name: click to rename in place (Enter saves, Esc cancels).
const renaming = ref(false)
const nameDraft = ref('')
const nameInput = ref<HTMLInputElement | null>(null)
async function startRename() {
  nameDraft.value = lib.name
  renaming.value = true
  await nextTick()
  nameInput.value?.select()
}
async function commitRename() {
  if (!renaming.value) return
  renaming.value = false
  const next = nameDraft.value.trim()
  if (!next || next === lib.name) return
  try {
    await lib.rename(next)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось переименовать хранилище')
  }
}
const row = (active: boolean) =>
  active ? 'bg-sidebar-accent text-foreground font-semibold' : 'text-foreground hover:bg-foreground/[.06] font-medium'

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
  <nav aria-label="Навигация" class="bg-sidebar flex h-full w-62 shrink-0 flex-col border-r text-[13px]">
    <div data-tauri-drag-region class="flex h-[52px] shrink-0 items-center justify-end px-2.5">
      <button
        class="text-muted-foreground hover:bg-foreground/[.08] relative flex size-8 items-center justify-center rounded-lg transition-colors active:scale-95"
        aria-label="Заблокировать (⌘L)"
        title="Заблокировать (⌘L)"
        @click="session.lock()"
      >
        <AutolockRing :left="session.autolockLeft" :total="session.autolockTotal" :size="28" />
        <Lock class="size-3.5" :stroke-width="2.2" />
      </button>
    </div>
    <div class="flex flex-col gap-2.5 px-2.5 pb-3">
      <div class="flex items-center gap-2.5 px-1.5">
        <div class="min-w-0 flex-1 leading-tight">
          <input
            v-if="renaming"
            ref="nameInput"
            v-model="nameDraft"
            aria-label="Название хранилища"
            maxlength="60"
            class="bg-card ring-primary/60 -mx-1 w-[calc(100%+.5rem)] rounded-md px-1 text-[14px] font-semibold outline-none ring-1"
            @keydown.enter.prevent="commitRename"
            @keydown.esc.prevent="renaming = false"
            @blur="commitRename"
          />
          <button
            v-else
            type="button"
            class="hover:bg-foreground/[.06] -mx-1 block max-w-[calc(100%+.5rem)] truncate rounded-md px-1 text-left text-[14px] font-semibold transition-colors"
            title="Переименовать хранилище"
            @click="startRename"
          >
            {{ lib.name || 'Личное' }}
          </button>
          <div class="text-muted-foreground text-xs">{{ total }} {{ plural(total) }}</div>
        </div>
        <button
          class="bg-primary text-primary-foreground flex size-[30px] items-center justify-center rounded-lg transition hover:brightness-110 active:scale-95"
          aria-label="Новая запись (⌘N)"
          title="Новая запись (⌘N)"
          @click="newRecord"
        >
          <Plus class="size-[15px]" :stroke-width="2.4" />
        </button>
      </div>
      <button
        class="bg-card text-muted-foreground hover:text-foreground flex h-8 items-center gap-2 rounded-lg border px-2.5 text-left transition-colors"
        @click="emit('search')"
      >
        <Search class="size-3.5" />
        <span class="flex-1">Поиск</span>
        <kbd class="rounded border px-1.5 font-mono text-[11px]">⌘K</kbd>
      </button>
    </div>
    <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-auto px-2.5 pt-0.5 pb-3">
      <section class="flex flex-col gap-px">
        <RouterLink
          to="/recent"
          class="flex h-[30px] items-center gap-2.5 rounded-[7px] px-2 transition-colors"
          :class="row(isActive('/recent'))"
        >
          <Clock class="size-4" :class="isActive('/recent') ? 'text-primary' : 'text-muted-foreground'" />Недавние
        </RouterLink>
        <RouterLink
          to="/secrets"
          class="flex h-[30px] items-center gap-2.5 rounded-[7px] px-2 transition-colors"
          :class="row(isActive('/secrets'))"
        >
          <KeyRound class="text-secret size-4" />
          <span class="flex-1">Секреты</span>
          <span class="text-muted-foreground text-xs font-normal">{{ lib.secretsCount || '' }}</span>
        </RouterLink>
      </section>
      <section class="flex flex-col gap-px">
        <h4 class="text-muted-foreground px-2 pb-1 text-[11px] font-semibold tracking-wider uppercase">Типы</h4>
        <RouterLink
          v-for="t in lib.types"
          :key="t.id"
          :to="`/t/${t.id}`"
          class="flex h-[30px] items-center gap-2.5 rounded-[7px] px-2 transition-colors"
          :class="row(isActive(`/t/${t.id}`))"
        >
          <KindTile :icon="typeIcon(t.icon)" :tone="toneForType(lib.types, t.id)" size="sm" />
          <span class="flex-1 truncate">{{ t.name }}</span>
          <span class="text-muted-foreground text-xs font-normal">{{ lib.countByType.get(t.id) || '' }}</span>
        </RouterLink>
        <RouterLink
          to="/types"
          class="text-muted-foreground flex h-[30px] items-center gap-2.5 rounded-[7px] px-2 transition-colors hover:bg-foreground/[.06]"
          :class="route.path.startsWith('/types') && 'bg-sidebar-accent text-foreground'"
        >
          <Plus class="size-4" /> <span>{{ lib.types.length ? 'Настроить типы' : 'Новый тип…' }}</span>
        </RouterLink>
      </section>
      <section class="flex flex-col gap-px">
        <div class="flex items-center px-2 pb-1">
          <h4 class="text-muted-foreground flex-1 text-[11px] font-semibold tracking-wider uppercase">Файлы</h4>
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
          class="flex h-[30px] items-center gap-2.5 rounded-[7px] px-2 font-mono transition-colors"
          :class="row(isActive('/'))"
        >
          <FolderRoot class="size-4" :class="isActive('/') ? 'text-primary' : 'text-muted-foreground'" /> /
        </RouterLink>
        <FolderTree ref="tree" :parent="null" :depth="0" :active-id="activeFolder" :active-record-id="activeRecord" />
        <p
          v-if="!lib.folders.length && !lib.records.some((r) => !r.secret_kind)"
          class="text-muted-foreground px-2 text-xs"
        >
          Здесь пока пусто
        </p>
      </section>
      <section v-if="topTags.length" class="flex flex-col gap-1.5">
        <h4 class="text-muted-foreground px-2 text-[11px] font-semibold tracking-wider uppercase">Теги</h4>
        <div class="flex flex-wrap gap-1 px-1.5">
          <RouterLink
            v-for="[tag, n] in topTags"
            :key="tag"
            :to="`/tag/${encodeURIComponent(tag)}`"
            :title="`${n}`"
            class="flex h-6 items-center rounded-full px-2 text-xs transition-colors"
            :class="
              route.path === `/tag/${encodeURIComponent(tag)}`
                ? 'bg-primary-soft text-primary-soft-fg'
                : 'bg-foreground/[.07] text-foreground-2 hover:bg-foreground/[.11]'
            "
            >#{{ tag }}</RouterLink
          >
        </div>
      </section>
    </div>
    <div class="flex shrink-0 items-center gap-0.5 border-t px-2.5 py-2">
      <RouterLink
        to="/trash"
        class="text-muted-foreground flex h-8 flex-1 items-center gap-2 rounded-[7px] px-2 transition-colors hover:bg-foreground/[.06]"
        :class="isActive('/trash') && 'bg-sidebar-accent text-foreground'"
      >
        <Trash2 class="size-[15px]" /> <span>Корзина</span>
        <span class="text-xs">{{ lib.trashCount || '' }}</span>
      </RouterLink>
      <RouterLink
        to="/settings"
        aria-label="Настройки"
        title="Настройки"
        class="text-muted-foreground flex size-8 items-center justify-center rounded-[7px] transition-colors hover:bg-foreground/[.06]"
        :class="isActive('/settings') && 'bg-sidebar-accent text-foreground'"
      >
        <SlidersHorizontal class="size-4" />
      </RouterLink>
    </div>
  </nav>
</template>
