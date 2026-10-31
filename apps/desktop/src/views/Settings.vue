<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useLibrary } from '@/stores/library'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { Fingerprint, FolderOpen, KeyRound, Lock, Timer } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import SegmentedControl from '@/components/crypt/SegmentedControl.vue'
import { getThemePref, setThemePref, type ThemePref } from '@/lib/theme'
import { useRoute } from 'vue-router'
import SnapshotsSection from '@/components/SnapshotsSection.vue'
import BackupCard from '@/components/BackupCard.vue'
import TransferSection from '@/components/TransferSection.vue'
import { api, CryptApiError } from '@/lib/api'
import type { UnlockMethods, Settings as SettingsDTO, AgentStatus, SshConfigStatus } from '@/lib/types'
import { useSession } from '@/stores/session'

const session = useSession()
const route = useRoute()
type Section = 'security' | 'ssh' | 'vault' | 'danger'
const SECTIONS: { id: Section; label: string }[] = [
  { id: 'security', label: 'Безопасность' },
  { id: 'ssh', label: 'Секреты и SSH' },
  { id: 'vault', label: 'Хранилище' },
  { id: 'danger', label: 'Опасная зона' },
]
const readSection = (): Section => {
  const q = String(route.query.section ?? '')
  if (SECTIONS.some((x) => x.id === q)) return q as Section
  try {
    const v = localStorage.getItem('crypt.settings.section')
    if (SECTIONS.some((x) => x.id === v)) return v as Section
  } catch {}
  return 'security'
}
const section = ref<Section>(readSection())
function pick(id: Section) {
  section.value = id
  try {
    localStorage.setItem('crypt.settings.section', id)
  } catch {}
}
const markTop = computed(() => SECTIONS.findIndex((x) => x.id === section.value) * 38)
const theme = ref<ThemePref>(getThemePref())
watch(theme, (t) => setThemePref(t))
const themeOptions = [
  { value: 'system' as const, label: 'Система' },
  { value: 'light' as const, label: 'Светлая' },
  { value: 'dark' as const, label: 'Тёмная' },
]
const autolockOptions = [1, 5, 15, 30, 60].map((m) => ({ value: m, label: m === 60 ? '1 час' : `${m} мин` }))
const router = useRouter()
const lib = useLibrary()
const deleteOpen = ref(false)
// Bumped after an import so the snapshot list shows the new "before import" snapshot.
const snapshotsKey = ref(0)
const deleteConfirmation = ref('')
const deletePin = ref('')
const deleting = ref(false)
function resetDeletion(open: boolean) {
  if (deleting.value) return
  deleteOpen.value = open
  deleteConfirmation.value = ''
  deletePin.value = ''
}
const confirmShake = ref(false)
function tryDelete() {
  if (deleteConfirmation.value === 'УДАЛИТЬ') return deleteVault()
  confirmShake.value = false
  requestAnimationFrame(() => (confirmShake.value = true))
  setTimeout(() => (confirmShake.value = false), 500)
}
async function deleteVault() {
  if (deleteConfirmation.value !== 'УДАЛИТЬ' || deleting.value) return
  deleting.value = true
  try {
    await session.flushAll()
    const status = await api.deleteVault(deleteConfirmation.value, deletePin.value || undefined)
    hideSeed()
    deletePin.value = ''
    lib.clear()
    session.status = status
    session.methods = null
    await router.replace('/onboarding')
  } catch (e) {
    fail(e)
    await session.refresh()
    await refresh()
  } finally {
    deleting.value = false
  }
}
const methods = ref<UnlockMethods | null>(null)
const autolock = ref(15)
const path = ref('')
const pinOpen = ref(false)
const pin = ref('')
const pin2 = ref('')
const seed = ref<string[] | null>(null)
const seedPin = ref('')
const seedLeft = ref(0)
let seedTimer: ReturnType<typeof setInterval> | null = null

function fail(e: unknown) {
  toast.error(e instanceof CryptApiError ? e.message : String(e))
}
async function refresh() {
  methods.value = await api.unlockMethods().catch(() => null)
}
onMounted(async () => {
  await refresh()
  autolock.value = (await api.getSettings()).autolockMinutes
  path.value = await api.vaultPath()
})
onBeforeUnmount(hideSeed)

