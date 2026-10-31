import { beforeEach, expect, test } from 'vitest'
import { nextTick } from 'vue'
import { useViewPref } from '@/lib/viewPref'

beforeEach(() => localStorage.clear())

test('remembers the chosen view', async () => {
  const v = useViewPref('directory', 'list')
  expect(v.value).toBe('list')
  v.value = 'grid'
  await nextTick()
  expect(useViewPref('directory', 'list').value).toBe('grid')
})

test('garbage falls back to the initial value', () => {
  localStorage.setItem('crypt.view.directory', 'table')
  expect(useViewPref('directory', 'list').value).toBe('list')
})
