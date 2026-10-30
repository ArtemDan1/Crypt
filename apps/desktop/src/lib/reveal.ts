import { ref, getCurrentInstance, onBeforeUnmount, getCurrentScope, onScopeDispose, watch } from 'vue'
import { useSession } from '@/stores/session'
export function useRevealTimer(ms = 30_000) {
  const revealed = ref<string | null>(null),
    session = useSession()
  let timer: ReturnType<typeof setTimeout> | undefined
  let active = true
  function hide() {
    if (timer) clearTimeout(timer)
    timer = undefined
    revealed.value = null
  }
  function show(value: string) {
    if (!active || session.status.state !== 'unlocked') return
    hide()
    revealed.value = value
    timer = setTimeout(hide, ms)
  }
  const unhook = session.beforeLock(hide)
  const stop = watch(
    () => session.status.state,
    (state) => {
      if (state !== 'unlocked') hide()
    },
    { flush: 'sync' },
  )
  function dispose() {
    active = false
    hide()
    unhook()
    stop()
  }
  if (getCurrentInstance()) onBeforeUnmount(dispose)
  else if (getCurrentScope()) onScopeDispose(dispose)
  return { revealed, show, hide }
}
