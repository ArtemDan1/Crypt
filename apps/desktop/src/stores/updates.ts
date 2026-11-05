import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { api } from '@/lib/api'
import type { UpdateInfo } from '@/lib/types'

const CHECK_PERIOD = 12 * 3600_000 // background checks at most twice a day
const RETRY_PAUSE = 30 * 60_000 // after a failed one (no network), wait a bit
const SKIPPED_KEY = 'crypt.update.skipped'
const LAST_CHECK_KEY = 'crypt.update.lastCheck'

const read = (k: string) => {
  try {
    return localStorage.getItem(k)
  } catch {
    return null
  }
}
const write = (k: string, v: string) => {
  try {
    localStorage.setItem(k, v)
  } catch {
    /* fine: just checks again next launch */
  }
}
const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

/** Updates from GitHub Releases: check → download .pkg → Installer. */
export const useUpdates = defineStore('updates', () => {
  const available = ref<UpdateInfo | null>(null)
  const checking = ref(false)
  /** 0…1 while downloading, null otherwise. */
  const progress = ref<number | null>(null)
  const installing = ref(false)
  const error = ref('')
  const dismissed = ref(false)
  const skipped = ref(read(SKIPPED_KEY))
  let lastAttempt = 0

  /** What the corner card offers: not skipped and not put off with «Позже». */
  const offer = computed(() => {
    const u = available.value
    if (!u || dismissed.value || u.version === skipped.value) return null
    return u
  })

  /** Manual check: always shows the result, even after «Позже». */
  async function check() {
    dismissed.value = false
    checking.value = true
    error.value = ''
    try {
      available.value = await api.checkUpdate()
      write(LAST_CHECK_KEY, String(Date.now()))
    } finally {
      checking.value = false
    }
    return available.value
  }

  /** On launch: quiet about errors, no network is not a breakage. */
  async function checkSilently(now = Date.now()) {
    const last = Number(read(LAST_CHECK_KEY) ?? 0)
    if (now - last < CHECK_PERIOD || now - lastAttempt < RETRY_PAUSE) return
    lastAttempt = now
    try {
      available.value = await api.checkUpdate()
      write(LAST_CHECK_KEY, String(now))
    } catch {
      /* next launch tries again */
    }
  }

  async function install() {
    const u = available.value
    if (!u || progress.value !== null || installing.value) return
    error.value = ''
    progress.value = 0
    const off = await listen<number>('update-progress', (e) => (progress.value = e.payload)).catch(() => null)
    try {
      await api.downloadUpdate(u.pkgUrl, u.size)
      progress.value = null
      installing.value = true
      // Saves edits, locks the vault, opens Installer and quits.
      await api.installUpdate()
    } catch (e) {
      error.value = message(e)
    } finally {
      off?.()
      progress.value = null
      installing.value = false
    }
  }

  function dismiss() {
    dismissed.value = true
  }
  function skip() {
    const u = available.value
    if (!u) return
    skipped.value = u.version
    write(SKIPPED_KEY, u.version)
  }

  return { available, checking, progress, installing, error, offer, check, checkSilently, install, dismiss, skip }
})
