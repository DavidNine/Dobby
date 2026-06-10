import { renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { useCssVar } from './useCssVar'

afterEach(() => {
  document.documentElement.removeAttribute('style')
  document.documentElement.removeAttribute('data-theme')
})

describe('useCssVar', () => {
  it('returns the fallback when the variable is unset', () => {
    const { result } = renderHook(() => useCssVar('--not-defined', 'fallback'))
    expect(result.current).toBe('fallback')
  })

  it('reads an inline custom property set on <html>', () => {
    document.documentElement.style.setProperty('--color-accent', '#abcdef')
    const { result } = renderHook(() => useCssVar('--color-accent', 'x'))
    expect(result.current).toBe('#abcdef')
  })
})
