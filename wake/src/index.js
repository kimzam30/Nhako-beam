// Nhako Beam wake service: a Cloudflare Worker that wakes a sleeping phone so it can take files
// while the app is closed. The engine (core/src/wake.rs) posts {"token": "..."} to /wake; this
// Worker sends that token an empty, high-priority Firebase Cloud Messaging message.
//
// What the push carries: nothing but "wake up". No names, no files, not who is sending. This
// Worker logs nothing about tokens and keeps nothing between requests except Google's access token.
//
// Secrets (wrangler secret put): FIREBASE_SERVICE_ACCOUNT, the service account JSON from Firebase.
// Optional binding: PER_TOKEN, a rate limiter (wrangler.toml), so a token can't be used to spam a phone.

const TOKEN = /^[A-Za-z0-9_:-]{32,4096}$/;
const SCOPE = 'https://www.googleapis.com/auth/firebase.messaging';
const OAUTH = 'https://oauth2.googleapis.com/token';

/** Google's access token, reused until a minute before it expires. */
let cached = null;

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname === '/' && request.method === 'GET') return text(200, 'Nhako Beam wake service');
    if (url.pathname !== '/wake') return text(404, 'Not found');
    if (request.method !== 'POST') return text(405, 'Use POST');

    const raw = await request.text();
    if (raw.length > 8192) return text(413, 'Too big');
    let token;
    try {
      token = JSON.parse(raw).token;
    } catch {
      return text(400, 'Send JSON: {"token": "..."}');
    }
    if (typeof token !== 'string' || !TOKEN.test(token)) return text(400, 'Not a wake token');

    if (env.PER_TOKEN) {
      const { success } = await env.PER_TOKEN.limit({ key: await sha256(token) });
      if (!success) return text(429, 'Too many wake-ups for this device. Try again in a minute.');
    }

    let account;
    try {
      account = JSON.parse(env.FIREBASE_SERVICE_ACCOUNT);
    } catch {
      return text(500, 'FIREBASE_SERVICE_ACCOUNT is not set up');
    }

    const res = await sendWake(account, token);
    if (res.ok) return new Response(null, { status: 204 });
    // The app was uninstalled or its data cleared: tell the sender to forget the token.
    if (res.status === 404 || res.status === 400) return text(410, 'That device can no longer be woken');
    return text(503, 'Firebase did not take the message, try again later');
  },
};

/** One empty data message, high priority so Android delivers it to a sleeping phone at once. */
export async function sendWake(account, token, fetcher = fetch) {
  const access = await accessToken(account, fetcher);
  return fetcher(`https://fcm.googleapis.com/v1/projects/${account.project_id}/messages:send`, {
    method: 'POST',
    headers: { authorization: `Bearer ${access}`, 'content-type': 'application/json' },
    body: JSON.stringify({
      message: {
        token,
        data: { nhako: 'wake' },
        // A wake-up that arrives later than this is no use: the sender has given up.
        android: { priority: 'HIGH', ttl: '60s' },
      },
    }),
  });
}

/** Signs a JWT with the service account's key and trades it for an access token. */
export async function accessToken(account, fetcher = fetch, now = Date.now()) {
  if (cached && cached.email === account.client_email && cached.until > now) return cached.token;
  const iat = Math.floor(now / 1000);
  const claims = { iss: account.client_email, scope: SCOPE, aud: OAUTH, iat, exp: iat + 3600 };
  const jwt = await signJwt(claims, account.private_key);
  const res = await fetcher(OAUTH, {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({ grant_type: 'urn:ietf:params:oauth:grant-type:jwt-bearer', assertion: jwt }),
  });
  if (!res.ok) throw new Error(`Google turned the service account down (HTTP ${res.status})`);
  const { access_token, expires_in } = await res.json();
  cached = { email: account.client_email, token: access_token, until: now + (expires_in - 60) * 1000 };
  return access_token;
}

export async function signJwt(claims, privateKeyPem) {
  const enc = (obj) => b64url(new TextEncoder().encode(JSON.stringify(obj)));
  const input = `${enc({ alg: 'RS256', typ: 'JWT' })}.${enc(claims)}`;
  const der = Uint8Array.from(atob(privateKeyPem.replace(/-----[^-]+-----/g, '').replace(/\s+/g, '')), (c) => c.charCodeAt(0));
  const key = await crypto.subtle.importKey('pkcs8', der, { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' }, false, ['sign']);
  const sig = await crypto.subtle.sign('RSASSA-PKCS1-v1_5', key, new TextEncoder().encode(input));
  return `${input}.${b64url(new Uint8Array(sig))}`;
}

/** Forget the cached access token (tests). */
export function resetCache() {
  cached = null;
}

function b64url(bytes) {
  let s = '';
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

async function sha256(s) {
  const d = new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(s)));
  return [...d].map((b) => b.toString(16).padStart(2, '0')).join('');
}

function text(status, body) {
  return new Response(body, { status, headers: { 'content-type': 'text/plain; charset=utf-8' } });
}
