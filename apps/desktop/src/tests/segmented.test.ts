import { expect, test } from 'vitest'
import { mount } from '@vue/test-utils'
import SegmentedControl from '@/components/crypt/SegmentedControl.vue'

const options = [
  { value: 'list', label: 'Списком' },
  { value: 'grid', label: 'Карточками' },
]

test('clicking an option emits its value', async () => {
  const w = mount(SegmentedControl, { props: { modelValue: 'list', options, label: 'Вид' } })
  await w.findAll('button')[1]!.trigger('click')
  expect(w.emitted('update:modelValue')).toEqual([['grid']])
})

test('the active option is pressed', () => {
  const w = mount(SegmentedControl, { props: { modelValue: 'grid', options, label: 'Вид' } })
  const [a, b] = w.findAll('button')
  expect(a!.attributes('aria-pressed')).toBe('false')
  expect(b!.attributes('aria-pressed')).toBe('true')
  expect(w.attributes('aria-label')).toBe('Вид')
})
