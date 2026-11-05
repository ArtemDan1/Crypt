// macOS WebKit capitalises and autocorrects text fields ("root" → "Root").
// Fields here hold logins, hosts and keys, so that is off for every input.
const ATTRS: [string, string][] = [
  ['autocapitalize', 'off'],
  ['autocorrect', 'off'],
  ['spellcheck', 'false'],
  ['writingsuggestions', 'false'],
]

/** A field can still opt back in by setting the attribute itself. */
export function plainTyping(el: Element) {
  if (!(el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement)) return
  for (const [name, value] of ATTRS) if (!el.hasAttribute(name)) el.setAttribute(name, value)
}

export function watchPlainTyping(root: Element = document.documentElement) {
  root.querySelectorAll('input, textarea').forEach(plainTyping)
  const observer = new MutationObserver((changes) => {
    for (const c of changes)
      for (const n of c.addedNodes)
        if (n instanceof Element) {
          plainTyping(n)
          n.querySelectorAll('input, textarea').forEach(plainTyping)
        }
  })
  observer.observe(root, { childList: true, subtree: true })
  return () => observer.disconnect()
}
