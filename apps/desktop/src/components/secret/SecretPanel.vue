<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount } from 'vue'
import { KeyRound } from '@lucide/vue'
import { toast } from 'vue-sonner'
import SecretValue from './SecretValue.vue'
import SshKeySection from './SshKeySection.vue'
import TotpCode from './TotpCode.vue'
import { api } from '@/lib/api'
import type { SecretOverview } from '@/lib/types'
import { SECRET_FIELDS, SECRET_KIND_LABEL } from '@/lib/secretFields'
import { useLibrary } from '@/stores/library'
const props = defineProps<{ recordId: string }>(),
  overview = ref<SecretOverview | null>(null),
  lib = useLibrary()
let active = true,
  request = 0
const fields = computed(() =>
  overview.value
    ? SECRET_FIELDS[overview.value.kind].filter(
        (f) => overview.value?.kind !== 'sshkey' || !['public_key', 'passphrase'].includes(f.name),
      )
    : [],
)
async function load() {
  const n = ++request
  try {
    const value = await api.secretOverview(props.recordId)
    if (active && n === request) overview.value = value
  } catch (e) {
    toast.error(e instanceof Error ? e.message : 'Не удалось загрузить секрет')
  }
}
async function saved() {
  await load()
  await lib.changed()
}
watch(
  () => props.recordId,
  () => {
    overview.value = null
    load()
  },
  { immediate: true },
)
onBeforeUnmount(() => {
  active = false
  ++request
  overview.value = null
})
</script>
<template>
  <section v-if="overview" class="my-5 rounded-xl border bg-card p-5">
    <h2 class="mb-3 flex items-center gap-2 text-sm font-medium">
      <KeyRound class="size-4 text-secret" />{{ SECRET_KIND_LABEL[overview.kind] }}
    </h2>
    <SshKeySection
      v-if="overview.kind === 'sshkey'"
      :record-id="recordId"
      :info="overview.ssh"
      :enabled="overview.agentEnabled"
      @saved="saved"
    /><SecretValue
      v-for="meta in fields"
      :key="`${recordId}:${meta.name}`"
      :record-id="recordId"
      :meta="meta"
      :filled="overview.filled.includes(meta.name)"
      :public-value="overview.public[meta.name]"
      :history-count="overview.historyCounts[meta.name] ?? 0"
      :read-only="meta.name === 'private_key'"
      @saved="saved"
    /><TotpCode v-if="overview.hasTotp" :record-id="recordId" />
  </section>
</template>
