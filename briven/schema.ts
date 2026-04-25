/**
 * ISY schema ported from convex/schema 2.ts for the briven dogfood test.
 *
 * Notes on the port:
 * - Convex union-of-literal fields (status, class, etc.) become `text()`
 *   because @briven/schema doesn't have an enum() helper yet. Application-
 *   level validation still holds; Postgres-level enum is a later PR.
 * - v.int64() and v.number() (used for timestamps + cents) → bigint().
 * - v.id("users") → text().references("users", "id").
 * - v.optional(X) → X (no .notNull() call).
 * - Convex _creationTime auto-field → explicit created_at bigint (ms epoch)
 *   where the source table had no explicit created/updated fields.
 */
import { bigint, boolean, schema, table, text } from '@briven/cli/schema';

export default schema({
  users: table({
    columns: {
      id: text().primaryKey(),
      email: text().notNull(),
      emailVerifiedAt: bigint(),
      passwordHash: text(),
      recoveryEmail: text(),
      // Convex literal union: pending | active | suspended | deleted
      status: text().notNull(),
      createdAt: bigint().notNull(),
      updatedAt: bigint().notNull(),
    },
    indexes: [{ columns: ['email'], unique: true }],
  }),

  uins: table({
    columns: {
      id: text().primaryKey(),
      uin: bigint().notNull(),
      ownerId: text().references('users', 'id'),
      // Literal union: standard | short | memorable | palindrome | sequential | ultra-rare | commemorative
      class: text().notNull(),
      // Literal union: available | owned | reserved | retired | canary
      status: text().notNull(),
      isPrimary: boolean().notNull(),
      acquiredAt: bigint(),
      acquisitionTransactionId: text(),
      retiredAt: bigint(),
      allocatedAt: bigint().notNull(),
    },
    indexes: [
      { columns: ['uin'], unique: true },
      { columns: ['ownerId'] },
      { columns: ['status', 'class'] },
      { columns: ['ownerId', 'isPrimary'] },
    ],
  }),

  marketplace_listings: table({
    columns: {
      id: text().primaryKey(),
      uin: bigint().notNull(),
      // Literal union: short | memorable | palindrome | sequential | ultra-rare
      class: text().notNull(),
      // Literal union: primary | secondary
      listingType: text().notNull(),
      sellerId: text().references('users', 'id'),
      priceCents: bigint().notNull(),
      // v.literal('EUR') — stored as text, length-constrained at app layer
      currency: text().notNull().default("'EUR'"),
      // Literal union: active | sold | expired | delisted
      status: text().notNull(),
      createdAt: bigint().notNull(),
      expiresAt: bigint(),
      soldAt: bigint(),
      soldToId: text().references('users', 'id'),
    },
    indexes: [
      { columns: ['uin'] },
      { columns: ['status', 'class'] },
      { columns: ['sellerId'] },
    ],
  }),

  uin_transactions: table({
    columns: {
      id: text().primaryKey(),
      uin: bigint().notNull(),
      // Literal union: primary_sale | secondary_sale | auction_sale | retirement
      transactionType: text().notNull(),
      listingId: text().references('marketplace_listings', 'id'),
      sellerId: text().references('users', 'id'),
      buyerId: text().references('users', 'id'),
      grossCents: bigint().notNull(),
      flndrnCommissionCents: bigint().notNull(),
      paymentProcessorFeeCents: bigint().notNull(),
      sellerNetCents: bigint().notNull(),
      polarPaymentId: text(),
      maviPayTransactionId: text(),
      completedAt: bigint().notNull(),
    },
    indexes: [
      { columns: ['uin'] },
      { columns: ['buyerId'] },
      { columns: ['sellerId'] },
      { columns: ['completedAt'] },
    ],
  }),

  user_balances: table({
    columns: {
      id: text().primaryKey(),
      userId: text().notNull().references('users', 'id'),
      balanceCents: bigint().notNull(),
      // v.literal('EUR')
      currency: text().notNull().default("'EUR'"),
      lastPayoutAt: bigint(),
    },
    indexes: [{ columns: ['userId'] }],
  }),
});
