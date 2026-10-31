<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { Fingerprint } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import CryptMark from '@/components/crypt/CryptMark.vue'
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
const shake = ref(false)
const success = ref(false)
const seedCount = computed(() => seedText.value.trim().split(/\s+/).filter(Boolean).length)

const methods = computed(() => session.methods)
const showPin = computed(() => !!methods.value?.pin)
const showBio = computed(() => !!methods.value?.biometric)
const PIN_SLOTS = 6

function message(e: unknown) {
  return e instanceof CryptApiError ? e.message : String(e)
}

async function done() {
  success.value = true
  await new Promise((r) => setTimeout(r, 450))
  await router.replace('/')
}
function shakePin() {
  shake.value = false
  requestAnimationFrame(() => (shake.value = true))
  setTimeout(() => (shake.value = false), 500)
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
    shakePin()
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
    <div class="flex w-full max-w-[400px] flex-col items-center px-4 text-center">
      <div :class="success ? 'unlock-ok' : 'unlock-float'">
        <CryptMark :size="76" :animated="success" />
      </div>
      <h1 class="mt-5 text-[22px] font-semibold tracking-[-.02em]">Хранилище заблокировано</h1>
      <template v-if="!seedMode">
        <template v-if="showBio">
          <div class="relative mt-9 flex size-28 items-center justify-center">
            <template v-if="busy">
              <span class="unlock-ring border-primary absolute inset-0 rounded-full border-2" />
              <span class="unlock-ring unlock-ring-2 border-primary absolute inset-0 rounded-full border-2" />
            </template>
            <button
              class="bg-card relative flex size-[104px] items-center justify-center overflow-hidden rounded-full border shadow-[0_0_0_8px_var(--color-muted)] transition-[transform,color] active:scale-95 disabled:opacity-90"
              :class="busy ? 'text-primary' : 'text-foreground'"
              :disabled="busy"
              aria-label="Разблокировать Touch ID"
              @click="biometric"
            >
              <Fingerprint class="size-12" :stroke-width="1.5" />
              <span
                v-if="busy"
                class="unlock-scan bg-primary absolute inset-x-[18px] top-[51px] h-0.5 rounded-full shadow-[0_0_12px_var(--color-primary)]"
              />
            </button>
          </div>
          <p class="mt-4 text-[14px] font-medium">{{ busy ? 'Проверка…' : 'Touch ID или пароль Mac' }}</p>
        </template>
        <template v-if="showPin">
          <div v-if="showBio" class="text-muted-foreground mt-8 mb-5 flex w-full items-center gap-3 text-xs">
            <span class="bg-border h-px flex-1" />или PIN‑код<span class="bg-border h-px flex-1" />
          </div>
          <label class="relative mt-8 block cursor-text" :class="[showBio && 'mt-0', shake && 'anim-shake']">
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
                class="bg-card flex h-[50px] w-[42px] items-center justify-center rounded-[10px] border-[1.5px] transition-[border-color,box-shadow]"
                :class="
                  error
                    ? 'border-destructive'
                    : i === pin.length + 1
                      ? 'border-primary shadow-[0_0_0_3px_color-mix(in_srgb,var(--color-primary)_15%,transparent)]'
                      : ''
                "
              >
                <span
                  v-if="i <= pin.length"
                  class="anim-tick size-2.5 rounded-full"
                  :class="error ? 'bg-destructive' : 'bg-foreground'"
                />
              </span>
            </span>
          </label>
          <Button class="mt-4 h-9 w-full max-w-[260px]" :disabled="busy || pin.length < 6" @click="submitPin"
            >Открыть</Button
          >
          <p class="text-muted-foreground mt-3 text-xs">
            <template v-if="methods?.attemptsLeft != null">Осталось попыток: {{ methods.attemptsLeft }}. </template
            >После 5 неверных попыток PIN отключается — понадобится Touch ID или сид-фраза
          </p>
        </template>
        <p v-if="error" class="text-destructive anim-in mt-4 text-[13px]" role="alert">{{ error }}</p>
        <button
          class="text-muted-foreground hover:text-foreground mt-10 min-h-9 text-[13px] underline-offset-4 hover:underline"
          @click="setSeedMode(true)"
        >
          Войти по сид-фразе
        </button>
      </template>
      <template v-else>
        <div class="anim-in flex w-full flex-col items-stretch text-left">
          <p class="text-muted-foreground mt-2 text-center text-[13px]">Введите 24 слова сид-фразы</p>
          <Textarea
            v-model="seedText"
            rows="5"
            class="mt-6 font-mono"
            spellcheck="false"
            autocomplete="off"
            aria-label="Сид-фраза"
          />
          <div class="mt-3 flex items-center gap-2.5">
            <span class="bg-muted h-1 flex-1 overflow-hidden rounded-full"
              ><span
                class="bg-primary block h-1 transition-[width] duration-300"
                :style="{ width: `${(Math.min(seedCount, 24) / 24) * 100}%` }"
            /></span>
            <span class="text-muted-foreground font-mono text-xs">{{ Math.min(seedCount, 24) }} / 24</span>
          </div>
          <p v-if="error" class="text-destructive mt-3 text-[13px]" role="alert">{{ error }}</p>
          <div class="mt-4 flex w-full gap-2">
            <Button v-if="showPin || showBio" variant="outline" @click="setSeedMode(false)">Назад</Button>
            <Button class="flex-1" :disabled="busy" @click="submitSeed">Открыть</Button>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.unlock-float {
  animation: float 2.5s ease-in-out 1;
}
@keyframes float {
  0%,
  100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-5px);
  }
}
.unlock-ok {
  animation: ok 0.45s cubic-bezier(0.3, 1.4, 0.5, 1) both;
}
@keyframes ok {
  to {
    transform: scale(1.08);
  }
}
.unlock-ring {
  animation: ring 1.1s cubic-bezier(0.2, 0.7, 0.3, 1) infinite;
}
.unlock-ring-2 {
  animation-delay: 0.35s;
}
@keyframes ring {
  from {
    transform: scale(0.85);
    opacity: 0.7;
  }
  to {
    transform: scale(1.8);
    opacity: 0;
  }
}
.unlock-scan {
  animation: scan 1.1s ease-in-out infinite;
}
@keyframes scan {
  0%,
  100% {
    transform: translateY(-20px);
    opacity: 0;
  }
  30%,
  70% {
    opacity: 1;
  }
  50% {
    transform: translateY(20px);
  }
}
@media (prefers-reduced-motion: reduce) {
  .unlock-float,
  .unlock-ok {
    animation: none;
  }
}
</style>
