<script setup lang="ts">
import { computed, ref } from 'vue'
import { KeyRound, Plus, X } from '@lucide/vue'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { typeIcon } from '@/lib/icons'
import type { Id } from '@/lib/types'
import SecretChip from '@/components/secret/SecretChip.vue'
import { api } from '@/lib/api'
import { SECRET_KIND_LABEL } from '@/lib/secretFields'
import type { SecretKind } from '@/lib/types'
import { toast } from 'vue-sonner'
import { useLibrary } from '@/stores/library'

const props = defineProps<{ value: Id[]; multiple: boolean; secrets: boolean; targetType?: Id | null; selfId?: Id }>()
const emit = defineEmits<{ change: [ids: Id[]] }>()
const lib = useLibrary()
const q = ref('')
const open = ref(false)

const candidates = computed(() =>
  lib.records
    .filter((r) => r.id !== props.selfId && !props.value.includes(r.id))
    .filter((r) => (props.secrets ? !!r.secret_kind : !r.secret_kind))
    .filter((r) => !props.targetType || r.type_id === props.targetType)
    .filter((r) => !q.value.trim() || r.title.toLowerCase().includes(q.value.trim().toLowerCase()))
    .slice(0, 30),
)
function add(id: Id) {
  emit('change', props.multiple ? [...props.value, id] : [id])
  q.value = ''
  if (!props.multiple) open.value = false
}
function remove(id: Id) {
  emit(
    'change',
    props.value.filter((x) => x !== id),
  )
}
async function create(kind: SecretKind) {
  try {
    const id = await api.createRecord({ title: `Новый секрет · ${SECRET_KIND_LABEL[kind]}`, secret_kind: kind })
    await lib.changed()
    add(id)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось создать секрет')
  }
}
const title = (id: Id) => lib.titleById.get(id)
</script>

<template>
  <div class="flex flex-wrap items-center gap-1.5 py-1">
    <span
      v-for="id in value"
      :key="id"
      class="flex h-[26px] items-center gap-1.5 rounded-[7px] pr-1 pl-2 text-[13px]"
      :class="secrets ? 'bg-secret-bg text-secret-fg font-medium' : 'bg-card border'"
    >
      <SecretChip v-if="secrets" :id="id" />
      <RouterLink v-else-if="title(id) !== undefined" :to="`/r/${id}`" class="hover:underline">{{
        title(id) || 'Без названия'
      }}</RouterLink>
      <span v-else class="text-muted-foreground line-through">удалено</span>
      <button
        class="text-muted-foreground hover:text-foreground rounded p-0.5"
        :aria-label="`Убрать ${title(id) ?? ''}`"
        @click="remove(id)"
      >
        <X class="size-3" />
      </button>
    </span>
    <Popover v-if="multiple || !value.length" v-model:open="open">
      <PopoverTrigger as-child>
        <button
          class="text-muted-foreground hover:text-foreground border-border-strong flex h-[26px] items-center gap-1 rounded-[7px] border border-dashed px-2 text-[13px]"
        >
          <Plus class="size-3.5" /> {{ secrets ? 'Привязать секрет' : 'Связать' }}
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" class="w-72 p-1.5">
        <input
          v-model="q"
          class="mb-1 h-8 w-full rounded-md border bg-transparent px-2 text-[13px] outline-none"
          placeholder="Найти…"
          aria-label="Найти запись"
        />
        <div class="max-h-64 overflow-auto">
          <button
            v-for="r in candidates"
            :key="r.id"
            class="hover:bg-accent flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-[13px]"
            @click="add(r.id)"
          >
            <KeyRound v-if="secrets" class="text-secret size-3.5" />
            <component
              :is="typeIcon(lib.typeById.get(r.type_id ?? '')?.icon)"
              v-else
              class="text-muted-foreground size-3.5"
            />
            <span class="truncate">{{ r.title || 'Без названия' }}</span>
          </button>
          <p v-if="!candidates.length" class="text-muted-foreground px-2 py-2 text-xs">
            {{ secrets ? 'Секретов пока нет' : 'Ничего не найдено' }}
          </p>
          <div v-if="secrets && !candidates.length" class="border-t p-2">
            <p class="mb-2 text-xs text-muted-foreground">Создать секрет</p>
            <button
              v-for="(label, kind) in SECRET_KIND_LABEL"
              :key="kind"
              class="hover:bg-accent w-full rounded px-2 py-1.5 text-left text-sm"
              @click="create(kind)"
            >
              {{ label }}
            </button>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  </div>
</template>
