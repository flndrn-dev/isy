import { describe, expect, test } from 'vitest'
import { pickCandidate } from './uins'

describe('pickCandidate', () => {
  test('returns a bigint', () => {
    const x = pickCandidate()
    expect(typeof x).toBe('bigint')
  })

  test('never returns a value in the flndrn-internal range [100_000_000, 100_000_099]', () => {
    for (let i = 0; i < 10_000; i++) {
      const x = pickCandidate()
      expect(x >= 100_000_100n).toBe(true)
    }
  })

  test('never returns a value in the canary range [700_000_000, 700_000_099]', () => {
    for (let i = 0; i < 10_000; i++) {
      const x = pickCandidate()
      const inCanary = x >= 700_000_000n && x <= 700_000_099n
      expect(inCanary).toBe(false)
    }
  })

  test('never returns a value >= 1_000_000_000 or < 100_000_100', () => {
    for (let i = 0; i < 10_000; i++) {
      const x = pickCandidate()
      expect(x >= 100_000_100n && x <= 999_999_999n).toBe(true)
    }
  })

  test('produces values in both sub-ranges over many samples', () => {
    // Statistical check: over 10k samples, both sub-ranges should be hit.
    let hitLow = false
    let hitHigh = false
    for (let i = 0; i < 10_000; i++) {
      const x = pickCandidate()
      if (x < 700_000_000n) hitLow = true
      else if (x > 700_000_099n) hitHigh = true
      if (hitLow && hitHigh) break
    }
    expect(hitLow).toBe(true)
    expect(hitHigh).toBe(true)
  })
})
