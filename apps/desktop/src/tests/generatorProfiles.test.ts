import { beforeEach, expect, test } from 'vitest'
import { GENERATOR_PROFILES, activeProfile, loadGeneratorOptions, saveGeneratorOptions } from '@/lib/generatorProfiles'

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
