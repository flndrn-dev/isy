import { mutation } from './_generated/server'
import { v } from 'convex/values'

export const publishKeyPackage = mutation({
  args: { uin: v.number(), keyPackageBytes: v.bytes() },
  handler: async (ctx, args) => {
    return await ctx.db.insert('keyPackages', {
      ...args,
      publishedAt: Date.now(),
      consumed: false,
    })
  },
})

export const fetchKeyPackage = mutation({
  // Mutation, not query, because it marks consumed=true atomically
  args: { uin: v.number() },
  handler: async (ctx, args) => {
    const kp = await ctx.db
      .query('keyPackages')
      .withIndex('by_uin_unconsumed', q =>
        q.eq('uin', args.uin).eq('consumed', false)
      )
      .first()
    if (!kp) throw new Error('No KeyPackage available for UIN')
    await ctx.db.patch(kp._id, { consumed: true })
    return kp.keyPackageBytes
  },
})
