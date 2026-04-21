import { mutation } from './_generated/server'
import { v } from 'convex/values'

// Dev-only: create a user row directly. Bypasses Better Auth entirely.
// DO NOT port to production — this will be removed when scope B lands.
export const createTestUser = mutation({
  args: {
    email: v.string(),
    emailVerified: v.optional(v.boolean()),
  },
  handler: async (ctx, { email, emailVerified }) => {
    const now = Date.now()
    const userId = await ctx.db.insert('users', {
      email,
      emailVerifiedAt: emailVerified === false ? undefined : now,
      passwordHash: undefined,
      recoveryEmail: undefined,
      status: 'pending',
      createdAt: now,
      updatedAt: now,
    })
    return userId
  },
})

// Dev-only: wipes scope-A's two logical tables. Same pattern as proto-mls.
// DO NOT port to production.
export const resetAllTables = mutation({
  args: {},
  handler: async (ctx) => {
    const uins = await ctx.db.query('uins').collect()
    for (const row of uins) await ctx.db.delete(row._id)
    const users = await ctx.db.query('users').collect()
    for (const row of users) await ctx.db.delete(row._id)
    return { uinsDeleted: uins.length, usersDeleted: users.length }
  },
})
