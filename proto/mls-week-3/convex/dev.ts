import { mutation } from './_generated/server'

// Throwaway: wipes all prototype state. Only exists in proto-mls branch.
// Do NOT port this to production.
export const resetAllTables = mutation({
  args: {},
  handler: async (ctx) => {
    const uins = await ctx.db.query('uins').collect()
    for (const row of uins) await ctx.db.delete(row._id)
    const keyPackages = await ctx.db.query('keyPackages').collect()
    for (const row of keyPackages) await ctx.db.delete(row._id)
    const messages = await ctx.db.query('messages').collect()
    for (const row of messages) await ctx.db.delete(row._id)
    return {
      uinsDeleted: uins.length,
      keyPackagesDeleted: keyPackages.length,
      messagesDeleted: messages.length,
    }
  },
})
