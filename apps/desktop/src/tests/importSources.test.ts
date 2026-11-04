import { expect, test } from 'vitest'
import { IMPORT_SOURCES } from '@/lib/importSources'
test('all five browsers have exactly three export steps', () => {
  const browsers = IMPORT_SOURCES.filter((s) => s.group === 'Пароли из браузера')
  expect(browsers.map((s) => s.id)).toEqual(['chrome', 'safari', 'firefox', 'edge', 'brave'])
  for (const browser of browsers) expect(browser.steps).toHaveLength(3)
})
