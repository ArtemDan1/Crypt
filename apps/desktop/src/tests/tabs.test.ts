import { beforeEach, expect, test, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createMemoryHistory, createRouter } from 'vue-router'
import { useTabs } from '@/stores/tabs'

const openWindow = vi.fn()
vi.mock('@/lib/api', () => ({ api: { openWindow: (p: string) => openWindow(p) } }))

const Page = { template: '<div />' }
async function setup(start = '/', opts: { persist?: boolean; onLastClosed?: () => void } = {}) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/unlock', component: Page },
      {
        path: '/',
        component: Page,
        children: [
          { path: '', component: Page },
          { path: 'r/:id', component: Page },
          { path: 'folder/:id', component: Page },
        ],
      },
    ],
  })
  await router.push(start)
  const tabs = useTabs()
  tabs.attach(router, { persist: opts.persist ?? false, onLastClosed: opts.onLastClosed })
  await router.isReady()
  const path = () => router.currentRoute.value.fullPath
  const settle = () => new Promise((r) => setTimeout(r))
  return { router, tabs, path, settle }
}

beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
  openWindow.mockReset()
})

test('each tab keeps its own back/forward history', async () => {
  const { router, tabs, path, settle } = await setup()
  await router.push('/r/a')
  await router.push('/r/b')
  expect(tabs.canBack).toBe(true)
  tabs.back()
  await settle()
  expect(path()).toBe('/r/a')
  expect(tabs.canForward).toBe(true)

  const first = tabs.activeId
  tabs.open('/folder/x')
  await settle()
  expect(path()).toBe('/folder/x')
  expect(tabs.canBack).toBe(false)

  tabs.activate(first)
  await settle()
  expect(path()).toBe('/r/a')
  tabs.forward()
  await settle()
  expect(path()).toBe('/r/b')
  // Switching and back/forward did not add history steps.
  expect(tabs.active!.stack).toEqual(['/', '/r/a', '/r/b'])
})

test('a new navigation after going back drops the forward entries', async () => {
  const { router, tabs, settle } = await setup()
  await router.push('/r/a')
  await router.push('/r/b')
  tabs.back()
  await settle()
  await router.push('/r/c')
  expect(tabs.active!.stack).toEqual(['/', '/r/a', '/r/c'])
  expect(tabs.canForward).toBe(false)
})

test('pages outside the app shell are not recorded', async () => {
  const { router, tabs } = await setup('/r/a')
  await router.push('/unlock')
  expect(tabs.active!.stack).toEqual(['/r/a'])
})

test('closing the active tab shows its neighbour; the last tab resets or closes the window', async () => {
  const onLastClosed = vi.fn()
  const { tabs, path, settle } = await setup('/r/a', { onLastClosed })
  const a = tabs.activeId
  const b = tabs.open('/r/b')
  tabs.open('/r/c', { background: true })
  expect(tabs.tabs.map((t) => tabs.pathOf(t))).toEqual(['/r/a', '/r/b', '/r/c'])
  tabs.close(b)
  await settle()
  expect(path()).toBe('/r/c')
  tabs.closeOthers(a)
  await settle()
  expect(tabs.tabs).toHaveLength(1)
  expect(path()).toBe('/r/a')
  tabs.close(a)
  expect(onLastClosed).toHaveBeenCalled()
})

test('main window: last tab turns into an empty tab; tabs survive a restart', async () => {
  const first = await setup('/r/a', { persist: true })
  first.tabs.open('/folder/x')
  await first.settle()
  first.tabs.close(first.tabs.activeId)
  await first.settle()
  first.tabs.close(first.tabs.activeId)
  await first.settle()
  expect(first.path()).toBe('/')
  first.tabs.open('/r/z')
  await first.settle()

  setActivePinia(createPinia())
  const again = await setup('/', { persist: true })
  await again.settle()
  expect(again.tabs.tabs.map((t) => again.tabs.pathOf(t))).toEqual(['/', '/r/z'])
  expect(again.path()).toBe('/r/z')
})

test('broken saved tabs are ignored', async () => {
  localStorage.setItem('crypt.tabs', JSON.stringify({ tabs: [{ id: 1, stack: ['http://x'] }] }))
  const { tabs } = await setup('/r/a', { persist: true })
  expect(tabs.tabs.map((t) => tabs.pathOf(t))).toEqual(['/r/a'])
})

test('links follow modifiers: plain, ⌘ new tab, ⌘⌥ new window', async () => {
  const { tabs, path, settle } = await setup()
  await tabs.follow('/r/a')
  expect(tabs.tabs).toHaveLength(1)
  expect(path()).toBe('/r/a')
  tabs.follow('/r/b', { metaKey: true, altKey: false })
  await settle()
  expect(tabs.tabs).toHaveLength(2)
  expect(path()).toBe('/r/b')
  await tabs.follow('/r/c', { metaKey: true, altKey: true })
  expect(openWindow).toHaveBeenCalledWith('/r/c')
  expect(tabs.tabs).toHaveLength(2)
})

test('tabs can be reordered and picked by number', async () => {
  const { tabs, path, settle } = await setup('/r/a')
  const a = tabs.activeId
  tabs.open('/r/b')
  tabs.open('/r/c')
  tabs.move(a, 2)
  expect(tabs.tabs.map((t) => tabs.pathOf(t))).toEqual(['/r/b', '/r/c', '/r/a'])
  tabs.select(9)
  await settle()
  expect(path()).toBe('/r/a')
  tabs.cycle(1)
  await settle()
  expect(path()).toBe('/r/b')
})
