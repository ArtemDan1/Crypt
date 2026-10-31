import { expect, test, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'

const { printSeed } = vi.hoisted(() => ({ printSeed: vi.fn(async () => {}) }))
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: vi.fn() }))
vi.mock('@/lib/api', () => ({
  CryptApiError: class extends Error {},
  api: { generateSeed: async () => Array.from({ length: 24 }, (_, i) => `word${i + 1}`), printSeed },
}))
vi.mock('vue-router', () => ({ useRoute: () => ({ query: {} }), useRouter: () => ({ replace: vi.fn() }) }))
import Onboarding from '@/views/Onboarding.vue'

test('seed actions copy all words in order and use native desktop printing', async () => {
  setActivePinia(createPinia())
  const writeText = vi.fn(async () => {})
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } })
  const w = mount(Onboarding)
  const button = (text: string) => w.findAll('button').find((b) => b.text().includes(text))!
  await button('Создать новое').trigger('click')
  await flushPromises()
  expect(w.text()).not.toContain('word1')
  expect(button('Проверить').attributes('disabled')).toBeDefined()
  await button('Показать фразу').trigger('click')
  expect(w.text()).toContain('word1')
  await w.get('input[type=checkbox]').setValue(true)
  expect(button('Проверить').attributes('disabled')).toBeUndefined()
  await button('Скопировать').trigger('click')
  expect(writeText).toHaveBeenCalledWith(Array.from({ length: 24 }, (_, i) => `word${i + 1}`).join(' '))
  await button('Распечатать').trigger('click')
  expect(printSeed).toHaveBeenCalledOnce()
  w.unmount()
})
