<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { Check, ChevronRight, Copy, KeyRound, RefreshCw } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import SegmentedControl from '@/components/crypt/SegmentedControl.vue'
import { api, CryptApiError } from '@/lib/api'
import {
  GENERATOR_PROFILES,
  activeProfile,
  applyGeneratorProfile,
  loadGeneratorOptions,
  saveGeneratorOptions,
  type GenOptions,
} from '@/lib/generatorProfiles'
import type { GeneratedPassword } from '@/lib/types'
import { useLibrary } from '@/stores/library'
import { useSession } from '@/stores/session'

const router = useRouter()
const lib = useLibrary()
const o = ref<GenOptions>(loadGeneratorOptions())
const main = ref<GeneratedPassword | null>(null)
const alts = ref<GeneratedPassword[]>([])
const error = ref('')
const spin = ref(0)
const copied = ref<number | null>(null)
let seq = 0

const modes = [
  { value: 'chars' as const, label: 'Символы' },
  { value: 'words' as const, label: 'Фраза' },
]
const sets: { key: keyof GenOptions; label: string; sample: string }[] = [
  { key: 'upper', label: 'Заглавные', sample: 'A–Z' },
  { key: 'lower', label: 'Строчные', sample: 'a–z' },
  { key: 'digits', label: 'Цифры', sample: '0–9' },
  { key: 'minus', label: 'Минус', sample: '-' },
  { key: 'underline', label: 'Подчёркивание', sample: '_' },
  { key: 'space', label: 'Пробел', sample: '␣' },
  { key: 'symbols', label: 'Спецсимволы', sample: '!#$%' },
  { key: 'brackets', label: 'Скобки', sample: '[{(<' },
  { key: 'latin1', label: 'Латиница‑1', sample: 'äöß' },
]
const separators = [
  { value: '-', label: '-' },
  { value: '.', label: '.' },
  { value: ' ', label: 'пробел' },
  { value: '_', label: '_' },
]

async function generate(withAlts = true) {
  const n = ++seq
  try {
    const out = await api.generatePasswords({ ...o.value }, withAlts ? 6 : 1)
    if (n !== seq) return
    error.value = ''
    main.value = out[0] ?? null
    if (withAlts) alts.value = out.slice(1)
    spin.value += 180
  } catch (e) {
    if (n !== seq) return
    main.value = null
    alts.value = []
    error.value = e instanceof CryptApiError ? e.message : String(e)
  }
}
watch(
  o,
  (v) => {
    saveGeneratorOptions(v)
    generate()
  },
  { deep: true, immediate: true },
)
// Drop generated values when the vault locks.
const unhook = useSession().beforeLock(() => {
  main.value = null
  alts.value = []
})
onBeforeUnmount(unhook)

const bits = computed(() => main.value?.bits ?? 0)
const quality = computed(() =>
  bits.value >= 100 ? 'очень надёжный' : bits.value >= 75 ? 'надёжный' : bits.value >= 50 ? 'средний' : 'слабый',
)
const meterTone = computed(() => (bits.value >= 75 ? 'bg-primary' : bits.value >= 50 ? 'bg-secret' : 'bg-destructive'))
const qualityTone = computed(() =>
  bits.value >= 75 ? 'text-primary' : bits.value >= 50 ? 'text-secret' : 'text-destructive',
)
const chars = computed(() =>
  [...(main.value?.value ?? '')].map((c) => ({
    c,
    tone: /[0-9]/.test(c) ? 'text-primary-soft-fg' : /\p{L}/u.test(c) ? '' : 'text-foreground',
  })),
)
const profile = computed(() => activeProfile(o.value))

