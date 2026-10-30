<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { Fingerprint } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { CryptApiError } from '@/lib/api'
import { useSession } from '@/stores/session'

const session = useSession()
const router = useRouter()

const pin = ref('')
const pinInput = ref<HTMLInputElement | null>(null)
const error = ref('')
const busy = ref(false)
const seedMode = ref(false)
const seedText = ref('')

const methods = computed(() => session.methods)
const showPin = computed(() => !!methods.value?.pin)
const showBio = computed(() => !!methods.value?.biometric)
const PIN_SLOTS = 6

function message(e: unknown) {
  return e instanceof CryptApiError ? e.message : String(e)
}

async function done() {
  await router.replace('/')
}

async function biometric() {
  if (busy.value) return
  busy.value = true
  error.value = ''
  try {
    await session.unlockBiometric()
    await done()
  } catch (e) {
    // A cancelled prompt is not an error worth shouting about.
    if (!(e instanceof CryptApiError && e.code === 'AuthCancelled')) error.value = message(e)
  } finally {
    busy.value = false
  }
}

async function submitPin() {
  if (busy.value || pin.value.length < 6) return
  busy.value = true
  error.value = ''
  try {
    await session.unlockPin(pin.value)
    await done()
  } catch (e) {
    error.value = message(e)
    pin.value = ''
    await nextTick()
    pinInput.value?.focus()
  } finally {
    busy.value = false
  }
}

function onPinInput(e: Event) {
  const v = (e.target as HTMLInputElement).value.replace(/\D/g, '').slice(0, 12)
  pin.value = v
  ;(e.target as HTMLInputElement).value = v
  // No auto-submit: a PIN may be up to 12 digits, and a premature try burns one of 5 attempts.
}

async function submitSeed() {
  const words = seedText.value.trim().toLowerCase().split(/\s+/).filter(Boolean)
  if (words.length !== 24) {
    error.value = `Нужно 24 слова, сейчас ${words.length}`
    return
  }
  busy.value = true
  error.value = ''
  try {
    await session.unlockSeed(words)
    seedText.value = ''
    await done()
  } catch (e) {
    error.value = message(e)
  } finally {
    busy.value = false
  }
}

onMounted(async () => {
  await session.refresh().catch(() => {})
  if (session.status.state === 'unlocked') return done()
  if (showPin.value) pinInput.value?.focus()
  else if (!showBio.value) seedMode.value = true
})
function setSeedMode(on: boolean) {
  seedMode.value = on
  error.value = ''
}
</script>

<template>
  <div class="relative flex h-full items-center justify-center">
    <div data-tauri-drag-region class="absolute inset-x-0 top-0 h-12" />
    <div class="flex w-full max-w-[380px] flex-col items-center px-4 text-center">
      <span
        class="bg-foreground text-background flex size-14 items-center justify-center rounded-[14px] text-[26px] font-bold"
        >C</span
      >
      <h1 class="mt-5 text-[22px] font-semibold tracking-tight">Хранилище заблокировано</h1>

      <template v-if="!seedMode">
        <template v-if="showBio">
          <button
            class="bg-card text-secret mt-9 flex size-28 items-center justify-center rounded-full border shadow-[0_0_0_8px_var(--color-muted)] transition-opacity disabled:opacity-60"
            :disabled="busy"
            aria-label="Разблокировать Touch ID"
            @click="biometric"
          >
            <Fingerprint class="size-[52px]" :stroke-width="1.5" />
          </button>
          <p class="mt-4 font-medium">Touch ID или пароль Mac</p>
        </template>

        <template v-if="showPin">
          <div v-if="showBio" class="text-muted-foreground mt-8 mb-5 flex w-full items-center gap-3 text-xs">
            <span class="bg-border h-px flex-1" />или PIN-код<span class="bg-border h-px flex-1" />
          </div>
          <label class="relative mt-8 block" :class="showBio && 'mt-0'">
            <span class="sr-only">PIN-код</span>
            <input
              ref="pinInput"
              :value="pin"
              type="password"
              inputmode="numeric"
              autocomplete="off"
              class="absolute inset-0 opacity-0"
              :disabled="busy"
              @input="onPinInput"
              @keydown.enter="submitPin"
            />
            <span class="flex gap-2.5" aria-hidden="true">
              <span
                v-for="i in Math.max(PIN_SLOTS, pin.length)"
                :key="i"
                class="bg-card flex h-12 w-10 items-center justify-center rounded-lg border"
                :class="i === pin.length + 1 && 'border-secret'"
              >
                <span v-if="i <= pin.length" class="bg-foreground size-2.5 rounded-full" />
              </span>
            </span>
          </label>
          <Button class="mt-4 w-full max-w-[260px]" :disabled="busy || pin.length < 6" @click="submitPin"
            >Открыть</Button
          >
          <p class="text-muted-foreground mt-3 text-xs">
            <template v-if="methods?.attemptsLeft != null">Осталось попыток: {{ methods.attemptsLeft }}. </template
            >После 5 неверных попыток PIN отключается — понадобится Touch ID или сид-фраза
          </p>
        </template>

        <p v-if="error" class="text-destructive mt-4 text-[13px]" role="alert">{{ error }}</p>
        <button
          class="text-muted-foreground hover:text-foreground mt-10 text-[13px] underline-offset-4 hover:underline"
          @click="setSeedMode(true)"
        >
          Войти по сид-фразе
        </button>
      </template>

      <template v-else>
        <p class="text-muted-foreground mt-2 text-[13px]">Введите 24 слова сид-фразы</p>
        <Textarea
          v-model="seedText"
          rows="5"
          class="mt-6 font-mono"
          spellcheck="false"
          autocomplete="off"
          aria-label="Сид-фраза"
        />
        <p v-if="error" class="text-destructive mt-3 text-[13px]" role="alert">{{ error }}</p>
        <div class="mt-4 flex w-full gap-2">
          <Button v-if="showPin || showBio" variant="outline" @click="setSeedMode(false)">Назад</Button>
          <Button class="flex-1" :disabled="busy" @click="submitSeed">Открыть</Button>
        </div>
      </template>
    </div>
  </div>
</template>
