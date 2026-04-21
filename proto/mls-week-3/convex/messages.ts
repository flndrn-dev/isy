import { mutation } from './_generated/server'
import { v } from 'convex/values'

export const submitCiphertext = mutation({
  args: {
    recipientUin: v.number(),
    senderUin: v.number(),
    groupId: v.bytes(),
    ciphertext: v.bytes(),
  },
  handler: async (ctx, args) => {
    return await ctx.db.insert('messages', {
      ...args,
      createdAt: Date.now(),
      delivered: false,
    })
  },
})

export const fetchCiphertext = mutation({
  args: { recipientUin: v.number() },
  handler: async (ctx, args) => {
    const undelivered = await ctx.db
      .query('messages')
      .withIndex('by_recipient_undelivered', q =>
        q.eq('recipientUin', args.recipientUin).eq('delivered', false)
      )
      .collect()
    for (const m of undelivered) {
      await ctx.db.patch(m._id, { delivered: true })
    }
    return undelivered.map(m => ({
      senderUin: m.senderUin,
      groupId: m.groupId,
      ciphertext: m.ciphertext,
    }))
  },
})
