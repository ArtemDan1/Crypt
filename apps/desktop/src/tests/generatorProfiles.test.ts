import { beforeEach, expect, test } from 'vitest'
import {
  GENERATOR_PROFILES,
  applyGeneratorProfile,
  activeProfile,
  loadGeneratorOptions,
  saveGeneratorOptions,
} from '@/lib/generatorProfiles'

beforeEach(() => localStorage.clear())

test('options of a profile are recognised as that profile', () => {
  const wifi = GENERATOR_PROFILES.find((p) => p.id === 'wifi')!
  const o = { ...loadGeneratorOptions(), ...wifi.options }
  expect(activeProfile(o)).toBe('wifi')
  expect(activeProfile({ ...o, length: 17 })).toBeNull()
})

test('garbage in storage gives defaults', () => {
  localStorage.setItem('crypt.pwgen', '{not json')
  const o = loadGeneratorOptions()
  expect(o.length).toBe(20)
  expect(o.mode).toBe('chars')
})

test('saved options are read back', () => {
  saveGeneratorOptions({ ...loadGeneratorOptions(), mode: 'words', words: 7 })
  const o = loadGeneratorOptions()
  expect(o.mode).toBe('words')
  expect(o.words).toBe(7)
})

test('named PIN profile resets overrides and custom overrides are not called a PIN profile', () => {
  const pin = applyGeneratorProfile('pin6')
  expect(pin.include).toBe('')
  expect(pin.exclude).toBe('')
  expect(pin.digits).toBe(true)
  expect(pin.lower || pin.upper || pin.symbols || pin.latin1 || pin.space).toBe(false)
  expect(activeProfile({ ...pin, include: '€' })).toBeNull()
})