async function setAutolock(value: unknown) {
  const v = Number(value)
  try {
    autolock.value = (await api.setSettings({ ...(await api.getSettings()), autolockMinutes: v })).autolockMinutes
    session.applyAutolock(autolock.value)
  } catch (err) {
    fail(err)
  }
}
async function toggleBio(on: boolean) {
  try {
    if (on) await api.enableBiometric()
    else await api.disableBiometric()
  } catch (e) {
    fail(e)
  }
  await refresh()
}
async function savePin() {
  if (!/^\d{6,12}$/.test(pin.value) || pin.value !== pin2.value) {
    toast.error('PIN — от 6 до 12 цифр, оба поля должны совпадать')
    return
  }
  try {
    await api.setPin(pin.value)
    toast('PIN сохранён')
    pinOpen.value = false
    pin.value = pin2.value = ''
  } catch (e) {
    fail(e)
  }
  await refresh()
}
async function removePin() {
  try {
    await api.removePin()
    toast('PIN удалён')
  } catch (e) {
    fail(e)
  }
  await refresh()
}
async function showSeed() {
  try {
    seed.value = await api.revealSeed(seedPin.value || undefined)
    seedPin.value = ''
    seedLeft.value = 60
    seedTimer = setInterval(() => {
      if (--seedLeft.value <= 0) hideSeed()
    }, 1000)
  } catch (e) {
    if (!(e instanceof CryptApiError && e.code === 'AuthCancelled')) fail(e)
  }
}
function hideSeed() {
  seed.value = null
  if (seedTimer) clearInterval(seedTimer)
  seedTimer = null
}
async function reveal() {
  try {
    await revealItemInDir(path.value)
  } catch (e) {
    fail(e)
  }
}
const secretSettings = ref<SettingsDTO | null>(null),
  agent = ref<AgentStatus | null>(null),
  sshConfig = ref<SshConfigStatus | null>(null),
  settingsBusy = ref(false)
async function loadAgent() {
  try {
    const [s, a, c] = await Promise.all([api.getSettings(), api.agentStatus(), api.sshConfigStatus()])
    secretSettings.value = s
    agent.value = a
    sshConfig.value = c
  } catch (e) {
    fail(e)
  }
}
onMounted(loadAgent)
async function changeSecretSettings(patch: Partial<SettingsDTO>) {
  if (settingsBusy.value) return
  settingsBusy.value = true
  try {
    secretSettings.value = await api.setSettings({ ...(await api.getSettings()), ...patch })
    await loadAgent()
  } catch (e) {
    fail(e)
    await loadAgent()
  } finally {
    settingsBusy.value = false
  }
}
async function configure(on: boolean) {
  if (settingsBusy.value) return
  settingsBusy.value = true
  try {
    if (on) await api.configureSshConfig()
    else await api.unconfigureSshConfig()
    await loadAgent()
    toast(on ? 'Блок Crypt добавлен в ~/.ssh/config' : 'Блок Crypt удалён')
  } catch (e) {
    fail(e)
  } finally {
    settingsBusy.value = false
  }
}
async function copyExport() {
  if (!sshConfig.value) return
  try {
    await navigator.clipboard.writeText(sshConfig.value.exportLine)
    toast('Команда скопирована')
  } catch (e) {
    fail(e)
  }
}
</script>

