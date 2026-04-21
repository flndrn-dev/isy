import { defineSchema, defineTable } from 'convex/server'
import { v } from 'convex/values'

export default defineSchema({
  uins: defineTable({
    uin: v.number(),                  // 9-digit integer
    credentialBytes: v.bytes(),       // serialized MLS BasicCredential
    publicSignatureKey: v.bytes(),    // Ed25519 public key
    createdAt: v.number(),
  })
    .index('by_uin', ['uin']),

  keyPackages: defineTable({
    uin: v.number(),
    keyPackageBytes: v.bytes(),       // serialized KeyPackage (ciphersuite-bound)
    publishedAt: v.number(),
    consumed: v.boolean(),            // one-shot per MLS spec
  })
    .index('by_uin_unconsumed', ['uin', 'consumed']),

  messages: defineTable({
    recipientUin: v.number(),
    senderUin: v.number(),            // for routing only — content is opaque
    groupId: v.bytes(),               // MLS group ID
    ciphertext: v.bytes(),            // serialized MlsMessageOut
    createdAt: v.number(),
    delivered: v.boolean(),           // true once recipient has fetched
  })
    .index('by_recipient_undelivered', ['recipientUin', 'delivered']),
})
