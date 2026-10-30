<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { KeyRound, Plus, Search, LockKeyhole, CreditCard, FileLock, Terminal, Key } from '@lucide/vue'
import { toast } from 'vue-sonner'
import RecordContextMenu from '@/components/RecordContextMenu.vue'
import SshImportDialog from '@/components/secret/SshImportDialog.vue'
import PageHeader from '@/components/PageHeader.vue'
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
    <PageHeader title="Секреты" :sub="`${secrets.length}`"
      ><template #icon><KeyRound class="text-secret size-5" /></template>
      <Button variant="ghost" size="sm" @click="importOpen = true">Импорт из ~/.ssh</Button>
      <DropdownMenu
        ><DropdownMenuTrigger as-child
          ><Button size="sm" variant="outline"
            ><Plus class="size-4 text-secret" /> Новый секрет</Button
          ></DropdownMenuTrigger
        ><DropdownMenuContent align="end"
          ><DropdownMenuItem v-for="kind in kinds" :key="kind" @select="create(kind)"
            ><component :is="icons[kind]" class="size-4 text-secret" />{{ SECRET_KIND_LABEL[kind] }}</DropdownMenuItem
          ></DropdownMenuContent
        ></DropdownMenu
      >
    </PageHeader>
    <div class="flex flex-wrap gap-1 border-b px-6 pt-3" role="tablist" aria-label="Виды секретов">
      <button
        role="tab"
        :aria-selected="tab === null"
        class="border-b-2 px-3 py-2 text-sm"
        :class="tab === null ? 'border-secret text-secret' : 'border-transparent text-muted-foreground'"
        @click="tab = null"
      >
        Все <span class="ml-1 text-xs">{{ secrets.length }}</span></button
      ><button
        v-for="kind in kinds"
        :key="kind"
        role="tab"
        :aria-selected="tab === kind"
        class="border-b-2 px-3 py-2 text-sm"
        :class="tab === kind ? 'border-secret text-secret' : 'border-transparent text-muted-foreground'"
        @click="tab = kind"
      >
        {{ labels[kind] }} <span class="ml-1 text-xs">{{ secrets.filter((r) => r.secret_kind === kind).length }}</span>
      </button>
    </div>
    <div class="flex items-center gap-2 px-6 py-4">
      <Search class="size-4 text-muted-foreground" /><input
        v-model="query"
        placeholder="Найти секрет…"
        aria-label="Поиск секретов"
        class="h-9 w-full max-w-md rounded-md border bg-transparent px-3 text-sm outline-none focus:border-secret"
      />
    </div>
    <div class="flex-1 overflow-auto px-6">
      <RecordContextMenu v-for="r in filtered" :key="r.id" :record="r"><RouterLink
        :to="`/r/${r.id}`"
        class="hover:bg-accent flex items-center gap-3 rounded-lg border-b px-3 py-3.5"
        ><span class="bg-secret/10 flex size-9 items-center justify-center rounded-lg"
          ><component :is="icons[r.secret_kind!]" class="size-4 text-secret" /></span
        ><span class="min-w-0 flex-1 truncate text-sm font-medium">{{ r.title || 'Без названия' }}</span
        ><span class="text-xs text-muted-foreground">{{ lib.folders.find((f) => f.id === r.folder)?.name }}</span
        ><time class="w-16 text-right text-xs text-muted-foreground">{{ fmt.format(r.updated_at) }}</time></RouterLink
      ></RecordContextMenu>
      <div v-if="!filtered.length" class="py-20 text-center">
        <KeyRound class="mx-auto mb-3 size-8 text-secret" />
        <p class="text-sm">{{ query ? 'Ничего не найдено' : 'Секретов пока нет' }}</p>
        <p class="mt-1 text-xs text-muted-foreground">Создайте логин, ключ, токен, карту или защищённую заметку.</p>
      </div>
    </div>
    <SshImportDialog v-model:open="importOpen" />
  </div>
</template>
