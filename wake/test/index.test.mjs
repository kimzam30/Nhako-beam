// cd wake && npm test  (Node 22; no packages needed)
import { test } from 'node:test';
import assert from 'node:assert/strict';
import worker, { accessToken, resetCache, sendWake, signJwt } from '../src/index.js';

const TOKEN = 'c3VwZXItc2VjcmV0:APA91bHn-test_token-0123456789abcdefghijklmnopqrstuvwxyz';

async function serviceAccount() {
  const pair = await crypto.subtle.generateKey(
    { name: 'RSASSA-PKCS1-v1_5', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
    true,
    ['sign', 'verify'],
  );
  const der = new Uint8Array(await crypto.subtle.exportKey('pkcs8', pair.privateKey));
  const b64 = Buffer.from(der).toString('base64').replace(/(.{64})/g, '$1\n');
  return {
    pair,
    account: { project_id: 'nhako-test', client_email: 'wake@nhako-test.iam.gserviceaccount.com', private_key: `-----BEGIN PRIVATE KEY-----\n${b64}\n-----END PRIVATE KEY-----\n` },
  };
}

/** Pretends to be Google: hands out an access token, and answers FCM with `fcmStatus`. */
function fakeGoogle(fcmStatus = 200) {
  const calls = [];
  const fetcher = async (url, init) => {
    calls.push({ url: String(url), init });
    if (String(url).startsWith('https://oauth2.googleapis.com/')) return Response.json({ access_token: 'ya29.test', expires_in: 3600 });
    return new Response('{}', { status: fcmStatus });
  };
  return { calls, fetcher };
}

test('the JWT is signed with the service account key and asks only for messaging', async () => {
  const { pair, account } = await serviceAccount();
  const jwt = await signJwt({ iss: account.client_email, scope: 'x' }, account.private_key);
  const [h, c, s] = jwt.split('.');
  const dec = (x) => Buffer.from(x.replace(/-/g, '+').replace(/_/g, '/'), 'base64');
  assert.deepEqual(JSON.parse(dec(h)), { alg: 'RS256', typ: 'JWT' });
  assert.equal(JSON.parse(dec(c)).iss, account.client_email);
  const ok = await crypto.subtle.verify('RSASSA-PKCS1-v1_5', pair.publicKey, dec(s), new TextEncoder().encode(`${h}.${c}`));
  assert.ok(ok, 'signature checks out');
});

test('sends one empty high-priority message, and reuses the access token', async () => {
  resetCache();
  const { account } = await serviceAccount();
  const google = fakeGoogle();
  await sendWake(account, TOKEN, google.fetcher);
  await sendWake(account, TOKEN, google.fetcher);
  const oauth = google.calls.filter((c) => c.url.includes('oauth2'));
  const fcm = google.calls.filter((c) => c.url.includes('fcm.googleapis.com'));
  assert.equal(oauth.length, 1, 'one access token for both');
  assert.equal(fcm.length, 2);
  assert.equal(fcm[0].url, 'https://fcm.googleapis.com/v1/projects/nhako-test/messages:send');
  assert.equal(fcm[0].init.headers.authorization, 'Bearer ya29.test');
  const body = JSON.parse(fcm[0].init.body);
  assert.deepEqual(body, { message: { token: TOKEN, data: { nhako: 'wake' }, android: { priority: 'HIGH', ttl: '60s' } } }, 'nothing else rides along');
  const claims = JSON.parse(Buffer.from(new URLSearchParams(oauth[0].init.body).get('assertion').split('.')[1], 'base64'));
  assert.equal(claims.scope, 'https://www.googleapis.com/auth/firebase.messaging');
  assert.equal(claims.exp - claims.iat, 3600);
});

test('the access token is fetched again once it is about to expire', async () => {
  resetCache();
  const { account } = await serviceAccount();
  const google = fakeGoogle();
  const t0 = Date.now();
  await accessToken(account, google.fetcher, t0);
  await accessToken(account, google.fetcher, t0 + 3_500_000);
  assert.equal(google.calls.length, 1);
  await accessToken(account, google.fetcher, t0 + 3_545_000);
  assert.equal(google.calls.length, 2);
});

async function call(body, { fcmStatus = 200, limited = false, method = 'POST', path = '/wake' } = {}) {
  resetCache();
  const { account } = await serviceAccount();
  const google = fakeGoogle(fcmStatus);
  const realFetch = globalThis.fetch;
  globalThis.fetch = google.fetcher;
  try {
    const env = { FIREBASE_SERVICE_ACCOUNT: JSON.stringify(account), PER_TOKEN: { limit: async () => ({ success: !limited }) } };
    const init = { method };
    if (method === 'POST') init.body = typeof body === 'string' ? body : JSON.stringify(body);
    const res = await worker.fetch(new Request(`https://wake.example.workers.dev${path}`, init), env);
    return { status: res.status, google };
  } finally {
    globalThis.fetch = realFetch;
  }
}

test('the /wake endpoint answers the way the engine expects', async () => {
  assert.equal((await call({ token: TOKEN })).status, 204);
  assert.equal((await call({ token: TOKEN }, { fcmStatus: 404 })).status, 410, 'an unregistered token is gone');
  assert.equal((await call({ token: TOKEN }, { fcmStatus: 500 })).status, 503);
  assert.equal((await call({ token: TOKEN }, { limited: true })).status, 429);
  assert.equal((await call({ token: 'short' })).status, 400);
  assert.equal((await call('not json')).status, 400);
  assert.equal((await call({ token: `${'a'.repeat(40)}"}` })).status, 400);
  assert.equal((await call(null, { method: 'GET' })).status, 405);
  assert.equal((await call(null, { method: 'GET', path: '/' })).status, 200);
  assert.equal((await call(null, { method: 'GET', path: '/x' })).status, 404);
  const limited = await call({ token: TOKEN }, { limited: true });
  assert.equal(limited.google.calls.length, 0, 'nothing is sent once the limit is hit');
});
