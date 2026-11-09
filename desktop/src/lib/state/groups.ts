// A send to several devices shows as one card on Home. Android has the same rules in
// state/SendGroups.kt; change both together.
import type { Transfer } from '../engine/types';

/** Up to this many devices in one send. */
export const MAX_DEVICES = 8;

/** One card on Home: a transfer on its own, or a send to several devices. */
export type ActivityItem = { kind: 'one'; transfer: Transfer } | { kind: 'group'; id: string; transfers: Transfer[] };

/** The transfers as Home shows them, in order. A group sits where its first transfer would. A
 *  group down to one shown transfer (the others dismissed) is just that transfer again. */
export function activityItems(transfers: Transfer[], groups: Record<string, string[]>): ActivityItem[] {
  const groupOf = new Map<string, string>();
  for (const [g, ids] of Object.entries(groups)) for (const id of ids) groupOf.set(id, g);
  const items: ActivityItem[] = [];
  const seen = new Set<string>();
  for (const tr of transfers) {
    const g = groupOf.get(tr.id);
    if (g === undefined) {
      items.push({ kind: 'one', transfer: tr });
      continue;
    }
    if (seen.has(g)) continue;
    seen.add(g);
    const members = transfers.filter((x) => groupOf.get(x.id) === g);
    items.push(members.length > 1 ? { kind: 'group', id: g, transfers: members } : { kind: 'one', transfer: tr });
  }
  return items;
}

/** The group's progress: all bytes over all devices, so a fast device doesn't hide a slow one. */
export function groupProgress(transfers: Transfer[]): number {
  const total = transfers.reduce((n, x) => n + x.totalBytes, 0);
  if (total <= 0) return transfers.every((x) => x.status === 'done') ? 1 : 0;
  return transfers.reduce((n, x) => n + (x.status === 'done' ? x.totalBytes : x.doneBytes), 0) / total;
}