<template>
  <div class="flex h-full">
    <nav aria-label="Разделы настроек" class="relative flex w-[220px] shrink-0 flex-col gap-0.5 px-3 py-8">
      <h1 data-tauri-drag-region class="mb-3.5 ml-3 text-[22px] font-semibold tracking-[-.02em]">Настройки</h1>
      <span
        aria-hidden="true"
        class="bg-primary absolute left-3 h-[22px] w-[3px] rounded-full transition-[top] duration-350 ease-[cubic-bezier(.3,1.2,.5,1)]"
        :style="{ top: `${81 + markTop}px` }"
      />
      <button
        v-for="x in SECTIONS"
        :key="x.id"
        class="flex h-9 items-center rounded-lg pr-3 pl-[18px] text-left text-[14px] transition-colors"
        :class="[
          section === x.id ? 'bg-accent font-semibold' : 'hover:bg-foreground/[.05] font-medium',
          x.id === 'danger' && 'text-destructive',
        ]"
        :aria-current="section === x.id ? 'page' : undefined"
        @click="pick(x.id)"
      >
        {{ x.label }}
      </button>
    </nav>
    <div class="flex-1 overflow-auto py-9 pr-12 pl-6">
      <div :key="section" class="anim-in flex max-w-[680px] flex-col gap-[18px]">
        <h2 class="text-[20px] font-semibold tracking-[-.01em]">{{ SECTIONS.find((x) => x.id === section)!.label }}</h2>
        <section v-if="section === 'security'" class="bg-card flex flex-col gap-3 rounded-[14px] border px-[18px] py-4">
          <span class="flex flex-col gap-0.5"
            ><span class="text-[14px] font-medium">Тема</span
            ><span class="text-muted-foreground text-xs">«Система» следует настройке macOS</span></span
          >
          <SegmentedControl v-model="theme" :options="themeOptions" label="Тема" class="self-start" />
        </section>
        <section v-if="section === 'security'" class="bg-card divide-y rounded-[14px] border">
          <div class="flex items-center gap-3 px-4 py-3.5">
            <Timer class="text-muted-foreground size-5" />
            <div class="flex-1">
              <div class="font-medium">Автоблокировка</div>
              <div class="text-muted-foreground text-xs">Также при блокировке экрана и сне Mac</div>
            </div>
            <SegmentedControl
              :model-value="autolock"
              :options="autolockOptions"
              label="Автоблокировка"
              size="sm"
              @update:model-value="setAutolock"
            />
          </div>
          <div class="flex items-center gap-3 px-4 py-3.5">
            <Fingerprint class="text-muted-foreground size-5" />
            <div class="flex-1">
              <div class="font-medium">Touch ID</div>
              <div class="text-muted-foreground text-xs">
                {{
                  methods?.biometricAvailable ? 'Или пароль Mac, если Touch ID недоступен' : 'Недоступно на этом Mac'
                }}
              </div>
            </div>
            <Switch
              :model-value="!!methods?.biometric"
              :disabled="!methods?.biometricAvailable"
              aria-label="Touch ID"
              @update:model-value="toggleBio"
            />
          </div>
          <div class="px-4 py-3.5">
            <div class="flex items-center gap-3">
              <Lock class="text-muted-foreground size-5" />
              <div class="flex-1">
                <div class="font-medium">PIN-код</div>
                <div class="text-muted-foreground text-xs">
                  {{ methods?.pin ? 'Задан' : 'Не задан' }} · после 5 ошибок отключается
                </div>
              </div>
              <Button variant="outline" size="sm" @click="pinOpen = !pinOpen">{{
                methods?.pin ? 'Сменить' : 'Задать'
              }}</Button>
              <Button v-if="methods?.pin" variant="ghost" size="sm" class="text-destructive" @click="removePin"
                >Удалить</Button
              >
            </div>
            <div v-if="pinOpen" class="mt-3 flex gap-2 pl-8">
              <Input v-model="pin" type="password" inputmode="numeric" placeholder="Новый PIN" aria-label="Новый PIN" />
              <Input
                v-model="pin2"
                type="password"
                inputmode="numeric"
                placeholder="Ещё раз"
                aria-label="Повтор PIN"
                @keydown.enter="savePin"
              />
              <Button size="sm" class="h-8" @click="savePin">Сохранить</Button>
            </div>
          </div>
        </section>

        <section v-if="section === 'ssh' && secretSettings" class="bg-card divide-y rounded-[14px] border">
          <h2 class="px-4 py-3 text-sm font-medium">Секреты и ssh-agent</h2>
          <div class="flex items-center gap-3 px-4 py-3.5">
            <KeyRound class="size-5 text-secret" />
            <div class="flex-1 text-sm">Очищать буфер через</div>
            <Select
              :model-value="String(secretSettings.clipboardClearSeconds)"
              :disabled="settingsBusy"
              @update:model-value="changeSecretSettings({ clipboardClearSeconds: Number($event) })"
              ><SelectTrigger class="w-36" aria-label="Очистка буфера"><SelectValue /></SelectTrigger
              ><SelectContent
                ><SelectItem v-for="n in [15, 30, 60, 90]" :key="n" :value="String(n)"
                  >{{ n }} с</SelectItem
                ></SelectContent
              ></Select
            >
          </div>
          <div class="space-y-2 px-4 py-3.5">
            <label class="flex items-center justify-between text-sm"
              >ssh-agent<Switch
                :model-value="secretSettings.agentEnabled"
                :disabled="settingsBusy"
                aria-label="ssh-agent"
                @update:model-value="changeSecretSettings({ agentEnabled: $event })"
            /></label>
            <p class="break-all font-mono text-xs text-muted-foreground">
              {{ agent?.running ? 'работает' : 'выключен' }} · {{ agent?.identities ?? 0 }} ключей ·
              {{ agent?.socketPath }}
            </p>
            <p v-if="agent?.error" role="alert" class="text-xs text-destructive">{{ agent.error }}</p>
            <p class="text-xs text-muted-foreground">
              Каждая подпись требует Touch ID или пароля Mac. Для работы при заблокированном хранилище включите вход по
              Touch ID.
            </p>
          </div>
          <div class="flex flex-wrap items-center gap-3 px-4 py-3.5">
            <span class="flex-1 text-sm">Доверять ключу после подтверждения</span
            ><Select
              :model-value="String(secretSettings.agentTrustMinutes)"
              :disabled="settingsBusy"
              @update:model-value="changeSecretSettings({ agentTrustMinutes: Number($event) })"
              ><SelectTrigger class="w-36" aria-label="Окно доверия ключу"><SelectValue /></SelectTrigger
              ><SelectContent
                ><SelectItem v-for="n in [0, 1, 5, 15]" :key="n" :value="String(n)">{{
                  n ? `${n} мин` : 'каждый раз'
                }}</SelectItem></SelectContent
              ></Select
            >
          </div>
          <div v-if="sshConfig" class="space-y-3 px-4 py-3.5">
            <div class="flex items-center justify-between">
              <h3 class="text-sm font-medium">Подключить к ssh</h3>
              <span class="text-xs text-muted-foreground">{{
                sshConfig.configured ? 'Подключён' : 'Не подключён'
              }}</span>
            </div>
            <pre class="overflow-auto rounded-md bg-muted/40 p-3 font-mono text-[11px]">{{ sshConfig.block }}</pre>
            <p class="text-xs text-muted-foreground">
              Блок добавляется в конец файла. Ваши настройки сохраняются, перед первой записью создаётся резервная
              копия.
            </p>
            <div class="flex flex-wrap gap-2">
              <Button
                v-if="!sshConfig.configured"
                variant="outline"
                size="sm"
                :disabled="settingsBusy"
                @click="configure(true)"
                >Добавить в ~/.ssh/config</Button
              ><Button v-else variant="outline" size="sm" :disabled="settingsBusy" @click="configure(false)"
                >Убрать</Button
              ><Button variant="ghost" size="sm" @click="copyExport">Копировать export SSH_AUTH_SOCK</Button>
            </div>
          </div>
          <label class="flex items-center justify-between gap-3 px-4 py-3.5 text-sm"
            >Запускать при входе в систему<Switch
              :model-value="secretSettings.launchAtLogin"
              :disabled="settingsBusy"
              aria-label="Запускать при входе"
              @update:model-value="changeSecretSettings({ launchAtLogin: $event })"
          /></label>
        </section>

        <section v-if="section === 'security'" class="bg-card rounded-[14px] border px-[18px] py-4">
          <div class="flex items-center gap-3">
            <KeyRound class="text-secret size-5" />
            <div class="flex-1">
              <div class="font-medium">Сид-фраза</div>
              <div class="text-muted-foreground text-xs">Показ требует подтверждения, скроется через минуту</div>
            </div>
            <template v-if="!seed">
              <Input
                v-if="methods?.pin"
                v-model="seedPin"
                type="password"
                inputmode="numeric"
                placeholder="PIN (или Touch ID)"
                class="w-40"
                aria-label="PIN для показа"
                @keydown.enter="showSeed"
              />
              <Button variant="outline" size="sm" @click="showSeed">Показать</Button>
            </template>
            <Button v-else variant="outline" size="sm" @click="hideSeed">Скрыть · {{ seedLeft }}</Button>
          </div>
          <ol v-if="seed" class="bg-background mt-4 grid grid-cols-4 gap-1.5 rounded-[10px] p-3">
            <li
              v-for="(w, i) in seed"
              :key="i"
              class="bg-card anim-reveal selectable flex h-[34px] items-center gap-2 rounded-[7px] border px-2.5 font-mono text-[13px]"
              :style="{ animationDelay: `${i * 25}ms` }"
            >
              <span class="text-muted-foreground w-[18px] text-right text-xs">{{ i + 1 }}</span
              >{{ w }}
            </li>
          </ol>
        </section>

        <div
          v-if="seed && section === 'security'"
          class="text-muted-foreground -mt-2 flex items-center gap-2.5 text-xs"
        >
          <span class="bg-muted h-1 flex-1 overflow-hidden rounded-full"
            ><span
              class="bg-secret block h-1 transition-[width] duration-1000 ease-linear"
              :style="{ width: `${(seedLeft / 60) * 100}%` }"
          /></span>
          Скроется через {{ seedLeft }} с
        </div>

        <BackupCard v-if="section === 'vault'" />
        <button
          v-if="section === 'vault'"
          class="w-full rounded-xl border bg-card p-4 text-left hover:border-primary"
          @click="router.push('/import')"
        >
          <span class="font-medium">Импорт</span
          ><span class="mt-1 block text-sm text-muted-foreground"
            >Пароли браузеров, коды 2FA, KeePass, Markdown и SSH</span
          >
        </button>
        <TransferSection v-if="section === 'vault'" />
        <SnapshotsSection v-if="section === 'vault'" :key="snapshotsKey" />

        <section
          v-if="section === 'vault'"
          class="bg-card flex items-center gap-3 rounded-[14px] border px-[18px] py-4"
        >
          <FolderOpen class="text-muted-foreground size-5" />
          <div class="min-w-0 flex-1">
            <div class="font-medium">Расположение хранилища</div>
            <div class="text-muted-foreground truncate font-mono text-xs selectable">{{ path }}</div>
          </div>
          <Button variant="outline" size="sm" @click="reveal">Показать в Finder</Button>
        </section>
        <section v-if="section === 'danger'" class="border-destructive/30 rounded-[14px] border px-[18px] py-[18px]">
          <div class="font-medium">Удаление хранилища</div>
          <p class="text-muted-foreground mt-1 text-xs">
            Все записи, секреты, папки и ключи быстрого входа на этом Mac будут удалены. Отменить удаление нельзя.
          </p>
          <Button variant="destructive" size="sm" class="mt-3" @click="resetDeletion(true)">Удалить хранилище…</Button>
        </section>
      </div>
    </div>
    <Dialog :open="deleteOpen" @update:open="resetDeletion">
      <DialogContent
        :show-close-button="!deleting"
        @escape-key-down="deleting && $event.preventDefault()"
        @pointer-down-outside="deleting && $event.preventDefault()"
      >
        <DialogHeader>
          <DialogTitle>Удалить хранилище навсегда?</DialogTitle>
          <DialogDescription
            >Будут удалены все локальные данные, включая корзину. Сид-фраза без резервной копии файлов не восстановит
            удалённые записи.</DialogDescription
          >
        </DialogHeader>
        <p class="text-muted-foreground break-all font-mono text-xs">{{ path }}</p>
        <label class="space-y-2 text-sm"
          ><span>Для подтверждения введите УДАЛИТЬ</span
          ><Input
            v-model="deleteConfirmation"
            :class="confirmShake && 'anim-shake'"
            @keydown.enter="tryDelete"
            :disabled="deleting"
            autocomplete="off"
            aria-label="Подтверждение удаления"
        /></label>
        <label v-if="methods?.pin" class="space-y-2 text-sm"
          ><span>PIN (или оставьте пустым для Touch ID / пароля Mac)</span
          ><Input
            v-model="deletePin"
            :disabled="deleting"
            type="password"
            inputmode="numeric"
            autocomplete="off"
            aria-label="PIN для удаления"
        /></label>
        <p class="text-muted-foreground text-xs">Потребуется подтверждение PIN, Touch ID или паролем Mac.</p>
        <DialogFooter>
          <Button variant="outline" :disabled="deleting" @click="resetDeletion(false)">Отмена</Button>
          <Button variant="destructive" :disabled="deleteConfirmation !== 'УДАЛИТЬ' || deleting" @click="deleteVault">{{
            deleting ? 'Удаление…' : 'Удалить навсегда'
          }}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>
