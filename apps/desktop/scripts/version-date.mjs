// Sets the app version to today's date: 2026.11.4 (year.month.day, no leading zeros).
// Usage: pnpm version:date [YYYY.M.D]
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const d = new Date()
const version = process.argv[2] ?? `${d.getFullYear()}.${d.getMonth() + 1}.${d.getDate()}`
if (!/^\d{4}\.(?:[1-9]|1[0-2])\.(?:[1-9]|[12]\d|3[01])$/.test(version)) {
  console.error(`Неверная версия: ${version}`)
  process.exit(1)
}
const at = (p) => fileURLToPath(new URL(`../${p}`, import.meta.url))
const replace = (file, re, to) => {
  const src = readFileSync(at(file), 'utf8')
  if (!re.test(src)) throw new Error(`Версия не найдена в ${file}`)
  writeFileSync(at(file), src.replace(re, to))
}
replace('package.json', /"version": "[^"]+"/, `"version": "${version}"`)
replace('src-tauri/tauri.conf.json', /"version": "[^"]+"/, `"version": "${version}"`)
replace('src-tauri/Cargo.toml', /^version = "[^"]+"/m, `version = "${version}"`)
console.log(version)
