# Share via link

For a device without Nhako Beam: this device serves a small web page on the local network, and a
browser downloads from it after you let it in. It is plain HTTP/1.1, separate from `nhako/1` (which
is only between copies of Nhako Beam). Code: `core/src/share.rs`. Tests: `share_via_link_lets_in_only_who_you_allow`
and `a_shared_link_ends_by_itself` in `core/tests/engine.rs`, plus the unit tests in `share.rs`.

## The address

`http://<address>:<port>/<key>/`

- **address**: this device's IPv4 address on the network: the adapter with the default route if it
  has a same-network address, otherwise the first that does (`beacon::lan_adapters`). Without one,
  sharing refuses to start ("not on a local network").
- **port**: picked by the system, new for each link. The server listens on that address only.
- **key**: 16 letters and digits from a 31-character alphabet without look-alikes (about 80 bits).

Only connections from same-network addresses are served. A wrong key, or any other path, gets the
"Nothing is shared here now" page.

## Who gets in

- Opening the page (`GET /<key>/`) gives the browser a cookie (`nbv`, HttpOnly, SameSite=Strict, the
  key's path) and adds it to the list as **asking**. The app asks you; you let it in or turn it away.
- Everything else checks the cookie: only a browser you let in gets the list, files and zip. Until
  then nothing about what's shared is sent, not even names. A turned-away browser stays out.
- At most 8 browsers asking at once and 32 in all; past that the page says "Too many people are
  asking". At most 64 connections at once. A request head has 10 seconds and 8 KB.

## Paths

| Path | What |
|---|---|
| `/<key>/` | The page (`share_page.html`, its own script and styles inline, a strict Content-Security-Policy) |
| `/<key>/state` | `{"state": "asking" \| "allowed" \| "denied" \| "full" \| "ended", "from": "<this device's name>"}` |
| `/<key>/list` | Let in only: `{from, files: [{name, folder, size, href}], totalBytes, zipBytes, text}`. `zipBytes` is set when there's more than one file or a folder |
| `/<key>/f/<index>/<name>` | One file, as a download. `Range: bytes=…` (one range) for resuming. The name in the path is only for the browser; the index picks the file |
| `/<key>/all.zip` | Everything as one zip, keeping folders. Stored (not compressed), so it streams at disk speed and its length is known up front; sizes and checksums follow each file, and Zip64 is used past 4 GB. Names that clash get " (1)" |

Every answer has `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`, `Referrer-Policy:
no-referrer`, `X-Frame-Options: DENY` and `Connection: close` (one request per connection). Only GET
and HEAD.

## How long

A link works 15 minutes (`SHARE_FOR`). After that no new browsers or downloads start; downloads
running carry on, and the link stops once they're done. Stop ends everything at once, as does
quitting the app.

## What the app sees

The `linkShare` event (and `Snapshot.linkShare`) carries the whole share each time it changes: the
browsers, each with what it says it is ("Chrome on Android", from its User-Agent), its address, its
state, the bytes of downloads running now and their sizes, and how many downloads it finished.
Progress updates come at most every 250 ms.

## Privacy

It's a plain web page: anyone on the same Wi-Fi could see what's downloaded this way, and the app
says so. Between two copies of Nhako Beam everything stays end-to-end encrypted as usual.
