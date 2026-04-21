/**
 * UIN allocation and lookup.
 *
 * Scope A: free random allocation only. See
 * docs/superpowers/specs/2026-04-21-uin-registry-and-allocation-design.md
 * for the full data model and allocation policy.
 */

import { mutation } from './_generated/server'
import { v } from 'convex/values'

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

/**
 * Allocate a random standard-class UIN to a verified user.
 *
 * Preconditions (throws on violation):
 *   - user exists
 *   - user.emailVerifiedAt is set (non-null)
 *   - user does not already have a primary UIN
 *
 * On success:
 *   - Inserts a new 'uins' row with status='owned', class='standard', isPrimary=true
 *   - Patches users[userId] with status='active' and updatedAt
 *   - Returns the allocated UIN as bigint
 *
 * Atomicity: Convex mutations are transactional end-to-end; the collision check
 * and insert happen inside one transaction. No locks required.
 */
export const allocate = mutation({
  args: { userId: v.id('users') },
  handler: async (ctx, { userId }) => {
    const user = await ctx.db.get(userId)
    if (!user) throw new Error('user not found')
    if (user.emailVerifiedAt == null) throw new Error('email not verified')

    // Idempotency: has this user already got a primary UIN?
    // Casts to `any` are required while `_generated/` is stale (Doc = any,
    // so withIndex's type signature only knows system indexes). Remove when
    // Convex dev regenerates `_generated/` from our real schema.
    const existingPrimary = await (ctx.db.query('uins') as any)
      .withIndex('by_owner_primary', (q: any) =>
        q.eq('ownerId', userId).eq('isPrimary', true)
      )
      .first()
    if (existingPrimary) throw new Error('user already has a UIN')

    const MAX_ATTEMPTS = 20
    for (let i = 0; i < MAX_ATTEMPTS; i++) {
      const candidate = pickCandidate()
      const taken = await (ctx.db.query('uins') as any)
        .withIndex('by_uin', (q: any) => q.eq('uin', candidate))
        .first()
      if (taken) continue

      const now = Date.now()
      await ctx.db.insert('uins', {
        uin: candidate,
        ownerId: userId,
        class: 'standard',
        status: 'owned',
        isPrimary: true,
        acquiredAt: now,
        acquisitionTransactionId: undefined,
        retiredAt: undefined,
        allocatedAt: now,
      })
      await ctx.db.patch(userId, { status: 'active', updatedAt: now })
      return candidate
    }
    throw new Error('UIN allocation failed after 20 attempts — pool may be exhausted')
  },
})
