import { describe, expect, test, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'

const { unlockPin, unlockBiometric, methods } = vi.hoisted(() => ({
  unlockPin: vi.fn(async (_pin: string) => ({ state: 'unlocked', vaultId: 'v' })),
  unlockBiometric: vi.fn(async () => ({ state: 'unlocked', vaultId: 'v' })),
  methods: { biometric: false, biometricAvailable: true, pin: true, attemptsLeft: 5 },
}))
vi.mock('@/lib/api', async (orig) => {
  const real = await orig<typeof import('@/lib/api')>()
  return {
    ...real,
    api: {
      status: async () => ({ state: 'locked', vaultId: 'v' }),
      unlockMethods: async () => ({ ...methods }),
      unlockPin,
      unlockBiometric,
      getSettings: async () => ({ autolockMinutes: 15 }),
    },
  }
})
vi.mock('vue-router', () => ({ useRouter: () => ({ replace: vi.fn() }) }))

import Unlock from '@/views/Unlock.vue'

describe('Unlock PIN entry', () => {
  test('six digits do not submit on their own (PIN may be longer); Enter does', async () => {
    setActivePinia(createPinia())
    const w = mount(Unlock)
    await flushPromises()
    const input = w.get('input[type=password]')
    // Typed one digit at a time, like a person would.
    const pin = '48291355'
    for (let i = 1; i <= pin.length; i++) {
      await input.setValue(pin.slice(0, i))
      await flushPromises()
    }
    expect(unlockPin).not.toHaveBeenCalled()
    await input.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    expect(unlockPin).toHaveBeenCalledWith('48291355')
  })
})

test('Touch ID is requested only after an explicit click', async () => {
  methods.biometric = true
  unlockBiometric.mockClear()
  setActivePinia(createPinia())
  const w = mount(Unlock)
  await flushPromises()
  expect(unlockBiometric).not.toHaveBeenCalled()
  await w.get('button[aria-label="Разблокировать Touch ID"]').trigger('click')
  await flushPromises()
  expect(unlockBiometric).toHaveBeenCalledOnce()
  w.unmount()
  methods.biometric = false
})
