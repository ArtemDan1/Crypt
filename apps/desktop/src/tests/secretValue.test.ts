import { beforeEach, expect, test, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import SecretValue from '@/components/secret/SecretValue.vue'
import { api } from '@/lib/api'
import { useSession } from '@/stores/session'
vi.mock('@/lib/api', () => ({
  api: {
    setSecret: vi.fn().mockResolvedValue(undefined),
    getSettings: vi.fn().mockResolvedValue({ autolockMinutes: 15 }),
  },
}))
beforeEach(() => {
  vi.clearAllMocks()
  setActivePinia(createPinia())
  useSession().status = { state: 'unlocked', vaultId: 'test' }
})
test('public edits commit on blur', async () => {
  const view = mount(SecretValue, {
    props: {
      recordId: 'login',
      meta: { name: 'username', label: 'Логин', sensitive: false },
      filled: true,
      publicValue: 'anton',
    },
  })
  await view.get('[aria-label="Изменить Логин"]').trigger('click')
  await view.get('input').setValue('bob')
  await view.get('input').trigger('blur')
  await flushPromises()
  expect(api.setSecret).toHaveBeenCalledWith('login', 'username', 'bob')
  expect(view.emitted('saved')).toHaveLength(1)
  view.unmount()
})
test('sensitive drafts require explicit save', async () => {
  const view = mount(SecretValue, {
    props: { recordId: 'login', meta: { name: 'password', label: 'Пароль', sensitive: true }, filled: true },
  })
  await view.get('[aria-label="Изменить Пароль"]').trigger('click')
  await view.get('input').setValue('private draft')
  await view.get('input').trigger('blur')
  expect(api.setSecret).not.toHaveBeenCalled()
  view.unmount()
})
test('draft visibility expires after 30 seconds', async () => {
  vi.useFakeTimers()
  const view = mount(SecretValue, {
    props: { recordId: 'login', meta: { name: 'password', label: 'Пароль', sensitive: true }, filled: true },
  })
  try {
    await view.get('[aria-label="Изменить Пароль"]').trigger('click')
    await view.get('input').setValue('private draft')
    await view.get('[aria-label="Видимость ввода"]').trigger('click')
    expect(view.get('input').attributes('type')).toBe('text')
    await vi.advanceTimersByTimeAsync(30000)
    expect(view.get('input').attributes('type')).toBe('password')
  } finally {
    view.unmount()
    vi.useRealTimers()
  }
})
test('clicking the value itself starts inline editing', async () => {
  const view = mount(SecretValue, {
    props: {
      recordId: 'login',
      meta: { name: 'username', label: 'Логин', sensitive: false },
      filled: true,
      publicValue: 'anton',
    },
  })
  const trigger = view.get('[aria-label="Изменить Логин"]')
  expect(trigger.text()).toContain('anton')
  await trigger.trigger('click')
  expect((view.get('input').element as HTMLInputElement).value).toBe('anton')
  view.unmount()
})
test('sensitive draft saves on Enter and Escape discards it', async () => {
  const view = mount(SecretValue, {
    props: { recordId: 'login', meta: { name: 'password', label: 'Пароль', sensitive: true }, filled: true },
  })
  await view.get('[aria-label="Изменить Пароль"]').trigger('click')
  await view.get('input').setValue('discard me')
  await view.get('input').trigger('keydown', { key: 'Escape' })
  expect(view.find('input').exists()).toBe(false)
  await view.get('[aria-label="Изменить Пароль"]').trigger('click')
  await view.get('input').setValue('new secret')
  await view.get('input').trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(api.setSecret).toHaveBeenCalledWith('login', 'password', 'new secret')
  expect(api.setSecret).toHaveBeenCalledTimes(1)
  view.unmount()
})
