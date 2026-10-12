# Nhako Beam wake service

Android pauses apps that are closed, so a phone with Nhako Beam closed can't hear a request. This
service lets your other devices wake it up: they ask it to send the phone an empty push through
Firebase Cloud Messaging, the phone starts Nhako Beam in the background for about a minute, and the
transfer goes through as usual (with the request notification, or straight in if you set "Accept
files automatically" for that device).

Everything here is free: Firebase Cloud Messaging costs nothing at any volume, and the Cloudflare
Workers free plan allows 100,000 requests a day. No card is needed for either.

## What the push carries

Nothing but "wake up" (`{"nhako": "wake"}`). No device names, no file names, not who is sending.
Google sees that your phone got a push at a certain time; Cloudflare sees that some device asked for
it. Files never touch either: they still go straight between your devices, encrypted end to end.

A phone only shares its wake token with devices it trusts, inside the encrypted connection. Each
phone can be woken at most 6 times a minute.

## What you need

- A Google account (for Firebase) and a Cloudflare account. Both are free.
- Node 20 or newer on your computer.
- Phones with Google Play services. Phones without them (some Huawei phones, de-Googled ROMs) still
  work while Nhako Beam is open; they just can't be woken.

## Set it up (once, about 15 minutes)

### 1. Firebase

1. Go to [console.firebase.google.com](https://console.firebase.google.com) and choose **Create a
   project**. Call it `nhako-beam`. Google Analytics isn't needed: turn it off.
2. In the project, choose **Add app**, then **Android**. Package name: `io.github.kimzam30.nhakobeam`.
   Add a second Android app with `io.github.kimzam30.nhakobeam.debug` too: the .apk uses the first
   name, the tests build the debug one, and the build stops if either is missing. Skip the other
   steps of the wizard.
3. Download **google-services.json** (Project settings > General > Your apps) and put it at
   `android/app/google-services.json`. It lists both apps. It isn't secret, but it's yours, so it's
   git-ignored.
4. Project settings > **Service accounts** > **Generate new private key**. This file *is* secret:
   it lets whoever holds it send pushes to your app. Keep it out of git and delete it after step 2.4.

### 2. Cloudflare

1. Create a free account at [dash.cloudflare.com](https://dash.cloudflare.com).
2. In a terminal, from this folder: `npx wrangler@4.149.0 login`
3. `npx wrangler@4.149.0 deploy`. It prints the address, something like
   `https://nhako-wake.<your-name>.workers.dev`.
4. Give it the key from step 1.4:
   `npx wrangler@4.149.0 secret put FIREBASE_SERVICE_ACCOUNT < path/to/the-key.json`
   Then delete that file from your computer.
5. Check it: opening `https://nhako-wake.<your-name>.workers.dev/` in a browser says
   "Nhako Beam wake service".

### 3. Build the apps with it

Every app that should be able to wake a phone needs the address, with `/wake` at the end. Put it in
your shell before building (or in `~/.bashrc`):

```bash
export NHAKO_WAKE_URL=https://nhako-wake.<your-name>.workers.dev/wake
scripts/build-android.sh
scripts/build-linux.sh
```

On Windows: `$env:NHAKO_WAKE_URL = "https://..."` before `scripts\build-windows.ps1`.

Rebuild the Android app after adding `google-services.json`. Then open it once on the phone and,
on each of your other devices, open Nhako Beam while the phone is nearby: they swap the wake token
the next time they connect. Settings > About on the phone shows "Wake-up when closed: On".

## Check it works

Close Nhako Beam on the phone (swipe it away from recent apps) and wait a minute. Send it a file
from your laptop: the transfer says "Waking Aisyah's phone", then the request appears on the phone.

## Files

| File | What |
|---|---|
| `src/index.js` | The Worker: checks the request, signs in to Google with the service account, sends the push. |
| `wrangler.toml` | Its Cloudflare settings: no request logs, 6 wake-ups a minute per phone. |
| `test/` | Tests with a pretend Google (`npm test`, no packages needed). |

## Costs and limits

- Cloudflare Workers free plan: 100,000 requests a day. One transfer to a sleeping phone is one request.
- Firebase Cloud Messaging: free, no limit that a personal app gets near.
- If you go past the Workers free limit, wake-ups stop for the rest of the day (nothing is charged);
  devices that are open keep working.
