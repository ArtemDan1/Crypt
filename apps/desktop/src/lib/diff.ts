export interface DiffPart {
  op: 'same' | 'add' | 'del'
  text: string
}

/** Beyond this many LCS cells the middle is shown as a plain block replace. */
const MAX_CELLS = 4_000_000

/** Line diff from `a` (old) to `b` (new): common prefix/suffix, then LCS. */
export function diffLines(a: string, b: string): DiffPart[] {
  const x = a === '' ? [] : a.split('\n')
  const y = b === '' ? [] : b.split('\n')
  let start = 0
  while (start < x.length && start < y.length && x[start] === y[start]) start++
  let endX = x.length
  let endY = y.length
  while (endX > start && endY > start && x[endX - 1] === y[endY - 1]) {
    endX--
    endY--
  }
  const same = (lines: string[]) => lines.map((text): DiffPart => ({ op: 'same', text }))
  return [...same(x.slice(0, start)), ...middle(x.slice(start, endX), y.slice(start, endY)), ...same(x.slice(endX))]
}

function middle(x: string[], y: string[]): DiffPart[] {
  const n = x.length
  const m = y.length
  if (n * m > MAX_CELLS)
    return [...x.map((text): DiffPart => ({ op: 'del', text })), ...y.map((text): DiffPart => ({ op: 'add', text }))]
  // lcs[i][j] = LCS length of x[i..] and y[j..]
  const lcs = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1))
  for (let i = n - 1; i >= 0; i--)
    for (let j = m - 1; j >= 0; j--)
      lcs[i]![j] = x[i] === y[j] ? lcs[i + 1]![j + 1]! + 1 : Math.max(lcs[i + 1]![j]!, lcs[i]![j + 1]!)
  const out: DiffPart[] = []
  let i = 0
  let j = 0
  while (i < n && j < m) {
    if (x[i] === y[j]) {
      out.push({ op: 'same', text: x[i]! })
      i++
      j++
    } else if (lcs[i + 1]![j]! >= lcs[i]![j + 1]!) out.push({ op: 'del', text: x[i++]! })
    else out.push({ op: 'add', text: y[j++]! })
  }
  while (i < n) out.push({ op: 'del', text: x[i++]! })
  while (j < m) out.push({ op: 'add', text: y[j++]! })
  return out
}
