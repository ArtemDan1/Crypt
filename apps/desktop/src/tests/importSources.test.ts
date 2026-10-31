import { expect, test } from 'vitest'
import { IMPORT_SOURCES, defaultImportFolder } from '@/lib/importSources'
test('all five browsers have exactly three export steps', () => {
  const browsers = IMPORT_SOURCES.filter((s) => s.group === 'Пароли из браузера')
  expect(browsers.map((s) => s.id)).toEqual(['chrome', 'safari', 'firefox', 'edge', 'brave'])
  for (const browser of browsers) expect(browser.steps).toHaveLength(3)
})
test('default folder uses the Russian day and month without a period', () => {
  expect(defaultImportFolder('browser', new Date(2026, 9, 31))).toBe('Импорт из браузера · 31 окт')
  expect(defaultImportFolder('otp', new Date(2026, 9, 31))).toBe('Импорт 2FA · 31 окт')
})
