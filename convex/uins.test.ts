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

// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for uins.allocate, uins.lookupByUin, uins.lookupPrimaryByOwner,
// uins.poolStats.
//
// These are SKIPPED in the current degraded mode: J is locked out of Convex
// dashboard, so `_generated/` is a stale placeholder (Doc = any) that does not
// expose our real function surface to convex-test. Once Convex access is
// restored and `npx convex dev` regenerates `_generated/` against our real
// schema.ts, change `describe.skip(...)` to `describe(...)` and these will run.
// ─────────────────────────────────────────────────────────────────────────────
import { convexTest } from 'convex-test'
import schema from './schema'
// The stale `_generated/api.d.ts` types `api` as `{}` until `npx convex dev`
// regenerates it. We cast to `any` at every reference below. When Convex access
// is restored and codegen regenerates the real API surface, delete the casts.
import { api as _api } from './_generated/api'
const api = _api as any

describe.skip('uins.allocate [requires Convex codegen]', () => {
  test('happy path: allocates a UIN to a verified user', async () => {
    const t = convexTest(schema)
    const userId = await t.mutation(api.dev.createTestUser, { email: 'alice@example.com' })
    const uin = await t.mutation(api.uins.allocate, { userId })

    expect(typeof uin).toBe('bigint')
    expect(uin >= 100_000_100n && uin <= 999_999_999n).toBe(true)

    const row = await t.run(async (ctx) => {
      return await ctx.db
        .query('uins')
        .withIndex('by_uin', (q: any) => q.eq('uin', uin))
        .first()
    })
    expect(row).not.toBeNull()
    expect(row!.class).toBe('standard')
    expect(row!.status).toBe('owned')
    expect(row!.isPrimary).toBe(true)
    expect(row!.ownerId).toBe(userId)
    expect(row!.acquisitionTransactionId).toBeUndefined()
  })

  test('marks the user status as active after allocation', async () => {
    const t = convexTest(schema)
    const userId = await t.mutation(api.dev.createTestUser, { email: 'b@example.com' })
    await t.mutation(api.uins.allocate, { userId })

    const user: any = await t.run(async (ctx) => await ctx.db.get(userId))
    expect(user!.status).toBe('active')
  })

  test('rejects allocation for an unverified user', async () => {
    const t = convexTest(schema)
    const userId = await t.mutation(api.dev.createTestUser, {
      email: 'c@example.com',
      emailVerified: false,
    })

    await expect(
      t.mutation(api.uins.allocate, { userId })
    ).rejects.toThrow(/email not verified/)
  })

  test('rejects double allocation for the same user', async () => {
    const t = convexTest(schema)
    const userId = await t.mutation(api.dev.createTestUser, { email: 'd@example.com' })
    await t.mutation(api.uins.allocate, { userId })

    await expect(
      t.mutation(api.uins.allocate, { userId })
    ).rejects.toThrow(/already has a UIN/)
  })

  test('rejects allocation for a missing user', async () => {
    const t = convexTest(schema)
    const bogusUserId = 'jd700000000000000000000000000000' as any
    await expect(
      t.mutation(api.uins.allocate, { userId: bogusUserId })
    ).rejects.toThrow()
  })

  test('avoids a pre-seeded available UIN (collision retry)', async () => {
    const t = convexTest(schema)
    const reservedUin = 500_000_000n
    await t.run(async (ctx) => {
      await ctx.db.insert('uins', {
        uin: reservedUin,
        ownerId: undefined,
        class: 'memorable',
        status: 'available',
        isPrimary: false,
        allocatedAt: Date.now(),
      })
    })
    for (let i = 0; i < 20; i++) {
      const userId = await t.mutation(api.dev.createTestUser, { email: `u${i}@example.com` })
      const uin = await t.mutation(api.uins.allocate, { userId })
      expect(uin).not.toBe(reservedUin)
    }
  })

  test('produces 100 distinct UINs across 100 allocations', async () => {
    const t = convexTest(schema)
    const uins = new Set<bigint>()
    for (let i = 0; i < 100; i++) {
      const userId = await t.mutation(api.dev.createTestUser, { email: `user${i}@example.com` })
      const uin = await t.mutation(api.uins.allocate, { userId })
      uins.add(uin)
    }
    expect(uins.size).toBe(100)
  })
})

describe.skip('uins.lookupByUin [requires Convex codegen]', () => {
  test('returns the uin row when it exists', async () => {
    const t = convexTest(schema)
    const userId = await t.mutation(api.dev.createTestUser, { email: 'e@example.com' })
    const uin = await t.mutation(api.uins.allocate, { userId })

    const row = await t.query(api.uins.lookupByUin, { uin })
    expect(row).not.toBeNull()
    expect(row!.uin).toBe(uin)
    expect(row!.ownerId).toBe(userId)
  })

  test('returns null for a nonexistent uin', async () => {
    const t = convexTest(schema)
    const row = await t.query(api.uins.lookupByUin, { uin: 123_456_789n })
    expect(row).toBeNull()
  })
})

describe.skip('uins.lookupPrimaryByOwner [requires Convex codegen]', () => {
  test('returns the primary uin row for a user', async () => {
    const t = convexTest(schema)
    const userId = await t.mutation(api.dev.createTestUser, { email: 'f@example.com' })
    const uin = await t.mutation(api.uins.allocate, { userId })

    const row = await t.query(api.uins.lookupPrimaryByOwner, { userId })
    expect(row).not.toBeNull()
    expect(row!.uin).toBe(uin)
    expect(row!.isPrimary).toBe(true)
  })

  test('returns null if the user has no primary uin', async () => {
    const t = convexTest(schema)
    const userId = await t.mutation(api.dev.createTestUser, { email: 'g@example.com' })

    const row = await t.query(api.uins.lookupPrimaryByOwner, { userId })
    expect(row).toBeNull()
  })
})

describe.skip('uins.poolStats [requires Convex codegen]', () => {
  test('returns zero counts on an empty database', async () => {
    const t = convexTest(schema)
    const stats = await t.query(api.uins.poolStats, {})
    expect(stats.total).toBe(0)
    expect(stats.owned).toBe(0)
    expect(stats.available).toBe(0)
    expect(stats.canary).toBe(0)
    expect(stats.retired).toBe(0)
    expect(stats.reserved).toBe(0)
  })

  test('counts allocated UINs as owned', async () => {
    const t = convexTest(schema)
    for (let i = 0; i < 3; i++) {
      const userId = await t.mutation(api.dev.createTestUser, { email: `h${i}@example.com` })
      await t.mutation(api.uins.allocate, { userId })
    }
    const stats = await t.query(api.uins.poolStats, {})
    expect(stats.total).toBe(3)
    expect(stats.owned).toBe(3)
  })
})
