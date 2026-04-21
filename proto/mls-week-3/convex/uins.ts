import { mutation, query } from './_generated/server'
import { v } from 'convex/values'

export const registerUin = mutation({
  args: {
    uin: v.number(),
    credentialBytes: v.bytes(),
    publicSignatureKey: v.bytes(),
  },
  handler: async (ctx, args) => {
    // Allocation collision check
    const existing = await ctx.db
      .query('uins')
      .withIndex('by_uin', q => q.eq('uin', args.uin))
      .first()
    if (existing) throw new Error('UIN already taken')
    if (args.uin < 100_000_000 || args.uin > 999_999_999)
      throw new Error('UIN out of valid range')
    if (args.uin >= 700_000_000 && args.uin <= 700_000_099)
      throw new Error('UIN reserved for canary use')
    return await ctx.db.insert('uins', { ...args, createdAt: Date.now() })
  },
})

export const lookupUin = query({
  args: { uin: v.number() },
  handler: async (ctx, args) => {
    return await ctx.db
      .query('uins')
      .withIndex('by_uin', q => q.eq('uin', args.uin))
      .first()
  },
})
