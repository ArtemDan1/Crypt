import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api } from '@/lib/api'
import type { Folder, Id, NewRecord, RecordSummary, TypeDef } from '@/lib/types'

export const useLibrary = defineStore('library', () => {
  const name = ref('')
  const types = ref<TypeDef[]>([])
  const folders = ref<Folder[]>([])
  const tags = ref<[string, number][]>([])
  const records = ref<RecordSummary[]>([])
  const trashCount = ref(0)
  /** Bumped on every change so open views can refetch. */
  const version = ref(0)

  const typeById = computed(() => new Map(types.value.map((t) => [t.id, t])))
  const titleById = computed(() => new Map(records.value.map((r) => [r.id, r.title])))
  const countByType = computed(() => {
    const m = new Map<Id, number>()
    for (const r of records.value) if (r.type_id) m.set(r.type_id, (m.get(r.type_id) ?? 0) + 1)
    return m
  })
  const countByFolder = computed(() => {
    const m = new Map<Id, number>()
    for (const r of records.value) if (r.folder) m.set(r.folder, (m.get(r.folder) ?? 0) + 1)
    return m
  })
  const secretsCount = computed(() => records.value.filter((r) => r.secret_kind).length)

  async function refreshAll() {
    const [n, t, f, g, r, trash] = await Promise.all([
      api.vaultName(),
      api.listTypes(),
      api.listFolders(),
      api.allTags(),
      api.listRecords({}),
      api.listRecords({ trashed: true }),
    ])
    name.value = n
    types.value = t
    folders.value = f
    tags.value = g
    records.value = r
    trashCount.value = trash.length
    version.value++
  }

  /** Call after any mutation; cheap enough at this scale. */
  async function changed() {
    await refreshAll()
  }

  async function createRecord(rec: Partial<NewRecord> = {}): Promise<Id> {
    const id = await api.createRecord({
      title: rec.title ?? '',
      type_id: rec.type_id ?? null,
      folder: rec.folder ?? null,
      secret_kind: null,
    })
    await changed()
    return id
  }

  async function rename(next: string) {
    await api.renameVault(next)
    name.value = await api.vaultName()
  }

  function clear() {
    name.value = ''
    types.value = []
    folders.value = []
    tags.value = []
    records.value = []
    trashCount.value = 0
  }

  return {
    name,
    rename,
    types,
    folders,
    tags,
    records,
    trashCount,
    version,
    typeById,
    titleById,
    countByType,
    countByFolder,
    secretsCount,
    refreshAll,
    changed,
    createRecord,
    clear,
  }
})
