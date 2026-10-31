import type { PasswordOptions } from '@/lib/types'

export type GenOptions = Required<PasswordOptions>
export type ProfileId = 'standard' | 'pin6' | 'wifi' | 'phrase5'

const CHAR_SETS = {
  lower: false,
  upper: false,
  digits: false,
  symbols: false,
  minus: false,
  underline: false,
  space: false,
  brackets: false,
  latin1: false,
}

export const DEFAULT_GENERATOR: GenOptions = {
  mode: 'chars',
  length: 20,
  ...CHAR_SETS,
  lower: true,
  upper: true,
  digits: true,
  symbols: true,
  avoidAmbiguous: true,
  eachClass: true,
  include: '',
  exclude: '',
  words: 5,
  separator: '-',
  capitalize: true,
  addNumber: false,
}

export const GENERATOR_PROFILES: { id: ProfileId; label: string; options: Partial<GenOptions> }[] = [
  {
    id: 'standard',
    label: 'Стандартный',
    options: {
      mode: 'chars',
      length: 20,
      ...CHAR_SETS,
      lower: true,
      upper: true,
      digits: true,
      symbols: true,
      avoidAmbiguous: true,
    },
  },
  {
    id: 'pin6',
    label: 'PIN 6 цифр',
    options: { mode: 'chars', length: 6, ...CHAR_SETS, digits: true, avoidAmbiguous: false },
  },
  {
    id: 'wifi',
    label: 'Wi‑Fi',
    options: { mode: 'chars', length: 16, ...CHAR_SETS, lower: true, upper: true, digits: true, avoidAmbiguous: true },
  },
  { id: 'phrase5', label: 'Фраза 5 слов', options: { mode: 'words', words: 5, separator: '-', capitalize: true } },
]

export function applyGeneratorProfile(id: ProfileId): GenOptions {
  const profile = GENERATOR_PROFILES.find((p) => p.id === id)!
  return { ...DEFAULT_GENERATOR, ...profile.options }
}

export function activeProfile(o: GenOptions): ProfileId | null {
  if (o.mode === 'chars' && (o.include || o.exclude)) return null
  const hit = GENERATOR_PROFILES.find((p) =>
    (Object.keys(p.options) as (keyof GenOptions)[]).every((k) => o[k] === p.options[k]),
  )
  return hit?.id ?? null
}

const KEY = 'crypt.pwgen'

export function loadGeneratorOptions(): GenOptions {
  const out: GenOptions = { ...DEFAULT_GENERATOR }
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, unknown>
    for (const k of Object.keys(out) as (keyof GenOptions)[]) {
      const v = raw[k]
      if (typeof v === typeof out[k]) (out as Record<string, unknown>)[k] = v
    }
    if (out.mode !== 'chars' && out.mode !== 'words') out.mode = 'chars'
    if (!Number.isInteger(out.length) || out.length < 4 || out.length > 64) out.length = DEFAULT_GENERATOR.length
    if (!Number.isInteger(out.words) || out.words < 3 || out.words > 12) out.words = DEFAULT_GENERATOR.words
  } catch {}
  return out
}

export function saveGeneratorOptions(o: GenOptions) {
  try {
    localStorage.setItem(KEY, JSON.stringify(o))
  } catch {}
}
