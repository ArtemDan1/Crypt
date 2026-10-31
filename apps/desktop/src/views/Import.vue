<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open } from '@tauri-apps/plugin-dialog'
import { Upload, ArrowLeft, Check } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import SegmentedControl from '@/components/crypt/SegmentedControl.vue'
import LegacyImport from '@/components/LegacyImport.vue'
import SshImportDialog from '@/components/secret/SshImportDialog.vue'
import { IMPORT_SOURCES, defaultImportFolder, type ImportSource } from '@/lib/importSources'
import { api, CryptApiError } from '@/lib/api'
import type { BrowserPreview, OtpPreview, OtpSource, SshImportResult } from '@/lib/types'
import { useLibrary } from '@/stores/library'
import { useSession } from '@/stores/session'
const lib = useLibrary(),
  session = useSession()
const selected = ref<ImportSource | null>(null),
  step = ref<'source' | 'instruction' | 'preview' | 'done'>('source')
const busy = ref(false),
  error = ref(''),
  paths = ref<string[]>([]),
  password = ref(''),
  needsPassword = ref(false),
  folder = ref(''),
  sshOpen = ref(false)
const csv = ref<BrowserPreview | null>(null),
  otp = ref<OtpPreview | null>(null),
  summary = ref(''),
  details = ref<string[]>([]),
  trashed = ref(false)
const groups = ['Пароли из браузера', 'Коды 2FA', 'Другие менеджеры'] as const
const browsers = IMPORT_SOURCES.filter((s) => s.group === groups[0]).map((s) => ({ value: s.id, label: s.name }))
const isBrowser = computed(() => selected.value?.group === groups[0])
const total = computed(() => csv.value?.new ?? otp.value?.rows.length ?? 0)
let active = true,
  request = 0,
  off: (() => void) | undefined
