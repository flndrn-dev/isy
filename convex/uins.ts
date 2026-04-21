/**
 * UIN allocation and lookup.
 *
 * Scope A: free random allocation only. See
 * docs/superpowers/specs/2026-04-21-uin-registry-and-allocation-design.md
 * for the full data model and allocation policy.
 */

// Allowed random-draw sub-ranges, per spec §6.1:
//   [100_000_100, 700_000_000)  size 599_999_900
//   [700_000_100, 1_000_000_000) size 299_999_900
const LOW_POOL_START = 100_000_100n
const LOW_POOL_SIZE = 599_999_900n
const HIGH_POOL_START = 700_000_100n
const HIGH_POOL_SIZE = 299_999_900n
const TOTAL_POOL = LOW_POOL_SIZE + HIGH_POOL_SIZE // 899_999_800n

/**
 * Pick a candidate UIN uniformly at random from the two disjoint sub-ranges.
 * Math.random() is sufficient entropy — this is anti-clash, not cryptographic.
 */
export function pickCandidate(): bigint {
  const r = BigInt(Math.floor(Math.random() * Number(TOTAL_POOL)))
  return r < LOW_POOL_SIZE
    ? LOW_POOL_START + r
    : HIGH_POOL_START + (r - LOW_POOL_SIZE)
}
