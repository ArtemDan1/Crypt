<script setup lang="ts">
import { ref, watch } from 'vue'
import { Eye, RotateCcw, TriangleAlert } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import KindTile from '@/components/crypt/KindTile.vue'
import TrashPreview from '@/components/TrashPreview.vue'
import { toneForType } from '@/lib/tones'
import { api, CryptApiError } from '@/lib/api'
import { typeIcon } from '@/lib/icons'
import type { RecordSummary } from '@/lib/types'
import { useLibrary } from '@/stores/library'

const lib = useLibrary()
const items = ref<RecordSummary[]>([])
const confirmId = ref<string | null>(null)
const confirmAll = ref(false)
const fmt = new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })

async function load() {
  items.value = await api.listRecords({ trashed: true })
}
watch(() => lib.version, load, { immediate: true })

const emptyAll = () => run(() => api.emptyTrash(), 'Корзина очищена')
const leaving = ref<Record<string, 'restore' | 'purge'>>({})
const preview = ref<RecordSummary | null>(null)
const previewOpen = ref(false)
function openPreview(r: RecordSummary) {
  preview.value = r
  previewOpen.value = true
}
function restoreFromPreview(r: RecordSummary) {
  previewOpen.value = false
  restore(r)
}
const folderName = (id: string | null) => (id ? lib.folders.find((f) => f.id === id)?.name : null)
async function restore(r: RecordSummary) {
  leaving.value = { ...leaving.value, [r.id]: 'restore' }
  const where = folderName(r.folder)
  await new Promise((res) => setTimeout(res, 320))
  await run(() => api.restoreRecord(r.id), where ? `Восстановлено в «${where}»` : 'Запись восстановлена')
}
async function purge(id: string) {
  leaving.value = { ...leaving.value, [id]: 'purge' }
  await new Promise((res) => setTimeout(res, 320))
  await run(() => api.purgeRecord(id), 'Удалено навсегда')
}

