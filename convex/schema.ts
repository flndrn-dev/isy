import { defineSchema, defineTable } from 'convex/server'
import { v } from 'convex/values'

export default defineSchema({
  // ── Users ────────────────────────────────────────────────────────────────
  // Schema-only in scope A. Scope B (Better Auth) adds mutations against this.
  users: defineTable({
    email: v.string(),
    emailVerifiedAt: v.optional(v.number()),
    passwordHash: v.optional(v.string()),
    recoveryEmail: v.optional(v.string()),
    status: v.union(
      v.literal('pending'),
      v.literal('active'),
      v.literal('suspended'),
      v.literal('deleted'),
    ),
    createdAt: v.number(),
    updatedAt: v.number(),
  }).index('by_email', ['email']),

  // ── UINs ─────────────────────────────────────────────────────────────────
  // The core table. Logic in scope A (allocation).
  uins: defineTable({
    uin: v.int64(),
    ownerId: v.optional(v.id('users')),
    class: v.union(
      v.literal('standard'),
      v.literal('short'),
      v.literal('memorable'),
      v.literal('palindrome'),
      v.literal('sequential'),
      v.literal('ultra-rare'),
      v.literal('commemorative'),
    ),
    status: v.union(
      v.literal('available'),
      v.literal('owned'),
      v.literal('reserved'),
      v.literal('retired'),
      v.literal('canary'),
    ),
    isPrimary: v.boolean(),
    acquiredAt: v.optional(v.number()),
    acquisitionTransactionId: v.optional(v.id('uin_transactions')),
    retiredAt: v.optional(v.number()),
    allocatedAt: v.number(),
  })
    .index('by_uin', ['uin'])
    .index('by_owner', ['ownerId'])
    .index('by_status_class', ['status', 'class'])
    .index('by_owner_primary', ['ownerId', 'isPrimary']),

  // ── Marketplace listings ─────────────────────────────────────────────────
  // Schema-only scaffold for Phase 2.
  marketplace_listings: defineTable({
    uin: v.int64(),
    class: v.union(
      v.literal('short'),
      v.literal('memorable'),
      v.literal('palindrome'),
      v.literal('sequential'),
      v.literal('ultra-rare'),
    ),
    listingType: v.union(v.literal('primary'), v.literal('secondary')),
    sellerId: v.optional(v.id('users')),
    priceCents: v.int64(),
    currency: v.literal('EUR'),
    status: v.union(
      v.literal('active'),
      v.literal('sold'),
      v.literal('expired'),
      v.literal('delisted'),
    ),
    createdAt: v.number(),
    expiresAt: v.optional(v.number()),
    soldAt: v.optional(v.number()),
    soldToId: v.optional(v.id('users')),
  })
    .index('by_uin', ['uin'])
    .index('by_status_class', ['status', 'class'])
    .index('by_seller', ['sellerId']),

  // ── UIN transactions (append-only audit log) ─────────────────────────────
  // Schema-only scaffold for Phase 2. 7-year retention per Cyprus tax law.
  uin_transactions: defineTable({
    uin: v.int64(),
    transactionType: v.union(
      v.literal('primary_sale'),
      v.literal('secondary_sale'),
      v.literal('auction_sale'),
      v.literal('retirement'),
    ),
    listingId: v.optional(v.id('marketplace_listings')),
    sellerId: v.optional(v.id('users')),
    buyerId: v.optional(v.id('users')),
    grossCents: v.int64(),
    flndrnCommissionCents: v.int64(),
    paymentProcessorFeeCents: v.int64(),
    sellerNetCents: v.int64(),
    polarPaymentId: v.optional(v.string()),
    maviPayTransactionId: v.optional(v.string()),
    completedAt: v.number(),
  })
    .index('by_uin', ['uin'])
    .index('by_buyer', ['buyerId'])
    .index('by_seller', ['sellerId'])
    .index('by_completedAt', ['completedAt']),

  // ── User balances (secondary-market seller payouts) ──────────────────────
  // Schema-only scaffold for Phase 2.
  user_balances: defineTable({
    userId: v.id('users'),
    balanceCents: v.int64(),
    currency: v.literal('EUR'),
    lastPayoutAt: v.optional(v.number()),
  }).index('by_user', ['userId']),
})
