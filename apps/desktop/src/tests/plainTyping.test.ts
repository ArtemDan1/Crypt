import { expect, test } from 'vitest'
import { watchPlainTyping } from '@/lib/plainTyping'

test('inputs added later get no auto-capitalisation, but explicit choices stay', async () => {
  const root = document.createElement('div')
  document.body.append(root)
  const stop = watchPlainTyping(root)
  root.innerHTML = '<form><input id="a"><textarea id="b"></textarea><input id="c" spellcheck="true"></form>'
  await Promise.resolve()
  const a = root.querySelector('#a')!
  expect(a.getAttribute('autocapitalize')).toBe('off')
  expect(a.getAttribute('autocorrect')).toBe('off')
  expect(root.querySelector('#b')!.getAttribute('spellcheck')).toBe('false')
  expect(root.querySelector('#c')!.getAttribute('spellcheck')).toBe('true')
  stop()
})
