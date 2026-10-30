<script setup lang="ts">
import { ref, onBeforeUnmount } from 'vue'
import { toast } from 'vue-sonner'
import { Copy, Plus, Terminal } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { api } from '@/lib/api'
import type { SshKeyInfo } from '@/lib/types'
import { useSession } from '@/stores/session'
const props = defineProps<{ recordId: string; info: SshKeyInfo | null; enabled: boolean }>(),
  emit = defineEmits<{ saved: [] }>()
const pasting = ref(false),
  pem = ref(''),
  passphrase = ref(''),
  error = ref(''),
  busy = ref(false),
  agentRunning = ref(true)
api
  .getSettings()
  .then((s) => (agentRunning.value = s.agentEnabled))
  .catch(() => {})
function clear() {
  pem.value = ''
  passphrase.value = ''
  pasting.value = false
  error.value = ''
}
const unhook = useSession().beforeLock(clear)
onBeforeUnmount(() => {
  clear()
  unhook()
})
async function generate() {
  busy.value = true
  error.value = ''
  try {
    await api.generateSshKey(props.recordId)
    emit('saved')
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Не удалось создать ключ'
  } finally {
    busy.value = false
  }
}
async function save() {
  busy.value = true
  error.value = ''
  try {
    await api.setSshKey(props.recordId, pem.value, passphrase.value || undefined)
    clear()
    emit('saved')
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Не удалось импортировать ключ'
    passphrase.value = ''
  } finally {
    busy.value = false
  }
}
async function toggle(enabled: boolean) {
  try {
    await api.setAgentEnabled(props.recordId, enabled)
    emit('saved')
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось изменить настройку')
  }
}
async function copy() {
  try {
    await api.copySecret(props.recordId, 'public_key')
    const s = await api.getSettings()
    toast(`Скопировано, буфер очистится через ${s.clipboardClearSeconds} с`)
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось скопировать')
  }
}
</script>
<template>
  <section class="space-y-3 rounded-lg border bg-muted/10 p-4">
    <div class="flex items-center gap-2 text-sm font-medium"><Terminal class="size-4 text-secret" />SSH-ключ</div>
    <template v-if="info"
      ><div class="flex flex-wrap items-center gap-2 text-xs">
        <span class="rounded border px-2 py-1 font-mono">{{ info.algorithm }}</span
        ><span v-if="info.bits" class="text-muted-foreground">{{ info.bits }} бит</span>
      </div>
      <p class="break-all font-mono text-xs text-secret">{{ info.fingerprint }}</p>
      <div class="flex items-center gap-2">
        <code class="min-w-0 flex-1 truncate text-xs text-muted-foreground">{{ info.public_key }}</code
        ><Button variant="ghost" size="sm" aria-label="Копировать публичный ключ" @click="copy"
          ><Copy class="size-4"
        /></Button>
      </div>
      <label class="flex items-center justify-between gap-3 text-sm"
        >Доступен ssh-agent<Switch :model-value="enabled" @update:model-value="toggle"
      /></label>
      <p v-if="!agentRunning" class="text-xs text-muted-foreground">
        ssh-agent выключен в настройках приложения.
      </p></template
    >
    <div class="flex flex-wrap gap-2">
      <Button v-if="!info" variant="outline" size="sm" :disabled="busy" @click="generate"
        ><Plus class="size-3.5" />Сгенерировать ed25519</Button
      ><Button variant="ghost" size="sm" :disabled="busy" @click="pasting = !pasting">{{
        info ? 'Заменить приватный ключ' : 'Вставить приватный ключ'
      }}</Button>
    </div>
    <div v-if="pasting" class="space-y-2">
      <textarea
        v-model="pem"
        rows="6"
        class="w-full rounded-md border bg-background p-3 font-mono text-xs"
        aria-label="Приватный ключ OpenSSH или PEM"
        placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
      /><input
        v-model="passphrase"
        type="password"
        autocomplete="off"
        class="w-full rounded-md border bg-background px-3 py-2 text-sm"
        aria-label="Пароль ключа"
        placeholder="Пароль ключа, если есть"
      />
      <p class="text-xs text-muted-foreground">
        Ключ хранится внутри зашифрованного хранилища. Пароль импорта не сохраняется.
      </p>
      <div class="flex gap-2">
        <Button size="sm" :disabled="busy || !pem.trim()" @click="save">Сохранить ключ</Button
        ><Button variant="ghost" size="sm" :disabled="busy" @click="clear">Отмена</Button>
      </div>
    </div>
    <p v-if="error" class="text-sm text-destructive" role="alert">{{ error }}</p>
  </section>
</template>