async function run(fn: () => Promise<unknown>, msg: string) {
  try {
    await fn()
    await lib.changed()
    toast(msg)
  } catch (e) {
    toast.error(e instanceof CryptApiError ? e.message : String(e))
  } finally {
    confirmId.value = null
    confirmAll.value = false
    leaving.value = {}
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <header data-tauri-drag-region class="flex h-16 shrink-0 items-center gap-3 pr-6 pl-9">
      <h1 class="text-[22px] font-semibold tracking-[-.02em]">Корзина</h1>
      <span class="text-muted-foreground text-[13px]">{{ items.length || '' }}</span>
      <span class="flex-1" />
      <Button
        v-if="items.length"
        variant="outline"
        size="sm"
        class="text-destructive border-destructive/30"
        @click="confirmAll = true"
        >Очистить корзину</Button
      >
    </header>
    <div
      v-if="confirmAll && items.length"
      role="alert"
      class="anim-pop bg-destructive/10 text-destructive mx-6 mb-3 ml-9 flex items-center gap-3 rounded-xl px-4 py-3"
    >
      <TriangleAlert class="size-[18px]" />
      <span class="flex-1 text-[13px] font-medium">Удалить всё навсегда? Восстановить будет нельзя.</span>
      <Button variant="ghost" size="sm" @click="confirmAll = false">Отмена</Button>
      <Button size="sm" class="bg-destructive text-white hover:bg-destructive/90" @click="emptyAll">Удалить</Button>
    </div>
    <div class="flex-1 overflow-auto pr-6 pb-8 pl-9">
      <TransitionGroup v-if="items.length" tag="div" name="trash" class="bg-card overflow-hidden rounded-xl border">
        <div
          v-for="r in items"
          :key="r.id"
          class="group/t hover:bg-foreground/[.03] flex items-center gap-3.5 border-t py-3 pr-3 pl-4 transition-[opacity,transform] duration-300 first:border-t-0"
          :class="
            leaving[r.id] === 'restore'
              ? 'translate-x-10 opacity-0'
              : leaving[r.id] === 'purge'
                ? 'scale-[.96] opacity-0'
                : ''
          "
        >
          <KindTile
            :icon="typeIcon(lib.typeById.get(r.type_id ?? '')?.icon)"
            :tone="r.secret_kind ? 'secret' : toneForType(lib.types, r.type_id)"
            size="lg"
            class="size-7! rounded-[7px]! [&_svg]:size-[15px]!"
          />
          <button
            type="button"
            class="focus-visible:ring-ring/50 flex min-w-0 flex-1 cursor-pointer flex-col justify-center gap-0.5 self-stretch rounded-md text-left outline-none focus-visible:ring-3"
            :title="`Посмотреть «${r.title || 'Без названия'}»`"
            @click="openPreview(r)"
          >
            <span class="truncate text-[14px] font-medium">{{ r.title || 'Без названия' }}</span>
            <span class="text-muted-foreground text-xs"
              >{{ folderName(r.folder) ? `Из «${folderName(r.folder)}» · ` : '' }}удалено
              {{ fmt.format(r.updated_at) }}</span
            >
          </button>
          <span
            v-if="confirmId !== r.id"
            class="flex gap-1.5 opacity-0 transition-opacity group-hover/t:opacity-100 focus-within:opacity-100"
          >
            <Button variant="ghost" size="sm" @click="openPreview(r)"><Eye class="size-3.5" /> Посмотреть</Button>
            <Button variant="outline" size="sm" @click="restore(r)"><RotateCcw class="size-3.5" /> Восстановить</Button>
            <Button variant="ghost" size="sm" class="text-destructive" @click="confirmId = r.id"
              >Удалить навсегда</Button
            >
          </span>
          <span v-else class="anim-pop flex items-center gap-1.5">
            <span class="text-muted-foreground text-xs">Точно?</span>
            <Button variant="outline" size="sm" @click="confirmId = null">Нет</Button>
            <Button size="sm" class="bg-destructive text-white hover:bg-destructive/90" @click="purge(r.id)"
              >Удалить</Button
            >
          </span>
        </div>
      </TransitionGroup>
      <TrashPreview v-model:open="previewOpen" :record="preview" @restore="restoreFromPreview" />
      <div v-if="!items.length" class="anim-in flex h-[480px] flex-col items-center justify-center gap-3.5 text-center">
        <svg width="96" height="96" viewBox="0 0 96 96" aria-hidden="true">
          <circle cx="48" cy="48" r="44" class="fill-primary-soft" />
          <rect x="30" y="38" width="36" height="34" rx="6" fill="none" class="stroke-primary" stroke-width="3" />
          <path d="M40 46v18M48 46v18M56 46v18" class="stroke-primary" stroke-width="3" stroke-linecap="round" />
          <g class="trash-lid">
            <path d="M26 32h44" class="stroke-primary" stroke-width="3" stroke-linecap="round" />
            <path d="M41 32v-5h14v5" fill="none" class="stroke-primary" stroke-width="3" stroke-linejoin="round" />
          </g>
        </svg>
        <span class="text-[17px] font-semibold">Корзина пуста</span>
        <span class="text-muted-foreground max-w-[320px] text-[13px] leading-relaxed"
          >Удалённые записи попадают сюда, и их можно восстановить, пока вы не очистите корзину.</span
        >
      </div>
    </div>
  </div>
</template>

<style scoped>
.trash-leave-active {
  transition: opacity 0.2s ease;
}
.trash-leave-to {
  opacity: 0;
}
.trash-lid {
  transform-box: fill-box;
  transform-origin: left bottom;
  animation: lid 1.6s ease-in-out 0.4s 1;
}
@keyframes lid {
  0%,
  70%,
  100% {
    transform: rotate(0);
  }
  80% {
    transform: rotate(-14deg);
  }
  90% {
    transform: rotate(0);
  }
}
@media (prefers-reduced-motion: reduce) {
  .trash-lid {
    animation: none;
  }
}
</style>
