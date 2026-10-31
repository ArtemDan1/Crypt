<script setup lang="ts">
import { ref, onBeforeUnmount } from 'vue'
import { useSession } from '@/stores/session'
import { open } from '@tauri-apps/plugin-dialog'
import { FileKey, FolderInput, Upload } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { api, CryptApiError } from '@/lib/api'
import { useLibrary } from '@/stores/library'

defineProps<{ source: 'keepass' | 'markdown' }>()
const emit = defineEmits<{ imported: [report: { message: string; details: string[] }] }>()
const lib = useLibrary()
const busy = ref(false)

// KeePass: file → password (and optional key file) → import.
const kdbx = ref<string | null>(null)
const keyfile = ref<string | null>(null)
const password = ref('')
const kdbxError = ref('')
const clear = () => {
  password.value = ''
  kdbx.value = null
  keyfile.value = null
}
const unhook = useSession().beforeLock(clear)
onBeforeUnmount(() => {
  clear()
  unhook()
})

const base = (p: string) => p.split(/[\\/]/).pop() ?? p
function fail(e: unknown) {
  toast.error(e instanceof CryptApiError ? e.message : String(e))
}
async function pickKdbx() {
  const path = await open({ multiple: false, filters: [{ name: 'KeePass', extensions: ['kdbx', 'kdb'] }] })
  if (typeof path === 'string') {
    kdbx.value = path
    keyfile.value = null
    password.value = ''
    kdbxError.value = ''
  }
}
async function pickKeyfile() {
  const path = await open({ multiple: false })
  if (typeof path === 'string') keyfile.value = path
}
function cancelKdbx() {
  kdbx.value = null
  keyfile.value = null
  password.value = ''
  kdbxError.value = ''
}
async function importKdbx() {
  if (!kdbx.value || busy.value) return
  busy.value = true
  kdbxError.value = ''
  try {
    const { report: r, snapshot } = await api.importKeepass(kdbx.value, password.value || null, keyfile.value)
    password.value = ''
    await lib.changed()
    const extra = [
      r.extra_secrets && `доп. секретов: ${r.extra_secrets}`,
      r.skipped_recycled && `из корзины KeePass пропущено: ${r.skipped_recycled}`,
      r.skipped_attachments && `вложений не перенесено: ${r.skipped_attachments}`,
      r.invalid_totp && `нераспознанных TOTP: ${r.invalid_totp}`,
    ].filter(Boolean)
    toast(`Импортировано записей: ${r.imported}`, {
      description: [...extra, `откатить можно к снимку «${snapshot}»`].join(' · '),
    })
    cancelKdbx()
    emit('imported', { message: `Импортировано записей: ${r.imported}`, details: extra.map(String) })
  } catch (e) {
    password.value = ''
    if (e instanceof CryptApiError && e.code === 'WrongImportKey') kdbxError.value = e.message
    else fail(e)
  } finally {
    busy.value = false
  }
}
async function importFolder() {
  const path = await open({ directory: true, multiple: false })
  if (typeof path !== 'string') return
  busy.value = true
  try {
    const { report: r, snapshot } = await api.importMarkdown(path)
    await lib.changed()
    const extra = [
      r.typed && `с типом: ${r.typed}`,
      r.links && `ссылок: ${r.links}`,
      r.skipped_files && `других файлов пропущено: ${r.skipped_files}`,
    ].filter(Boolean)
    toast(`Импортировано заметок: ${r.notes} в папку «${base(path)}»`, {
      description: [...extra, `откатить можно к снимку «${snapshot}»`].join(' · '),
    })
    emit('imported', { message: `Импортировано заметок: ${r.notes}`, details: extra.map(String) })
  } catch (e) {
    fail(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="divide-y rounded-xl border">
    <div v-if="source === 'keepass'" class="space-y-3 px-4 py-3.5">
      <div class="flex items-center gap-3">
        <FileKey class="text-secret size-5" />
        <div class="flex-1">
          <div class="font-medium">KeePass</div>
          <div class="text-muted-foreground text-xs">
            Записи .kdbx станут секретами «Логин»; группы — тегами. Вложения не переносятся.
          </div>
        </div>
        <Button v-if="!kdbx" variant="outline" size="sm" :disabled="busy" @click="pickKdbx"
          ><Upload class="size-3.5" />Выбрать файл…</Button
        >
      </div>
      <form v-if="kdbx" class="space-y-2.5 rounded-lg border p-3" @submit.prevent="importKdbx">
        <div class="truncate font-mono text-xs" :title="kdbx">{{ base(kdbx) }}</div>
        <Input
          v-model="password"
          type="password"
          placeholder="Мастер-пароль"
          aria-label="Мастер-пароль KeePass"
          autocomplete="off"
          autofocus
        />
        <div class="flex items-center gap-2 text-xs">
          <span class="text-muted-foreground flex-1 truncate">{{
            keyfile ? `Ключ-файл: ${base(keyfile)}` : 'Без ключ-файла'
          }}</span>
          <button type="button" class="text-muted-foreground hover:text-foreground" @click="pickKeyfile">
            {{ keyfile ? 'Другой…' : 'Добавить ключ-файл…' }}
          </button>
          <button
            v-if="keyfile"
            type="button"
            class="text-muted-foreground hover:text-foreground"
            @click="keyfile = null"
          >
            Убрать
          </button>
        </div>
        <p v-if="kdbxError" class="text-destructive text-[13px]">{{ kdbxError }}</p>
        <div class="flex justify-end gap-2">
          <Button type="button" variant="ghost" size="sm" @click="cancelKdbx">Отмена</Button>
          <Button size="sm" :disabled="busy || (!password && !keyfile)">{{
            busy ? 'Импорт…' : 'Импортировать'
          }}</Button>
        </div>
      </form>
    </div>

    <div v-if="source === 'markdown'" class="flex items-center gap-3 px-4 py-3.5">
      <FolderInput class="text-muted-foreground size-5" />
      <div class="flex-1">
        <div class="font-medium">Папка Obsidian или Markdown</div>
        <div class="text-muted-foreground text-xs">
          Подпапки станут папками, фронтматтер — тегами и полями подходящего типа, [[ссылки]] — связями.
        </div>
      </div>
      <Button variant="outline" size="sm" :disabled="busy" @click="importFolder"
        ><Upload class="size-3.5" />Выбрать папку…</Button
      >
    </div>
  </section>
</template>
