// Your own server settings: the same rules as core/src/server.rs, so the form can say what's
// wrong before anything is saved. Keep both in step (the tests mirror the Rust ones).
import type { Connection } from './types';

export type AddressProblem = 'empty' | 'scheme' | 'noHost' | 'path' | 'extra';
export type TokenProblem = 'tokenChars' | 'tokenLong';
export type ServerProblem = AddressProblem | TokenProblem;

type Parsed<T> = { ok: true; value: T } | { ok: false; problem: ServerProblem };

function parseHttpUrl(raw: string): Parsed<URL> {
  const s = raw.trim();
  if (!s) return { ok: false, problem: 'empty' };
  if (!s.includes('://')) return { ok: false, problem: 'scheme' };
  let url: URL;
  try {
    url = new URL(s);
  } catch {
    return { ok: false, problem: 'noHost' };
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') return { ok: false, problem: 'scheme' };
  if (!url.hostname) return { ok: false, problem: 'noHost' };
  if (url.search || url.hash || url.username || url.password || s.includes('?') || s.includes('#')) return { ok: false, problem: 'extra' };
  return { ok: true, value: url };
}

/** A relay address is just the server: `http(s)://host[:port]`. Returns it with a trailing slash. */
export function parseRelay(raw: string): Parsed<string> {
  const r = parseHttpUrl(raw);
  if (!r.ok) return r;
  if (r.value.pathname !== '/') return { ok: false, problem: 'path' };
  return { ok: true, value: r.value.href };
}

/** The lookup server's pkarr address. A bare server gets `/pkarr`, where relay/ serves it. */
export function parseLookup(raw: string): Parsed<string> {
  const r = parseHttpUrl(raw);
  if (!r.ok) return r;
  if (r.value.pathname === '/') r.value.pathname = '/pkarr';
  return { ok: true, value: r.value.href };
}

/** Printable ASCII, no spaces, at most 512 characters. Empty means no token. */
export function parseToken(raw: string): Parsed<string | null> {
  const s = raw.trim();
  if (!s) return { ok: true, value: null };
  if (s.length > 512) return { ok: false, problem: 'tokenLong' };
  if (!/^[\x21-\x7e]+$/.test(s)) return { ok: false, problem: 'tokenChars' };
  return { ok: true, value: s };
}

export interface OwnForm {
  relay: string;
  token: string;
  lookup: string;
}

export interface FormProblems {
  relay?: ServerProblem;
  token?: ServerProblem;
  lookup?: ServerProblem;
}

/** Checks the form; returns the problems, or the connection to save. */
export function readOwnForm(f: OwnForm): { problems: FormProblems; connection: Extract<Connection, { mode: 'own' }> | null } {
  const relay = parseRelay(f.relay);
  const token = parseToken(f.token);
  const lookup = f.lookup.trim() ? parseLookup(f.lookup) : ({ ok: true, value: null } as const);
  const problems: FormProblems = {};
  if (!relay.ok) problems.relay = relay.problem;
  if (!token.ok) problems.token = token.problem;
  if (!lookup.ok) problems.lookup = lookup.problem;
  if (!relay.ok || !token.ok || !lookup.ok) return { problems, connection: null };
  return {
    problems,
    connection: {
      mode: 'own',
      relay: f.relay.trim(),
      ...(token.value ? { token: token.value } : {}),
      ...(lookup.value ? { lookup: f.lookup.trim() } : {}),
    },
  };
}

/** "192.168.1.20" from "http://192.168.1.20:3340", for short status lines. */
export function hostOf(address: string): string {
  try {
    return new URL(address).hostname;
  } catch {
    return address;
  }
}