function applyProfile(p: (typeof GENERATOR_PROFILES)[number]) {
  o.value = applyGeneratorProfile(p.id)
}
async function copy(value: string, i: number | null = null) {
  try {
    await api.copyText(value)
    const s = await api.getSettings()
    if (i === null) toast.success(`Пароль скопирован · очистится через ${s.clipboardClearSeconds} с`)
    else {
      copied.value = i
      setTimeout(() => copied.value === i && (copied.value = null), 1400)
    }
  } catch (e) {
    toast.error(e instanceof CryptApiError ? e.message : 'Не удалось скопировать')
  }
}
function use(alt: GeneratedPassword) {
  main.value = alt
}
async function saveAsSecret() {
  if (!main.value) return
  try {
    const id = await api.createRecord({ title: 'Новый логин', secret_kind: 'login' })
    await api.setSecret(id, 'password', main.value.value)
    await lib.changed()
    await router.push(`/r/${id}`)
  } catch (e) {
    toast.error(e instanceof CryptApiError ? e.message : 'Не удалось создать секрет')
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <header data-tauri-drag-region class="flex h-[52px] shrink-0 items-center gap-2.5 border-b pr-5 pl-7">
      <RouterLink to="/secrets" class="text-muted-foreground hover:text-foreground text-[13px]">Секреты</RouterLink>
      <ChevronRight class="text-muted-foreground size-3" />
      <span class="flex-1 text-[13px]">Генератор паролей</span>
      <SegmentedControl v-model="o.mode" :options="modes" label="Режим" size="sm" />
    </header>
    <div class="flex min-h-0 flex-1">
      <div class="flex min-w-0 flex-1 flex-col gap-6 overflow-auto px-8 py-7">
        <div class="flex flex-col gap-3">
          <div class="bg-secret-bg flex items-center gap-2 rounded-[14px] border py-1.5 pr-1.5 pl-[18px]">
            <span
              :key="main?.value ?? 'none'"
              class="text-secret-fg anim-reveal selectable min-w-0 flex-1 py-2.5 font-mono text-[22px] leading-normal break-all"
            >
              <template v-if="main"
                ><span v-for="(ch, i) in chars" :key="i" :class="ch.tone">{{ ch.c }}</span></template
              >
              <span v-else class="text-muted-foreground font-sans text-base">—</span>
            </span>
            <button
              class="text-secret-fg hover:bg-foreground/[.08] flex size-11 items-center justify-center rounded-[10px] transition-colors"
              aria-label="Сгенерировать заново"
              @click="generate(false)"
            >
              <RefreshCw
                class="size-[18px] transition-transform duration-500 ease-[cubic-bezier(.3,1.3,.5,1)]"
                :style="{ transform: `rotate(${spin}deg)` }"
              />
            </button>
            <Button class="h-11 px-4" :disabled="!main" @click="main && copy(main.value)"
              ><Copy class="size-4" /> Скопировать</Button
            >
          </div>
          <div class="flex items-center gap-3">
            <span class="bg-muted h-1.5 flex-1 overflow-hidden rounded-full"
              ><span
                class="block h-1.5 rounded-full transition-[width,background-color] duration-500 ease-[cubic-bezier(.2,.8,.2,1)]"
                :class="meterTone"
                :style="{ width: `${Math.min(bits / 128, 1) * 100}%` }"
            /></span>
            <span class="text-[13px] font-medium" :class="qualityTone">{{ bits }} бит</span>
            <span class="text-muted-foreground text-[13px]">{{ quality }}</span>
          </div>
          <p v-if="error" class="text-destructive text-[13px]" role="alert">{{ error }}</p>
        </div>

        <div v-if="o.mode === 'chars'" class="flex flex-col gap-5">
          <label class="flex items-center gap-3.5">
            <span class="text-muted-foreground w-[90px] text-[13px]">Длина</span>
            <input v-model.number="o.length" type="range" min="4" max="64" class="accent-primary flex-1" />
            <span class="flex h-8 w-11 items-center justify-center rounded-lg border font-mono text-sm">{{
              o.length
            }}</span>
          </label>
          <div class="grid grid-cols-3 gap-2">
            <button
              v-for="s in sets"
              :key="s.key"
              class="hover:bg-foreground/[.04] flex h-10 items-center gap-2.5 rounded-[9px] border px-3 text-left transition-colors"
              :aria-pressed="!!o[s.key]"
              @click="(o[s.key] as boolean) = !o[s.key]"
            >
              <span
                class="flex size-[18px] items-center justify-center rounded-[5px] border-[1.5px] transition-colors"
                :class="o[s.key] ? 'bg-primary border-primary text-primary-foreground' : 'border-border-strong'"
              >
                <Check v-if="o[s.key]" class="anim-tick size-3" :stroke-width="3.4" />
              </span>
              <span class="flex-1 text-[13px]">{{ s.label }}</span>
              <span class="text-muted-foreground font-mono text-xs">{{ s.sample }}</span>
            </button>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <label class="text-muted-foreground flex flex-col gap-1.5 text-xs"
              >Добавить символы
              <input
                v-model.lazy="o.include"
                placeholder="например, €§"
                class="bg-background text-foreground h-9 rounded-lg border px-3 font-mono text-sm outline-none"
            /></label>
            <label class="text-muted-foreground flex flex-col gap-1.5 text-xs"
              >Исключить символы
              <input
                v-model.lazy="o.exclude"
                placeholder="например, ^`"
                class="bg-background text-foreground h-9 rounded-lg border px-3 font-mono text-sm outline-none"
            /></label>
          </div>
          <div class="flex flex-col gap-2.5">
            <label class="flex items-center gap-3 text-[13px]">
              <span class="flex-1"
                >Исключить похожие <span class="text-muted-foreground font-mono text-xs">O0Il1|</span></span
              >
              <Switch v-model="o.avoidAmbiguous" aria-label="Исключить похожие" />
            </label>
            <label class="flex items-center gap-3 text-[13px]">
              <span class="flex-1">Каждый набор хотя бы раз</span>
              <Switch v-model="o.eachClass" aria-label="Каждый набор хотя бы раз" />
            </label>
          </div>
        </div>

        <div v-else class="flex flex-col gap-5">
          <label class="flex items-center gap-3.5">
            <span class="text-muted-foreground w-[90px] text-[13px]">Слов</span>
            <input v-model.number="o.words" type="range" min="3" max="12" class="accent-primary flex-1" />
            <span class="flex h-8 w-11 items-center justify-center rounded-lg border font-mono text-sm">{{
              o.words
            }}</span>
          </label>
          <div class="flex items-center gap-3.5">
            <span class="text-muted-foreground w-[90px] text-[13px]">Разделитель</span>
            <SegmentedControl v-model="o.separator" :options="separators" label="Разделитель" size="sm" />
          </div>
          <div class="flex flex-col gap-2.5">
            <label class="flex items-center gap-3 text-[13px]">
              <span class="flex-1">С заглавной буквы</span>
              <Switch v-model="o.capitalize" aria-label="С заглавной буквы" />
            </label>
            <label class="flex items-center gap-3 text-[13px]">
              <span class="flex-1">Добавить число в конце</span>
              <Switch v-model="o.addNumber" aria-label="Добавить число в конце" />
            </label>
          </div>
          <p class="text-muted-foreground text-xs">
            Словарь EFF: 7 776 английских слов, каждое добавляет около 12,9 бита
          </p>
        </div>
      </div>

      <aside class="bg-background flex w-[320px] shrink-0 flex-col gap-5 border-l px-[18px] py-6">
        <section class="flex flex-col gap-2">
          <h3 class="text-muted-foreground text-[11px] font-semibold tracking-wider uppercase">Профили</h3>
          <div class="flex flex-wrap gap-1.5">
            <button
              v-for="p in GENERATOR_PROFILES"
              :key="p.id"
              class="h-[30px] rounded-full border px-2.5 text-xs font-medium transition-colors active:scale-95"
              :class="
                profile === p.id
                  ? 'bg-primary-soft text-primary-soft-fg border-transparent'
                  : 'bg-card hover:bg-foreground/[.04]'
              "
              @click="applyProfile(p)"
            >
              {{ p.label }}
            </button>
          </div>
        </section>
        <section class="flex min-h-0 flex-1 flex-col gap-1.5">
          <div class="flex items-center justify-between">
            <h3 class="text-muted-foreground text-[11px] font-semibold tracking-wider uppercase">Ещё варианты</h3>
            <button
              class="text-muted-foreground hover:bg-foreground/[.08] h-[26px] rounded-md px-2 text-xs"
              @click="generate()"
            >
              Обновить
            </button>
          </div>
          <div class="flex flex-col gap-1">
            <div
              v-for="(a, i) in alts"
              :key="a.value"
              class="bg-card anim-in flex items-center gap-1.5 rounded-lg border py-1.5 pr-1 pl-2.5"
              :style="{ animationDelay: `${i * 40}ms` }"
            >
              <span class="selectable min-w-0 flex-1 truncate font-mono text-xs">{{ a.value }}</span>
              <span v-if="copied === i" class="text-primary anim-tick pr-1.5 text-[11px]">Скопировано</span>
              <template v-else>
                <button
                  class="text-muted-foreground hover:bg-foreground/[.08] flex size-7 items-center justify-center rounded-md"
                  aria-label="Выбрать этот вариант"
                  @click="use(a)"
                >
                  <ChevronRight class="size-3.5" />
                </button>
                <button
                  class="text-muted-foreground hover:bg-foreground/[.08] flex size-7 items-center justify-center rounded-md"
                  aria-label="Скопировать"
                  @click="copy(a.value, i)"
                >
                  <Copy class="size-3.5" />
                </button>
              </template>
            </div>
          </div>
        </section>
        <Button variant="outline" class="h-[42px]" :disabled="!main" @click="saveAsSecret"
          ><KeyRound class="text-secret size-4" /> Сохранить как секрет</Button
        >
      </aside>
    </div>
  </div>
</template>
