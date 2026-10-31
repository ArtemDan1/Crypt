<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import CryptMark from '@/components/crypt/CryptMark.vue'
import { Check, Copy, Fingerprint, Printer, TriangleAlert } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import { isTauri } from '@tauri-apps/api/core'
import { toast } from 'vue-sonner'
import { api, CryptApiError } from '@/lib/api'
import { useSession } from '@/stores/session'

const route = useRoute()
const router = useRouter()
const session = useSession()

type Step = 1 | 2 | 3 | 4
const step = ref<Step>(route.query.step === '4' ? 4 : 1)
const mode = ref<'create' | 'restore'>('create')
const words = ref<string[]>([])
const restoreText = ref('')
const error = ref('')
const busy = ref(false)
const seedVisible = ref(false)
const seedRecorded = ref(false)

const steps = ['Хранилище', 'Сид-фраза', 'Проверка', 'Touch ID и PIN']

// Step 3: ask for three random positions.
const checkIdx = ref<number[]>([])
const checkInput = ref<string[]>(['', '', ''])

// Step 4
const biometric = ref(false)
const biometricAvailable = ref(false)
const pin = ref('')
const pin2 = ref('')

onMounted(async () => {
  if (step.value === 4) await loadMethods()
})

async function loadMethods() {
  const m = await api.unlockMethods().catch(() => null)
  biometricAvailable.value = m?.biometricAvailable ?? false
  biometric.value = m?.biometric ?? false
}

function fail(e: unknown) {
  error.value = e instanceof CryptApiError ? e.message : String(e)
}

async function startCreate() {
  seedVisible.value = false
  seedRecorded.value = false
  mode.value = 'create'
  words.value = await api.generateSeed()
  error.value = ''
  step.value = 2
}
async function printSeed() {
  try {
    if (isTauri()) await api.printSeed()
    else window.print()
  } catch (e) {
    fail(e)
  }
}
async function copySeed() {
  try {
    await navigator.clipboard.writeText(words.value.join(' '))
    toast('Сид-фраза скопирована')
  } catch {
    error.value = 'Не удалось скопировать сид-фразу. Выделите слова и скопируйте вручную.'
  }
}
function startRestore() {
  mode.value = 'restore'
  error.value = ''
  step.value = 2
}

function toVerify() {
  const pool = [...Array(24).keys()]
  checkIdx.value = [0, 1, 2]
    .map(() => pool.splice(Math.floor(Math.random() * pool.length), 1)[0]!)
    .sort((a, b) => a - b)
  checkInput.value = ['', '', '']
  error.value = ''
  step.value = 3
}

const verifyOk = computed(() =>
  checkIdx.value.every((i, k) => checkInput.value[k]!.trim().toLowerCase() === words.value[i]),
)

async function createWith(list: string[]) {
  busy.value = true
  error.value = ''
  try {
    await session.create(list)
    words.value = []
    await router.replace({ path: '/onboarding', query: { step: '4' } })
    step.value = 4
    await loadMethods()
  } catch (e) {
    fail(e)
  } finally {
    busy.value = false
  }
}

async function confirmVerify() {
  if (!verifyOk.value) {
    error.value = 'Слова не совпадают. Проверьте записанную фразу.'
    return
  }
  await createWith(words.value)
}

async function confirmRestore() {
  const list = restoreText.value.trim().toLowerCase().split(/\s+/).filter(Boolean)
  if (list.length !== 24) {
    error.value = `Нужно 24 слова, сейчас ${list.length}`
    return
  }
  await createWith(list)
}

async function toggleBiometric(on: boolean) {
  error.value = ''
  try {
    if (on) await api.enableBiometric()
    else await api.disableBiometric()
    biometric.value = on
  } catch (e) {
    biometric.value = !on
    fail(e)
  }
}

const pinValid = computed(() => /^\d{6,12}$/.test(pin.value) && pin.value === pin2.value)

