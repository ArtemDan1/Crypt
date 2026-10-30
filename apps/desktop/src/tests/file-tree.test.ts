import { expect, test, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import FolderTree from '@/components/FolderTree.vue'
import { useLibrary } from '@/stores/library'

vi.mock('@/lib/api', () => ({ api: {}, CryptApiError: class extends Error {} }))
const stubs = { RouterLink: { props: ['to'], template: '<a :href="to"><slot /></a>' } }
function library() {
  setActivePinia(createPinia())
  const lib = useLibrary()
  lib.folders = [
    { id: 'a', name: 'Работа', parent: null },
    { id: 'b', name: 'Проекты', parent: 'a' },
  ]
  lib.records = [
    { id: 'root', title: 'В корне', folder: null, type_id: null, tags: [], secret_kind: null, updated_at: 1 },
    { id: 'nested', title: 'В проекте', folder: 'b', type_id: null, tags: [], secret_kind: null, updated_at: 1 },
    { id: 'secret', title: 'Секрет', folder: null, type_id: null, tags: [], secret_kind: 'login', updated_at: 1 },
  ]
}
test('tree lists root files and expands directories to nested files', async () => {
  library()
  const w = mount(FolderTree, { props: { parent: null, depth: 0 }, global: { stubs } })
  expect(w.find('a[href="/r/root"]').exists()).toBe(true)
  expect(w.find('a[href="/r/secret"]').exists()).toBe(false)
  expect(w.find('a[href="/r/nested"]').exists()).toBe(false)
  await w.get('button[aria-label="Развернуть Работа"]').trigger('click')
  await w.get('button[aria-label="Развернуть Проекты"]').trigger('click')
  expect(w.get('a[href="/r/nested"]').text()).toBe('В проекте')
})
test('active file opens its ancestors and receives current-page indication', () => {
  library()
  const w = mount(FolderTree, {
    props: { parent: null, depth: 0, activeId: 'b', activeRecordId: 'nested' },
    global: { stubs },
  })
  expect(w.get('a[href="/r/nested"]').attributes('aria-current')).toBe('page')
})
