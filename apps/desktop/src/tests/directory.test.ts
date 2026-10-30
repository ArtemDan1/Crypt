import { expect, test, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import Directory from '@/views/Directory.vue'
import { useLibrary } from '@/stores/library'

vi.mock('vue-router', () => ({ useRouter: () => ({ push: vi.fn() }) }))

test('directory lists immediate children and records and provides ancestor links', () => {
  setActivePinia(createPinia())
  const lib = useLibrary()
  lib.folders = [
    { id: 'a', name: 'Работа', parent: null },
    { id: 'b', name: 'Проекты', parent: 'a' },
    { id: 'c', name: 'Архив', parent: 'b' },
  ]
  lib.records = [
    { id: 'root', title: 'В корне', folder: null, type_id: null, tags: [], secret_kind: null, updated_at: 1 },
    { id: 'nested', title: 'В проекте', folder: 'b', type_id: null, tags: [], secret_kind: null, updated_at: 1 },
  ]
  const options = { global: { stubs: { RouterLink: { props: ['to'], template: '<a :href="to"><slot /></a>' } } } }
  const root = mount(Directory, options)
  expect(root.find('a[href="/folder/a"]').exists()).toBe(true)
  expect(root.text()).toContain('В корне')
  expect(root.text()).not.toContain('В проекте')
  expect(root.find('a[href="/folder/c"]').exists()).toBe(false)
  const nested = mount(Directory, { ...options, props: { folderId: 'b' } })
  expect(nested.get('nav').text()).toContain('/Работа/Проекты/')
  expect(nested.find('a[href="/folder/c"]').exists()).toBe(true)
  expect(nested.text()).toContain('В проекте')
  expect(nested.text()).not.toContain('В корне')
})