async function finish(destination = '/') {
  busy.value = true
  error.value = ''
  try {
    if (pin.value) {
      if (!pinValid.value) {
        error.value = 'PIN — от 6 до 12 цифр, оба поля должны совпадать'
        return
      }
      await api.setPin(pin.value)
    }
    await session.refresh()
    await router.replace(destination)
  } catch (e) {
    fail(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="flex h-full overflow-auto bg-background">
    <aside class="hidden w-[300px] shrink-0 flex-col bg-sidebar p-9 text-sidebar-foreground sm:flex">
      <div data-tauri-drag-region class="h-8" />
      <CryptMark :size="64" />
      <h2 class="mt-6 text-2xl font-semibold">Только у вас.</h2>
      <p class="mt-3 text-sm text-sidebar-foreground/70">
        Всё шифруется прямо на этом Mac. Ключ от хранилища — только в ваших руках.
      </p>
      <ol class="mt-12 space-y-7 border-l border-sidebar-border pl-5" aria-label="Шаги">
        <li
          v-for="(label, i) in steps"
          :key="label"
          class="flex items-center gap-3 text-sm"
          :aria-current="i + 1 === step ? 'step' : undefined"
        >
          <span
            class="flex size-7 items-center justify-center rounded-full border"
            :class="i + 1 <= step ? 'bg-primary text-primary-foreground border-primary' : 'border-sidebar-border'"
          >
            <Check v-if="i + 1 < step" class="size-4" /><span v-else>{{ i + 1 }}</span> </span
          >{{ label }}
        </li>
      </ol>
    </aside>
    <main class="mx-auto w-full max-w-[720px] px-8 pt-16 pb-8 anim-in">
      <!-- 1. Choose -->
      <section v-if="step === 1" class="space-y-6">
        <div>
          <h1 class="text-[28px] font-semibold tracking-tight">Добро пожаловать в Crypt</h1>
          <p class="text-muted-foreground mt-2 leading-relaxed">
            Зашифрованная база знаний и хранилище паролей. Всё лежит только на этом Mac и открывается вашей сид-фразой.
          </p>
        </div>
        <div class="grid gap-3 sm:grid-cols-2">
          <button class="hover:bg-accent rounded-xl border p-5 text-left transition-colors" @click="startCreate">
            <div class="font-medium">Создать новое хранилище</div>
            <div class="text-muted-foreground mt-1 text-[13px]">Сгенерируем сид-фразу из 24 слов</div>
          </button>
          <button class="hover:bg-accent rounded-xl border p-5 text-left transition-colors" @click="startRestore">
            <div class="font-medium">Восстановить по сид-фразе</div>
            <div class="text-muted-foreground mt-1 text-[13px]">Если фраза уже есть</div>
          </button>
        </div>
        <button
          class="w-full rounded-xl border bg-card p-5 text-left hover:border-primary"
          @click="router.push('/restore')"
        >
          <div class="font-medium">Из бэкапа</div>
          <div class="mt-1 text-[13px] text-muted-foreground">Восстановить всю базу из файла .cryptbackup</div>
        </button>
      </section>

      <!-- 2a. Show seed -->
      <section v-else-if="step === 2 && mode === 'create'" class="seed-sheet space-y-6">
        <div>
          <h1 class="text-[28px] font-semibold tracking-tight">Запишите сид-фразу</h1>
          <p class="text-muted-foreground mt-2 leading-relaxed">
            Эти 24 слова — единственный способ восстановить хранилище и подключить новое устройство. Сервера нет, мы их
            нигде не храним.
          </p>
        </div>
        <Button v-if="!seedVisible" variant="outline" @click="seedVisible = true">Показать фразу</Button>
        <ol v-if="seedVisible" class="bg-card/40 grid grid-cols-4 gap-2 rounded-xl border p-4 print:bg-white">
          <li
            v-for="(w, i) in words"
            :key="i"
            :style="{ animationDelay: `${i * 25}ms` }"
            class="anim-reveal bg-secret-bg text-secret-fg flex h-[38px] items-center gap-2 rounded-md px-2.5 font-mono selectable"
          >
            <span class="text-muted-foreground w-[18px] text-right text-xs">{{ i + 1 }}</span>
            <span>{{ w }}</span>
          </li>
        </ol>
        <div
          role="note"
          class="flex gap-3 rounded-lg border border-border-strong bg-secret-bg px-4 py-3.5 leading-relaxed text-secret-fg"
        >
          <TriangleAlert class="mt-0.5 size-[18px] shrink-0" />
          Запишите на бумаге и храните офлайн. Не фотографируйте и не храните в заметках. Потеряете фразу — данные не
          восстановить никак.
        </div>
        <p v-if="error" class="text-destructive text-[13px] print:hidden">{{ error }}</p>
        <label class="flex items-center gap-2 text-sm print:hidden">
          <input v-model="seedRecorded" type="checkbox" :disabled="!seedVisible" /> Я записал(а) фразу
        </label>
        <div class="flex flex-wrap items-center gap-3 print:hidden">
          <Button variant="outline" @click="step = 1">Назад</Button>
          <Button variant="outline" @click="copySeed"><Copy class="size-4" /> Скопировать</Button>
          <Button variant="outline" @click="printSeed"><Printer class="size-4" /> Распечатать</Button>
          <div class="flex-1" />
          <Button :disabled="!seedVisible || !seedRecorded" @click="toVerify">Проверить</Button>
        </div>
      </section>

      <!-- 2b. Restore -->
      <section v-else-if="step === 2" class="space-y-6">
        <div>
          <h1 class="text-[28px] font-semibold tracking-tight">Введите сид-фразу</h1>
          <p class="text-muted-foreground mt-2">24 слова через пробел или с новой строки.</p>
        </div>
        <Textarea
          v-model="restoreText"
          rows="6"
          class="font-mono"
          spellcheck="false"
          autocomplete="off"
          aria-label="Сид-фраза"
        />
        <p v-if="error" class="text-destructive text-[13px]">{{ error }}</p>
        <div class="flex gap-3">
          <Button variant="outline" @click="step = 1">Назад</Button>
          <div class="flex-1" />
          <Button :disabled="busy" @click="confirmRestore">Восстановить</Button>
        </div>
      </section>

      <!-- 3. Verify -->
      <section v-else-if="step === 3" class="space-y-6">
        <div>
          <h1 class="text-[28px] font-semibold tracking-tight">Проверим, что фраза записана</h1>
          <p class="text-muted-foreground mt-2">Введите слова под этими номерами.</p>
        </div>
        <div class="grid gap-4 sm:grid-cols-3">
          <div v-for="(idx, k) in checkIdx" :key="idx" class="space-y-1.5">
            <Label :for="`w${k}`">Слово №{{ idx + 1 }}</Label>
            <Input
              :id="`w${k}`"
              :aria-invalid="!!checkInput[k] && checkInput[k]!.trim().toLowerCase() !== words[idx]"
              :class="
                checkInput[k]
                  ? checkInput[k]!.trim().toLowerCase() === words[idx]
                    ? 'border-primary'
                    : 'border-destructive'
                  : ''
              "
              v-model="checkInput[k]"
              class="font-mono"
              autocomplete="off"
              spellcheck="false"
              @keydown.enter="confirmVerify"
            />
            <Check
              v-if="checkInput[k] && checkInput[k]!.trim().toLowerCase() === words[idx]"
              class="size-4 text-primary"
              aria-label="Слово совпадает"
            />
          </div>
        </div>
        <p v-if="error" class="text-destructive text-[13px]">{{ error }}</p>
        <div class="flex gap-3">
          <Button variant="outline" @click="step = 2">Показать фразу ещё раз</Button>
          <div class="flex-1" />
          <Button :disabled="busy" @click="confirmVerify">Создать хранилище</Button>
        </div>
      </section>

      <!-- 4. Quick unlock -->
      <section v-else class="space-y-6">
        <div>
          <h1 class="text-[28px] font-semibold tracking-tight">Быстрый вход</h1>
          <p class="text-muted-foreground mt-2">Чтобы не вводить сид-фразу каждый раз.</p>
        </div>
        <div class="divide-y rounded-xl border">
          <div class="flex items-center gap-3 px-4 py-3.5">
            <Fingerprint class="text-muted-foreground size-5" />
            <div class="flex-1">
              <div class="font-medium">Touch ID</div>
              <div class="text-muted-foreground text-xs">
                {{ biometricAvailable ? 'Или пароль Mac, если Touch ID недоступен' : 'Недоступно на этом Mac' }}
              </div>
            </div>
            <Switch
              :model-value="biometric"
              :disabled="!biometricAvailable"
              aria-label="Touch ID"
              @update:model-value="(v: boolean) => toggleBiometric(v)"
            />
          </div>
          <div class="space-y-3 px-4 py-3.5">
            <div class="font-medium">
              PIN-код <span class="text-muted-foreground text-xs font-normal">— необязательно, от 6 цифр</span>
            </div>
            <div class="grid gap-3 sm:grid-cols-2">
              <Input
                v-model="pin"
                type="password"
                inputmode="numeric"
                placeholder="PIN"
                aria-label="PIN"
                autocomplete="off"
              />
              <Input
                v-model="pin2"
                type="password"
                inputmode="numeric"
                placeholder="Ещё раз"
                aria-label="Повтор PIN"
                autocomplete="off"
              />
            </div>
            <p v-if="pin2" class="text-xs" :class="pinValid ? 'text-primary' : 'text-destructive'" role="status">
              {{ pinValid ? 'PIN совпадает' : 'PIN должен совпадать и содержать от 6 до 12 цифр' }}
            </p>
          </div>
        </div>
        <p class="text-sm text-muted-foreground">
          SSH-ключи можно импортировать в разделе «Секреты».
          <button :disabled="busy" class="text-secret hover:underline" @click="finish('/settings')">
            Настроить ssh-agent
          </button>
        </p>
        <p v-if="error" class="text-destructive text-[13px]">{{ error }}</p>
        <div class="flex">
          <div class="flex-1" />
          <Button :disabled="busy" @click="finish()">Готово</Button>
        </div>
      </section>
    </main>
  </div>
</template>
