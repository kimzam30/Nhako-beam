// Mirrors the tests in core/src/server.rs, so the form and the engine agree.
import { describe, expect, it } from 'vitest';
import { hostOf, parseLookup, parseRelay, parseToken, readOwnForm } from './server';

const problem = (r: { ok: boolean; problem?: string }) => (r.ok ? null : r.problem);

describe('own server settings', () => {
  it('reads relay addresses like the engine', () => {
    expect(parseRelay(' http://192.168.1.20:3340 ')).toEqual({ ok: true, value: 'http://192.168.1.20:3340/' });
    expect(parseRelay('https://relay.example.com/')).toEqual({ ok: true, value: 'https://relay.example.com/' });
    expect(problem(parseRelay(''))).toBe('empty');
    expect(problem(parseRelay('192.168.1.20:3340'))).toBe('scheme');
    expect(problem(parseRelay('ftp://x'))).toBe('scheme');
    expect(problem(parseRelay('http://x/relay'))).toBe('path');
    expect(problem(parseRelay('http://x/?token=1'))).toBe('extra');
    expect(problem(parseRelay('http://user:pw@x/'))).toBe('extra');
  });

  it('adds /pkarr to a bare lookup server', () => {
    expect(parseLookup('http://10.0.0.2:3341')).toEqual({ ok: true, value: 'http://10.0.0.2:3341/pkarr' });
    expect(parseLookup('https://dns.example.com/pkarr')).toEqual({ ok: true, value: 'https://dns.example.com/pkarr' });
    expect(problem(parseLookup('dns.example.com'))).toBe('scheme');
  });

  it('checks tokens like the engine', () => {
    expect(parseToken('  ')).toEqual({ ok: true, value: null });
    expect(parseToken(' ab-12_x ')).toEqual({ ok: true, value: 'ab-12_x' });
    expect(problem(parseToken('two words'))).toBe('tokenChars');
    expect(problem(parseToken('café'))).toBe('tokenChars');
    expect(problem(parseToken('a'.repeat(513)))).toBe('tokenLong');
  });

  it('turns the form into a connection, leaving out empty parts', () => {
    expect(readOwnForm({ relay: 'http://10.0.0.2:3340', token: ' t0k ', lookup: '' }).connection).toEqual({
      mode: 'own',
      relay: 'http://10.0.0.2:3340',
      token: 't0k',
    });
    const bad = readOwnForm({ relay: '10.0.0.2', token: 'a b', lookup: 'x' });
    expect(bad.connection).toBeNull();
    expect(bad.problems).toEqual({ relay: 'scheme', token: 'tokenChars', lookup: 'scheme' });
    expect(hostOf('http://192.168.1.20:3340')).toBe('192.168.1.20');
  });
});
