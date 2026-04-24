/**
 * Ported from convex/uins 2.ts::poolStats on 2026-04-24.
 *
 * Dogfood test: no args, reads the uins table (empty today), returns
 * a count-by-class / count-by-status aggregate. Success = briven
 * runtime can execute this and return JSON.
 */
import type { Ctx } from '@briven/schema';

const UIN_CLASSES = [
  'standard',
  'short',
  'memorable',
  'palindrome',
  'sequential',
  'ultra-rare',
  'commemorative',
] as const;

const UIN_STATUSES = [
  'available',
  'owned',
  'reserved',
  'retired',
  'canary',
] as const;

interface UinRow {
  id: string;
  class: (typeof UIN_CLASSES)[number];
  status: (typeof UIN_STATUSES)[number];
}

export default async function poolStats(ctx: Ctx) {
  const all = (await ctx.db('uins').select(['id', 'class', 'status'])) as UinRow[];

  const byClass = Object.fromEntries(UIN_CLASSES.map((c) => [c, 0])) as Record<
    (typeof UIN_CLASSES)[number],
    number
  >;
  const byStatus = Object.fromEntries(UIN_STATUSES.map((s) => [s, 0])) as Record<
    (typeof UIN_STATUSES)[number],
    number
  >;

  for (const row of all) {
    if (row.class in byClass) byClass[row.class]++;
    if (row.status in byStatus) byStatus[row.status]++;
  }

  return { total: all.length, byClass, byStatus };
}
