import { beforeEach, expect, test, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
const mocks = vi.hoisted(() => ({
  previewBrowserCsv: vi.fn(),
  importBrowserCsv: vi.fn(),
  previewOtp: vi.fn(),
  importOtp: vi.fn(),
  cancelOtpImport: vi.fn(async () => {}),
  trashFile: vi.fn(async () => {}),
  hooks: [] as (() => void)[],
}))
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => false }))
vi.mock('@/stores/library', () => ({ useLibrary: () => ({ folders: [], changed: async () => {} }) }))
vi.mock('@/stores/session', () => ({
  useSession: () => ({
    beforeLock: (fn: () => void) => {
      mocks.hooks.push(fn)
      return () => {}
    },
  }),
}))
vi.mock('@/lib/api', () => ({
  api: mocks,
  CryptApiError: class extends Error {
    code: string
    constructor(code: string, message: string) {
      super(message)
      this.code = code
    }
  },
}))
import Import from '@/views/Import.vue'
import { CryptApiError } from '@/lib/api'
const mountWizard = () => mount(Import, { global: { stubs: { SshImportDialog: true, LegacyImport: true } } })
const button = (w: ReturnType<typeof mountWizard>, text: string) =>
  w.findAll('button').find((b) => b.text().includes(text))!
beforeEach(() => {
  vi.clearAllMocks()
  mocks.hooks = []
  mocks.previewBrowserCsv.mockResolvedValue({
    format: 'chromium',
    new: 1,
    duplicates: 0,
    skipped: 1,
    sample: [['Mail', 'alice']],
  })
  mocks.importBrowserCsv.mockResolvedValue({
    report: { imported: 1, duplicates: 0, skipped: [[3, 'NoPassword']] },
    snapshot: 'Before',
  })
  mocks.importOtp.mockResolvedValue({
    report: { attached: 0, created: 1, skipped: [], missing_batches: [] },
    snapshot: 'Before',
  })
})
test('CSV wizard previews, imports and offers to trash the plaintext source', async () => {
  const w = mountWizard()
  await button(w, 'Chrome').trigger('click')
  await button(w, 'Выбрать файл').trigger('click')
  await flushPromises()
  expect(w.text()).toContain('Mail')
  expect(w.text()).toContain('••••••••')
  await button(w, 'Импортировать (1)').trigger('click')
  await flushPromises()
  expect(w.text()).toContain('Строка 3: Нет пароля')
  expect(w.text()).toContain('Файл хранит пароли открытым текстом')
  await button(w, 'Переместить в корзину').trigger('click')
  await flushPromises()
  expect(mocks.trashFile).toHaveBeenCalledWith('/demo/import.csv')
  w.unmount()
})
test('encrypted Aegis asks for its password and imports the preview token', async () => {
  mocks.previewOtp
    .mockRejectedValueOnce(new CryptApiError('NeedsPassword', 'Введите пароль экспорта'))
    .mockResolvedValueOnce({
      token: 'opaque',
      rows: [{ issuer: 'Mail', account: 'alice', target: 'new' }],
      skipped: [],
      missingBatches: [],
    })
  const w = mountWizard()
  await button(w, 'Aegis').trigger('click')
  await button(w, 'Выбрать файл').trigger('click')
  await flushPromises()
  await w.get('input[type=password]').setValue('synthetic')
  await w.get('form').trigger('submit')
  await flushPromises()
  expect(w.find('input[type=password]').exists()).toBe(false)
  await button(w, 'Импортировать (1)').trigger('click')
  await flushPromises()
  expect(mocks.importOtp).toHaveBeenCalledWith('opaque', expect.any(String))
  w.unmount()
})
test('locking while a preview is pending never repopulates the screen', async () => {
  let resolve!: (value: unknown) => void
  mocks.previewBrowserCsv.mockReturnValue(new Promise((r) => (resolve = r)))
  const w = mountWizard()
  await button(w, 'Chrome').trigger('click')
  await button(w, 'Выбрать файл').trigger('click')
  mocks.hooks.forEach((fn) => fn())
  resolve({ new: 1, duplicates: 0, skipped: 0, sample: [['Late secret', 'alice']] })
  await flushPromises()
  expect(w.text()).not.toContain('Late secret')
  expect(w.text()).toContain('Перенесите секреты')
  expect(mocks.cancelOtpImport).toHaveBeenCalled()
  w.unmount()
})

test('a new preview waits until cancellation of the previous one completes', async () => {
  let release!: () => void
  mocks.cancelOtpImport.mockImplementationOnce(
    () =>
      new Promise<void>((resolve) => {
        release = resolve
      }),
  )
  const w = mountWizard()
  await button(w, 'Chrome').trigger('click')
  await button(w, 'Выбрать файл').trigger('click')
  expect(mocks.previewBrowserCsv).not.toHaveBeenCalled()
  release()
  await flushPromises()
  expect(mocks.previewBrowserCsv).toHaveBeenCalledOnce()
  w.unmount()
})
