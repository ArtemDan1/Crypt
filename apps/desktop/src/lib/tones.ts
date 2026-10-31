import type { TileTone } from '@/components/crypt/KindTile.vue'
import type { TypeDef } from '@/lib/types'

const TONES: TileTone[] = ['primary', 'sand', 'slate', 'rose', 'plum']

/** Stable tile colour for a record type: by its position in the type list. */
export function toneForType(types: TypeDef[], typeId: string | null | undefined): TileTone {
  if (!typeId) return 'neutral'
  const i = types.findIndex((t) => t.id === typeId)
  return i < 0 ? 'neutral' : TONES[i % TONES.length]!
}
