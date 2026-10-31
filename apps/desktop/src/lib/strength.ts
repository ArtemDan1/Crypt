export type Strength = { score: 0 | 1 | 2 | 3; label: 'Слабый' | 'Средний' | 'Надёжный' | 'Очень надёжный' }

const LABELS = ['Слабый', 'Средний', 'Надёжный', 'Очень надёжный'] as const

/** Rough strength from length and character classes; shown only for values already on screen. */
export function passwordStrength(value: string): Strength {
  const len = [...value].length
  const classes = [/[a-zа-яё]/, /[A-ZА-ЯЁ]/, /[0-9]/, /[^a-zA-Zа-яёА-ЯЁ0-9]/].filter((r) => r.test(value)).length
  let score: Strength['score'] = 0
  if ((len >= 20 && classes >= 3) || len >= 28) score = 3
  else if (len >= 14 && classes >= 3) score = 2
  else if (len >= 10) score = 1
  return { score, label: LABELS[score] }
}
