<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { Dices, Download, KeyRound, Plus, Search, LockKeyhole, CreditCard, FileLock, Terminal, Key } from '@lucide/vue'
import { toast } from 'vue-sonner'
import RecordContextMenu from '@/components/RecordContextMenu.vue'
import SshImportDialog from '@/components/secret/SshImportDialog.vue'
import KindTile from '@/components/crypt/KindTile.vue'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from '@/components/ui/dropdown-menu'
import { SECRET_KIND_LABEL } from '@/lib/secretFields'
import type { SecretKind } from '@/lib/types'
import { api } from '@/lib/api'
import { useLibrary } from '@/stores/library'
const lib = useLibrary(),
  router = useRouter(),
  query = ref(''),
  tab = ref<SecretKind | null>(null)
const importOpen = ref(false)
const kinds: SecretKind[] = ['login', 'sshkey', 'token', 'card', 'note']
const labels: Record<SecretKind, string> = {
  login: 'Логины',
  sshkey: 'SSH-ключи',
  token: 'Токены',
  card: 'Карты',
  note: 'Заметки',
}
const icons = { login: LockKeyhole, sshkey: Terminal, token: Key, card: CreditCard, note: FileLock }
const secrets = computed(() => lib.records.filter((r) => r.secret_kind))
const filtered = computed(() =>
  secrets.value.filter(
    (r) =>
      (!tab.value || r.secret_kind === tab.value) &&
      r.title.toLocaleLowerCase('ru').includes(query.value.toLocaleLowerCase('ru')),
  ),
)
const fmt = new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'short' })
const pill = 'bg-foreground/[.07] text-foreground-2 hover:bg-foreground/[.11]'
const folderName = (id: string | null) => (id ? lib.folders.find((f) => f.id === id)?.name : undefined)
async function create(kind: SecretKind) {
  try {
    const id = await api.createRecord({ title: `Новый секрет · ${SECRET_KIND_LABEL[kind]}`, secret_kind: kind })
    await lib.changed()
    await router.push(`/r/${id}`)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось создать секрет')
  }
}
</script>
<template>
  <div class="flex h-full flex-col">
    <header data-tauri-drag-region class="flex h-[52px] shrink-0 items-center gap-2 border-b pr-5 pl-7">
      <h1 class="flex-1 text-[17px] font-semibold tracking-[-.01em]">Секреты</h1>
      <Button
        variant="outline"
        size="sm"
        aria-label="Импорт из ~/.ssh"
        title="Импорт из ~/.ssh"
        @click="importOpen = true"
        ><Download class="size-4"
      /></Button>
      <Button variant="outline" size="sm" @click="router.push('/generator')"><Dices class="size-4" /> Генератор</Button>
      <DropdownMenu
        ><DropdownMenuTrigger as-child
          ><Button size="sm"><Plus class="size-4" /> Новый секрет</Button></DropdownMenuTrigger
        ><DropdownMenuContent align="end"
          ><DropdownMenuItem v-for="kind in kinds" :key="kind" @select="create(kind)"
            ><component :is="icons[kind]" class="text-secret size-4" />{{ SECRET_KIND_LABEL[kind] }}</DropdownMenuItem
          ></DropdownMenuContent
        ></DropdownMenu
      >
    </header>
    <div class="flex flex-col gap-3 px-7 pt-5 pb-3">
      <label
        class="bg-card text-muted-foreground focus-within:border-primary flex h-9 max-w-md items-center gap-2 rounded-lg border px-3 transition-colors"
      >
        <Search class="size-3.5" />
        <input
          v-model="query"
          placeholder="Найти секрет…"
          aria-label="Поиск секретов"
          class="text-foreground w-full bg-transparent text-sm outline-none"
        />
      </label>
      <div class="flex flex-wrap gap-1" role="tablist" aria-label="Виды секретов">
        <button
          role="tab"
          :aria-selected="tab === null"
          class="flex h-[26px] items-center gap-1.5 rounded-full px-2.5 text-xs font-medium transition-colors active:scale-95"
          :class="tab === null ? 'bg-foreground text-background' : pill"
          @click="tab = null"
        >
          Все <span class="opacity-65">{{ secrets.length }}</span></button
        ><button
          v-for="kind in kinds"
          :key="kind"
          role="tab"
          :aria-selected="tab === kind"
          class="flex h-[26px] items-center gap-1.5 rounded-full px-2.5 text-xs font-medium transition-colors active:scale-95"
          :class="tab === kind ? 'bg-foreground text-background' : pill"
          @click="tab = kind"
        >
          {{ labels[kind] }} <span class="opacity-65">{{ secrets.filter((r) => r.secret_kind === kind).length }}</span>
        </button>
      </div>
    </div>
    <div :key="tab ?? 'all'" class="anim-in flex-1 overflow-auto px-7 pb-8">
      <div v-if="filtered.length" class="bg-card overflow-hidden rounded-xl border">
        <RecordContextMenu v-for="r in filtered" :key="r.id" :record="r"
          ><RouterLink
            :to="`/r/${r.id}`"
            class="hover:bg-foreground/[.04] flex items-center gap-3 border-t px-3.5 py-2.5 transition-colors first:border-t-0"
            ><KindTile :icon="icons[r.secret_kind!]" tone="secret" size="lg" /><span
              class="flex min-w-0 flex-1 flex-col"
              ><span class="truncate text-[14px] font-semibold">{{ r.title || 'Без названия' }}</span
              ><span class="text-muted-foreground truncate text-xs"
                >{{ SECRET_KIND_LABEL[r.secret_kind!]
                }}{{ folderName(r.folder) ? ` · ${folderName(r.folder)}` : '' }}</span
              ></span
            ><time class="text-muted-foreground w-16 text-right text-xs">{{
              fmt.format(r.updated_at)
            }}</time></RouterLink
          ></RecordContextMenu
        >
      </div>
      <div v-else class="py-20 text-center">
        <KindTile :icon="KeyRound" tone="secret" size="lg" class="mx-auto mb-3" />
        <p class="text-sm font-medium">{{ query ? 'Ничего не найдено' : 'Секретов пока нет' }}</p>
        <p class="text-muted-foreground mt-1 text-xs">Создайте логин, ключ, токен, карту или защищённую заметку.</p>
      </div>
    </div>
    <SshImportDialog v-model:open="importOpen" />
  </div>
</template>
