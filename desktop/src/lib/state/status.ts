// Home's heading: what the app is doing right now, in one line. Android has the same rules in
// AppState.homeStatus; change both together.
import { t } from '../copy';
import type { DiscoveryState, Transfer } from '../engine/types';

/** Drives the dot beside the heading: steady when ready, pulsing while it looks or works. */
export type StatusTone = 'ready' | 'searching' | 'busy' | 'warn';

export interface HomeStatus {
  /** Changes only when the line says something new (not with every percent), so the heading
   *  rolls to a new line once rather than on each tick. */
  key: string;
  text: string;
  tone: StatusTone;
  /** A single transfer that's moving shows how far along it is. */
  percent: number | null;
}

export interface StatusInput {
  discovery: DiscoveryState;
  /** Transfers still going (waiting, connecting, transferring, paused, reconnecting). */
  active: Transfer[];
  reachable: number;
  staging: boolean;
  deviceName: (id: string) => string;
}

const percentOf = (x: Transfer) => (x.totalBytes > 0 ? Math.min(100, Math.floor((x.doneBytes / x.totalBytes) * 100)) : 0);

/** In order: no network, transfers, files waiting for a device, devices found, looking, ready. */
export function homeStatus(s: StatusInput): HomeStatus {
  if (s.discovery === 'offline') return { key: 'offline', text: t('home.status.offline'), tone: 'warn', percent: null };
  if (s.active.length > 1) {
    return { key: 'many', text: t('home.status.many', { n: s.active.length }), tone: 'busy', percent: null };
  }
  const x = s.active[0];
  if (x) {
    const device = s.deviceName(x.deviceId);
    const at = (key: string, text: string, tone: StatusTone, percent: number | null = null): HomeStatus => ({ key: `${key}:${x.id}`, text, tone, percent });
    switch (x.status) {
      case 'waiting':
        return at('waiting', t('home.status.waiting', { device }), 'busy');
      case 'connecting':
      case 'reconnecting':
        return at('connecting', t('home.status.connecting', { device }), 'busy');
      case 'paused':
        return at('paused', t('home.status.paused', { device }), 'warn', percentOf(x));
      default:
        return x.direction === 'send'
          ? at('sending', t('home.status.sending', { device }), 'busy', percentOf(x))
          : at('receiving', t('home.status.receiving', { device }), 'busy', percentOf(x));
    }
  }
  if (s.staging) return { key: 'staged', text: t('home.status.staged'), tone: 'ready', percent: null };
  if (s.discovery === 'blocked') return { key: 'blocked', text: t('home.status.blocked'), tone: 'warn', percent: null };
  if (s.reachable > 0) {
    return s.reachable === 1
      ? { key: 'nearby', text: t('home.status.nearby.one'), tone: 'ready', percent: null }
      : { key: 'nearby', text: t('home.status.nearby.many', { n: s.reachable }), tone: 'ready', percent: null };
  }
  if (s.discovery === 'searching') return { key: 'searching', text: t('home.searching'), tone: 'searching', percent: null };
  return { key: 'ready', text: t('home.title'), tone: 'ready', percent: null };
}
