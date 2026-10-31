import { createRouter, createWebHashHistory } from 'vue-router'
import { useSession } from '@/stores/session'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/tray', component: () => import('@/views/Tray.vue') },
    { path: '/restore', component: () => import('@/views/Restore.vue') },
    { path: '/onboarding', component: () => import('@/views/Onboarding.vue') },
    { path: '/unlock', component: () => import('@/views/Unlock.vue') },
    {
      path: '/',
      component: () => import('@/components/AppShell.vue'),
      children: [
        { path: '', component: () => import('@/views/Directory.vue') },
        { path: 'recent', component: () => import('@/views/Home.vue') },
        { path: 'r/:id', component: () => import('@/views/Record.vue'), props: true },
        { path: 't/:typeId', component: () => import('@/views/TypeTableView.vue'), props: true },
        { path: 'types/:typeId?', component: () => import('@/views/Types.vue'), props: true },
        { path: 'folder/:folderId', component: () => import('@/views/Directory.vue'), props: true },
        { path: 'tag/:tag', component: () => import('@/views/TagView.vue'), props: true },
        { path: 'import', component: () => import('@/views/Import.vue') },
        { path: 'secrets', component: () => import('@/views/Secrets.vue') },
        { path: 'generator', component: () => import('@/views/Generator.vue') },
        { path: 'trash', component: () => import('@/views/Trash.vue') },
        { path: 'settings', component: () => import('@/views/Settings.vue') },
      ],
    },
  ],
})

const gated = new Set(['/onboarding', '/unlock', '/restore'])

router.beforeEach(async (to) => {
  if (to.path === '/tray') return true
  const s = useSession()
  const st = s.status.state
  if (st === 'noVault' && !['/onboarding', '/restore'].includes(to.path)) return '/onboarding'
  if (st === 'locked' && to.path !== '/unlock') return '/unlock'
  if (st === 'unlocked' && gated.has(to.path)) {
    // Onboarding finishes its own last step (Touch ID / PIN) after the vault exists.
    if (to.path === '/onboarding' && to.query.step) return true
    return '/'
  }
  return true
})