function clear() {
  ++request
  password.value = ''
  needsPassword.value = false
  csv.value = null
  otp.value = null
  paths.value = []
  error.value = ''
  summary.value = ''
  details.value = []
  trashed.value = false
  busy.value = false
}
let cancellation = Promise.resolve()
function cancel() {
  clear()
  step.value = 'source'
  selected.value = null
  cancellation = api.cancelOtpImport().catch(() => {})
}
function choose(source: ImportSource) {
  if (busy.value) return
  clear()
  cancellation = api.cancelOtpImport().catch(() => {})
  selected.value = source
  step.value = 'instruction'
  folder.value = defaultImportFolder(source.group === groups[0] ? 'browser' : 'otp', new Date())
}
function tab(id: string) {
  const source = IMPORT_SOURCES.find((s) => s.id === id)
  if (source) choose(source)
}
const reasons: Record<string, string> = {
  NoPassword: 'Нет пароля',
  BadRow: 'Повреждённая строка',
  BadTotp: 'Код 2FA не распознан; логин сохранён',
  Hotp: 'HOTP не поддерживается',
  BadSecret: 'Некорректный ключ',
  Unsupported: 'Неподдерживаемый формат или алгоритм',
}
async function load(files: string[]) {
  if (busy.value || !selected.value || !files.length) return
  const source = selected.value
  if (source.id !== 'ga' && files.length !== 1) {
    error.value = 'Выберите один файл для этого источника'
    return
  }
  const seq = ++request
  paths.value = files
  busy.value = true
  error.value = ''
  csv.value = null
  otp.value = null
  trashed.value = false
  try {
    await cancellation
    if (!active || seq !== request) return
    if (isBrowser.value) {
      const p = await api.previewBrowserCsv(files[0]!)
      if (active && seq === request) csv.value = p
    } else {
      const input: OtpSource =
        source.id === 'ga'
          ? { kind: 'images', paths: files }
          : source.id === 'aegis'
            ? { kind: 'aegis', path: files[0]!, password: password.value || undefined }
            : { kind: 'twofas', path: files[0]! }
      const p = await api.previewOtp(input)
      if (active && seq === request) otp.value = p
    }
    if (active && seq === request) {
      needsPassword.value = false
      step.value = 'preview'
    }
  } catch (e) {
    if (active && seq === request) {
      if (e instanceof CryptApiError && ['NeedsPassword', 'ImportPassword'].includes(e.code)) needsPassword.value = true
      error.value = e instanceof Error ? e.message : String(e)
    }
  } finally {
    if (active && seq === request) {
      password.value = ''
      busy.value = false
    }
  }
}
async function pick() {
  if (!selected.value || busy.value) return
  try {
    const extensions = isBrowser.value
      ? ['csv']
      : selected.value.id === 'ga'
        ? ['png', 'jpg', 'jpeg']
        : selected.value.id === 'aegis'
          ? ['json']
          : ['2fas', 'json']
    const result = isTauri()
      ? await open({ multiple: selected.value.id === 'ga', filters: [{ name: selected.value.name, extensions }] })
      : `/demo/import.${extensions[0]}`
    if (result) await load(Array.isArray(result) ? result : [result])
  } catch (e) {
    error.value = String(e)
  }
}
async function run() {
  if (busy.value || !folder.value.trim() || !total.value) return
  const seq = request
  busy.value = true
  error.value = ''
  try {
    if (csv.value) {
      const r = await api.importBrowserCsv(paths.value[0]!, folder.value.trim())
      if (!active || seq !== request) return
      summary.value = `Импортировано логинов: ${r.report.imported}`
      details.value = [
        `Дублей пропущено: ${r.report.duplicates}`,
        ...r.report.skipped.map(([line, reason]) => `Строка ${line}: ${reasons[reason]}`),
        `Снимок для отката: ${r.snapshot}`,
      ]
    } else if (otp.value) {
      const r = await api.importOtp(otp.value.token, folder.value.trim())
      if (!active || seq !== request) return
      summary.value = `Добавлено к логинам: ${r.report.attached} · новых секретов: ${r.report.created}`
      details.value = [
        ...r.report.skipped.map(([name, reason]) => `${name}: ${reasons[reason]}`),
        ...r.report.missing_batches.map(([i, n]) => `Не хватает QR ${i} из ${n}`),
        `Снимок для отката: ${r.snapshot}`,
      ]
    }
    await lib.changed()
    if (active && seq === request) {
      otp.value = null
      csv.value = null
      step.value = 'done'
    }
  } catch (e) {
    if (active && seq === request) error.value = e instanceof Error ? e.message : String(e)
  } finally {
    if (active && seq === request) busy.value = false
  }
}
async function trash() {
  busy.value = true
  error.value = ''
  try {
    for (const path of paths.value) await api.trashFile(path)
    trashed.value = true
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    busy.value = false
  }
}
function legacyDone(report: { message: string; details: string[] }) {
  summary.value = report.message
  details.value = report.details
  step.value = 'done'
}
function sshDone(result: SshImportResult[]) {
  sshOpen.value = false
  summary.value = `Импортировано ключей: ${result.filter((r) => r.recordId).length}`
  details.value = result.filter((r) => r.error).map((r) => `${r.path.split('/').pop()}: ${r.error!.message}`)
  step.value = 'done'
}
const unhook = session.beforeLock(cancel)
onMounted(async () => {
  if (isTauri())
    try {
      const unlisten = await getCurrentWebview().onDragDropEvent((e) => {
        if (
          e.payload.type === 'drop' &&
          selected.value &&
          selected.value.group !== 'Другие менеджеры' &&
          step.value !== 'done'
        )
          void load(e.payload.paths)
      })
      if (active) off = unlisten
      else unlisten()
    } catch (e) {
      error.value = String(e)
    }
})
onBeforeUnmount(() => {
  active = false
  clear()
  unhook()
  off?.()
  cancellation = api.cancelOtpImport().catch(() => {})
})
</script>
<template>
  <div class="flex h-full flex-col">
    <header data-tauri-drag-region class="flex h-[52px] shrink-0 items-center border-b px-7">
      <h1 class="text-[17px] font-semibold">Импорт</h1>
      <span class="ml-auto text-xs text-muted-foreground">Всё обрабатывается на этом Mac</span>
    </header>
    <main class="mx-auto w-full max-w-[860px] flex-1 space-y-6 overflow-auto px-7 py-7 anim-in">
      <template v-if="step === 'source'"
        ><div>
          <h2 class="text-2xl font-semibold tracking-tight">Перенесите секреты в Crypt</h2>
          <p class="mt-2 text-sm text-muted-foreground">
            Выберите источник. Перед импортом браузеров и кодов 2FA проверим, что будет добавлено.
          </p>
        </div>
        <section v-for="group in groups" :key="group" class="space-y-3">
          <h3 class="text-sm font-medium text-foreground-2">{{ group }}</h3>
          <div class="grid gap-3 sm:grid-cols-3">
            <button
              v-for="source in IMPORT_SOURCES.filter((s) => s.group === group)"
              :key="source.id"
              class="rounded-xl border bg-card p-4 text-left transition-colors hover:border-primary hover:bg-primary-soft"
              @click="choose(source)"
            >
              <span class="block text-sm font-semibold">{{ source.name }}</span
              ><span class="mt-2 block text-xs text-muted-foreground">{{ source.format }}</span>
            </button>
          </div>
        </section>
      </template>
      <template v-else-if="selected">
        <button
          :disabled="busy"
          class="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
          @click="cancel"
        >
          <ArrowLeft class="size-4" />Другой источник
        </button>
        <div>
          <p class="mb-2 text-xs text-muted-foreground">
            {{ step === 'done' ? 'Готово' : step === 'preview' ? 'Предпросмотр' : 'Инструкция и файл' }}
          </p>
          <h2 class="text-2xl font-semibold">{{ selected.name }}</h2>
        </div>
        <template v-if="step === 'instruction'">
          <SegmentedControl
            v-if="isBrowser"
            :model-value="selected.id"
            :options="browsers"
            label="Браузер"
            @update:model-value="tab"
          />
          <ol class="space-y-3 rounded-xl border bg-card p-5">
            <li v-for="(instruction, i) in selected.steps" :key="i" class="flex gap-3 text-sm leading-relaxed">
              <span
                class="flex size-6 shrink-0 items-center justify-center rounded-full bg-primary-soft text-xs font-semibold text-primary-soft-fg"
                >{{ i + 1 }}</span
              >{{ instruction }}
            </li>
          </ol>
          <LegacyImport
            v-if="selected.id === 'keepass' || selected.id === 'markdown'"
            :source="selected.id"
            @imported="legacyDone"
          />
          <Button v-else-if="selected.id === 'ssh'" @click="sshOpen = true">Выбрать ключи из ~/.ssh</Button>
          <div v-else class="space-y-4 rounded-xl border border-dashed bg-card p-6">
            <Button :disabled="busy" variant="outline" @click="pick"
              ><Upload class="size-4" />{{ busy ? 'Проверяем…' : 'Выбрать файл…' }}</Button
            >
            <p class="text-sm text-muted-foreground">
              Или перетащите {{ selected.id === 'ga' ? 'все изображения QR-кодов' : 'файл' }} в окно ·
              {{ selected.format }}
            </p>
            <p v-if="paths.length" class="truncate text-xs text-muted-foreground">
              {{ paths.map((p) => p.split('/').pop()).join(', ') }}
            </p>
            <form v-if="needsPassword" class="flex gap-2" @submit.prevent="load(paths)">
              <Input
                v-model="password"
                type="password"
                autocomplete="off"
                aria-label="Пароль экспорта Aegis"
                placeholder="Пароль экспорта Aegis"
              /><Button :disabled="busy || !password" type="submit">Проверить</Button>
            </form>
          </div>
        </template>
        <template v-if="step === 'preview'">
          <div class="grid grid-cols-3 gap-3">
            <div class="rounded-xl border bg-card p-4">
              <strong class="text-2xl text-primary">{{ total }}</strong>
              <p class="mt-1 text-xs text-muted-foreground">Будет добавлено</p>
            </div>
            <div class="rounded-xl border bg-card p-4">
              <strong class="text-2xl">{{ csv?.duplicates ?? 0 }}</strong>
              <p class="mt-1 text-xs text-muted-foreground">Дублей</p>
            </div>
            <div class="rounded-xl border bg-card p-4">
              <strong class="text-2xl">{{ csv?.skipped ?? otp?.skipped.length ?? 0 }}</strong>
              <p class="mt-1 text-xs text-muted-foreground">Замечаний</p>
            </div>
          </div>
          <div class="divide-y rounded-xl border bg-card">
            <div
              v-for="([site, login], i) in csv?.sample ?? []"
              :key="i"
              class="flex items-center gap-4 px-4 py-3 text-sm"
            >
              <span class="min-w-0 flex-1 truncate"
                >{{ site }}<span class="mt-1 block text-xs text-muted-foreground">{{ login }}</span></span
              ><span class="font-mono text-secret-fg">••••••••</span>
            </div>
            <div v-for="(row, i) in otp?.rows ?? []" :key="i" class="flex items-center gap-4 px-4 py-3 text-sm">
              <span class="min-w-0 flex-1 truncate"
                >{{ row.issuer }}<span class="mt-1 block text-xs text-muted-foreground">{{ row.account }}</span></span
              ><span class="text-xs text-primary">{{
                row.target === 'new' ? 'Новый секрет' : `К логину «${row.target.attach}»`
              }}</span>
            </div>
          </div>
          <ul v-if="csv?.skippedDetails?.length" class="space-y-1 text-sm text-muted-foreground">
            <li v-for="([line, reason], i) in csv.skippedDetails" :key="i">Строка {{ line }}: {{ reasons[reason] }}</li>
          </ul>
          <ul v-if="otp" class="space-y-1 text-sm text-muted-foreground">
            <li v-for="([name, reason], i) in otp.skipped" :key="i">{{ name }}: {{ reasons[reason] }}</li>
            <li v-for="([index, size], i) in otp.missingBatches" :key="`batch-${i}`" class="text-secret-fg">
              Не хватает QR {{ index }} из {{ size }}
            </li>
          </ul>
          <label class="block space-y-2 text-sm"
            >Папка назначения<Input
              v-model="folder"
              list="import-folders"
              :disabled="busy"
              aria-label="Папка назначения" /><datalist id="import-folders">
              <option v-for="f in lib.folders.filter((f) => !f.parent)" :key="f.id" :value="f.name" /></datalist
          ></label>
          <p class="text-xs text-muted-foreground">Перед записью создадим снимок для отката.</p>
          <Button :disabled="busy || !total || !folder.trim()" @click="run">{{
            busy ? 'Импортируем…' : `Импортировать (${total})`
          }}</Button>
        </template>
        <template v-if="step === 'done'"
          ><div class="rounded-xl border bg-card p-5">
            <Check class="mb-3 size-8 text-primary" />
            <h3 class="font-semibold">{{ summary }}</h3>
            <ul class="mt-3 space-y-1 text-sm text-muted-foreground">
              <li v-for="(detail, i) in details" :key="i">{{ detail }}</li>
            </ul>
          </div>
          <div
            v-if="paths.length && (isBrowser || selected.id === 'ga')"
            class="space-y-3 rounded-xl border border-border-strong bg-secret-bg p-4 text-sm text-secret-fg"
          >
            <p>
              {{ isBrowser ? 'Файл хранит пароли открытым текстом' : 'Скриншоты QR содержат ключи 2FA' }}. После
              проверки импорта переместите {{ isBrowser ? 'файл' : 'изображения' }} в системную Корзину.
            </p>
            <Button variant="outline" :disabled="busy || trashed" @click="trash">{{
              trashed ? 'Перемещено в Корзину' : 'Переместить в корзину'
            }}</Button>
          </div>
          <Button variant="outline" @click="cancel">Импортировать ещё</Button>
        </template>
      </template>
      <p v-if="error" role="alert" class="text-sm text-destructive">{{ error }}</p>
    </main>
    <SshImportDialog v-model:open="sshOpen" @imported="sshDone" />
  </div>
</template>
