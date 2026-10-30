import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { idleTimer } from '@/lib/idle'
import { api } from '@/lib/api'
import type { UnlockMethods, VaultStatus } from '@/lib/types'

type Hook = () => Promise<void> | void

export const useSession = defineStore('session', () => {
  const status = ref<VaultStatus>({ state: 'noVault' })
  const methods = ref<UnlockMethods | null>(null)
  const hooks = new Set<Hook>()

  async function refresh() {
    status.value = await api.status()
    methods.value = status.value.state === 'noVault' ? null : await api.unlockMethods().catch(() => null)
  }
  async function create(words: string[]) {
    status.value = await api.createVault(words)
  }
  async function unlockSeed(words: string[]) {
    status.value = await api.unlockWithSeed(words)
  }
  async function unlockBiometric() {
    status.value = await api.unlockBiometric()
  }
  async function unlockPin(pin: string) {
    try {
      status.value = await api.unlockPin(pin)
    } finally {
      methods.value = await api.unlockMethods().catch(() => methods.value)
    }
  }
  /** Register work that must finish before the vault key is dropped (pending saves). */
  function beforeLock(fn: Hook) {
    hooks.add(fn)
    return () => hooks.delete(fn)
  }
  async function flushAll() {
    await Promise.allSettled([...hooks].map((h) => h()))
  }
  async function lock() {
    if (status.value.state !== 'unlocked') return
    await flushAll()
    status.value = await api.lock()
    methods.value = await api.unlockMethods().catch(() => null)
  }
  /** Rust already locked the vault (sleep, screen lock). */
  function markLocked(vaultId: string) {
    status.value = { state: 'locked', vaultId }
    api
      .unlockMethods()
      .then((m) => (methods.value = m))
      .catch(() => {})
  }

  // Auto-lock after inactivity while unlocked.
  const idle = idleTimer(() => {
    lock()
  }, 15 * 60_000)
  const poke = () => idle.activity()
  if (typeof window !== 'undefined') {
    for (const ev of ['pointerdown', 'keydown', 'wheel'] as const)
      window.addEventListener(ev, poke, { passive: true, capture: true })
  }
  function applyAutolock(minutes: number) {
    idle.setDelay(minutes * 60_000)
  }
  watch(
    () => status.value.state,
    async (s) => {
      if (s === 'unlocked') {
        const st = await api.getSettings().catch(() => null)
        if (st) applyAutolock(st.autolockMinutes)
        idle.start()
      } else idle.stop()
    },
  )

  return {
    status,
    methods,
    refresh,
    applyAutolock,
    create,
    unlockSeed,
    unlockBiometric,
    unlockPin,
    beforeLock,
    flushAll,
    lock,
    markLocked,
  }
})
