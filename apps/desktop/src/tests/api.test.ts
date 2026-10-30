import { beforeEach, describe, expect, test, vi } from 'vitest'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }))

import { api, call, CryptApiError } from '@/lib/api'

describe('call', () => {
  beforeEach(() => invoke.mockReset())

  test('structured rejection becomes CryptApiError with code', async () => {
    invoke.mockRejectedValueOnce({ code: 'WrongPin', message: 'Неверный PIN. Осталось попыток: 4' })
    const err = (await call('unlock_pin', { pin: '000000' }).catch((e: unknown) => e)) as CryptApiError
    expect(err).toBeInstanceOf(CryptApiError)
    expect(err.code).toBe('WrongPin')
    expect(err.message).toContain('Осталось попыток')
  })

  test('string rejection becomes code Unknown', async () => {
    invoke.mockRejectedValueOnce('boom')
    const err = (await call('status').catch((e: unknown) => e)) as CryptApiError
    expect(err).toBeInstanceOf(CryptApiError)
    expect(err.code).toBe('Unknown')
    expect(err.message).toBe('boom')
  })

  test('named wrappers pass camelCase args to snake_case commands', async () => {
    invoke.mockResolvedValueOnce({ state: 'unlocked', vaultId: 'x' })
    await api.unlockPin('482913')
    expect(invoke).toHaveBeenCalledWith('unlock_pin', { pin: '482913' })
  })
})
