# Nhako Beam UX copy

Version 0.1, Phase 1. Every user-facing string, keyed so the prototype and both apps can share them later. State numbers refer to [screens-and-states.md](screens-and-states.md). `{name}` style placeholders are filled at runtime.

---

## Voice

- **Calm and plain.** Like a helpful friend who knows computers, not a robot and not a salesperson.
- **Short.** Say it in the fewest words that are still clear. Titles under 5 words, body under 2 lines.
- **Say what happened, then what to do.** "Aisyah's laptop went offline. We'll keep trying." not "Error 0x12".
- **Use the device's name**, not "peer", "node" or "client".
- **No blame.** "That code didn't match" not "You entered an invalid code".
- **No exclamation marks, no emoji, no em dashes.** Sentence case everywhere.
- **Numbers:** "1.2 GB", "48 MB/s", "About 2 min left". Never more than one decimal.

Words we use and avoid:

| Use | Avoid |
|---|---|
| Send, receive | Upload, download, push |
| Device | Peer, node, endpoint, client |
| Nearby | LAN, local subnet |
| Direct | P2P, hole-punched |
| Relayed | TURN, forwarded |
| Code | Ticket, token |
| Trusted device | Paired peer, whitelisted |
| Save location | Output directory |

---

## 1. First run

| Key | Text |
|---|---|
| `onboarding.welcome.title` | Send anything, straight to your devices |
| `onboarding.welcome.body` | No cloud, no size limits, no account. Files go directly from one device to another. |
| `onboarding.welcome.cta` | Get started |
| `onboarding.welcome.point.encrypted` | End to end encrypted |
| `onboarding.welcome.point.fast` | Fast on the same Wi-Fi |
| `onboarding.welcome.point.anywhere` | Works across networks too |
| `onboarding.name.title` | Name this device |
| `onboarding.name.body` | This is what others see when they send you files. |
| `onboarding.name.placeholder` | For example, Hakim's laptop |
| `onboarding.name.cta` | Continue |
| `onboarding.perm.nearby.title` | Find devices nearby |
| `onboarding.perm.nearby.body` | Lets Nhako Beam see your other devices on the same Wi-Fi, so you don't need a code. |
| `onboarding.perm.notify.title` | Show transfer progress |
| `onboarding.perm.notify.body` | So you can see transfers and accept files while the app is in the background. |
| `onboarding.perm.allow` | Allow |
| `onboarding.perm.skip` | Not now |
| `onboarding.save.title` | Where should files go? |
| `onboarding.save.body` | You can change this any time in Settings. |
| `onboarding.save.change` | Change |
| `onboarding.done.cta` | Start beaming |

## 2. Home

| Key | Text |
|---|---|
| `home.title` | Ready to beam |
| `home.searching` | Looking for devices |
| `home.empty.title` | No devices nearby |
| `home.empty.body` | Make sure the other device has Nhako Beam open and is on the same Wi-Fi. Or use a code. |
| `home.action.sendCode` | Send with a code |
| `home.action.receiveCode` | Receive with a code |
| `home.action.chooseFiles` | Choose files |
| `home.action.sendFiles` | Send files |
| `home.badge.trusted` | Trusted |
| `home.orb.you` | This device |
| `home.orb.busy` | Sending |
| `home.orb.offline` | Offline |
| `home.banner.offline` | You're offline. Nearby devices will appear when you reconnect. |
| `home.banner.discoveryBlocked` | Can't look for nearby devices. Check that local network access is allowed. |
| `home.banner.discoveryBlocked.fix` | Fix this |
| `home.moreDevices` | {count} more devices |
| `home.drop.hint` | Drop to send |
| `home.drop.onDevice` | Send to {device} |
| `home.drop.noTarget` | Drop on a device |
| `home.staged.hint` | Tap a device to send |
| `home.staged.hint.pointer` | Click or drag onto a device to send |
| `home.activity` | Activity |
| `home.activity.empty` | Transfers will show up here. |

## 3. Send flow

| Key | Text |
|---|---|
| `send.choose.title` | Send to |
| `send.choose.nearby` | Nearby |
| `send.choose.trusted` | Your devices |
| `send.choose.useCode` | Use a code instead |
| `send.summary` | {count} files · {size} |
| `send.summary.one` | {fileName} · {size} |
| `send.review.title` | Check before sending |
| `send.review.remove` | Remove |
| `send.waiting` | Waiting for {device} to accept |
| `send.waiting.long` | Still waiting for {device} |
| `send.waiting.keep` | Keep waiting |
| `send.cancel` | Cancel |
| `send.declined` | {device} declined |
| `send.unreachable` | {device} is no longer reachable |
| `send.retry` | Try again |
| `send.unreadable` | Can't read {fileName}. Send the rest? |
| `send.unreadable.skip` | Send the rest |

## 4. Codes

| Key | Text |
|---|---|
| `code.show.title` | Your code |
| `code.show.body` | On the other device, choose Receive with a code and enter these words or scan this code. |
| `code.show.copy` | Copy |
| `code.show.copied` | Copied |
| `code.show.expiring` | Expires in {seconds}s |
| `code.show.expired` | This code expired |
| `code.show.new` | New code |
| `code.enter.title` | Enter the code |
| `code.enter.body` | Type the three words shown on the other device. |
| `code.enter.scan` | Scan instead |
| `code.enter.wrong` | That code didn't match. Check the words and try again. |
| `code.enter.unknownWord` | That word isn't in the list. Check the spelling. |
| `code.enter.connect` | Connect |
| `code.scan.title` | Point at the code |
| `code.scan.noCamera` | Camera access is off. You can type the words instead. |
| `code.matched` | Connected to {device} |
| `code.matched.verify` | Check that both screens show {check} |
| `code.matched.trust` | Save as a trusted device |
| `code.matched.trust.body` | Trusted devices find each other without a code. |

## 5. Incoming

| Key | Text |
|---|---|
| `incoming.title` | {device} wants to send you |
| `incoming.summary` | {summary} · {size} |
| `incoming.accept` | Accept |
| `incoming.decline` | Decline |
| `incoming.swipeAccept` | Accept |
| `incoming.swipeDecline` | Decline |
| `incoming.alwaysAccept` | Always accept from {device} |
| `incoming.noSpace` | Not enough space. You need {needed} more. |
| `incoming.noSpace.fix` | Change save location |
| `incoming.withdrawn` | {device} cancelled the request |
| `incoming.autoAccepted` | Receiving from {device} |
| `incoming.hint.gesture` | Swipe right to accept, left to decline |

Summary builder examples: "3 photos", "1 video", "3 photos and 1 video", "12 files", "A folder with 240 files".

## 6. Transfer

| Key | Text |
|---|---|
| `transfer.connecting` | Connecting |
| `transfer.sending` | Sending to {device} |
| `transfer.receiving` | Receiving from {device} |
| `transfer.speed` | {speed}/s |
| `transfer.eta` | About {time} left |
| `transfer.eta.calculating` | Working out time left |
| `transfer.route.nearby` | Nearby |
| `transfer.route.direct` | Direct |
| `transfer.route.relayed` | Relayed |
| `transfer.route.relayed.info` | Going through a relay because a direct connection wasn't possible. It's still end to end encrypted. |
| `transfer.pause` | Pause |
| `transfer.resume` | Resume |
| `transfer.paused` | Paused |
| `transfer.paused.byOther` | {device} paused this transfer |
| `transfer.reconnecting` | Reconnecting. Your progress is safe. |
| `transfer.reconnected` | Back on track |
| `transfer.done.sent` | Sent to {device} |
| `transfer.done.received` | Received from {device} |
| `transfer.done.open` | Open |
| `transfer.done.showInFolder` | Show in folder |
| `transfer.done.share` | Share |
| `transfer.done.partial` | Done, but {count} files were skipped |
| `transfer.failed` | Transfer stopped |
| `transfer.failed.retry` | Try again |
| `transfer.failed.retry.hint` | Picks up where it left off |
| `transfer.cancelled.byYou` | You cancelled this transfer |
| `transfer.cancelled.byOther` | {device} cancelled this transfer |
| `transfer.diskFull` | This device ran out of space. Free some up, then resume. |
| `transfer.diskFull.other` | {device} ran out of space. Paused until they free some up. |
| `transfer.notification.ongoing` | {percent}% · {speed}/s |
| `transfer.notification.done` | Sent {count} files to {device} |

### Failure reasons (shown under `transfer.failed`)

| Key | Text |
|---|---|
| `error.peerGone` | {device} went offline and didn't come back. |
| `error.network` | The connection dropped and couldn't be restored. |
| `error.fileChanged` | {fileName} changed while sending. Send it again to get the latest version. |
| `error.integrity` | A file arrived damaged, so it wasn't saved. Choose Try again to resend it. |
| `error.relayUnreachable` | Can't reach the relay server. Check your connection or relay settings. |
| `error.permission` | Nhako Beam doesn't have permission to save here. Choose another location. |
| `error.unknown` | Something went wrong. Try again, and if it keeps happening, check Settings > About for details. |

## 7. History

| Key | Text |
|---|---|
| `history.title` | History |
| `history.empty` | Files you send and receive will show up here. |
| `history.today` | Today |
| `history.yesterday` | Yesterday |
| `history.clear` | Clear history |
| `history.clear.confirm` | Clear all history? Your files stay where they are. |

## 8. Settings

| Key | Text |
|---|---|
| `settings.title` | Settings |
| `settings.device.name` | Device name |
| `settings.device.save` | Save location |
| `settings.trusted.title` | Trusted devices |
| `settings.trusted.empty` | No trusted devices yet. Save one after connecting with a code. |
| `settings.trusted.autoAccept` | Accept files automatically |
| `settings.trusted.remove` | Remove |
| `settings.trusted.remove.confirm` | Remove {device}? You'll need a code to connect again. |
| `settings.appearance.theme` | Theme |
| `settings.appearance.theme.system` | System |
| `settings.appearance.theme.light` | Light |
| `settings.appearance.theme.dark` | Dark |
| `settings.appearance.motion` | Motion |
| `settings.appearance.motion.system` | System |
| `settings.appearance.motion.full` | Full |
| `settings.appearance.motion.reduced` | Reduced |
| `settings.appearance.motion.hint` | Reduced swaps movement for gentle fades. |
| `settings.network.lan` | Find nearby devices |
| `settings.section.connection` | Connection |
| `settings.connection.servers` | Servers |
| `settings.connection.public` | Public |
| `settings.connection.own` | Your own |
| `settings.connection.public.hint` | Free relays run by n0, the team behind iroh. They help when a direct connection isn't possible. Files stay end to end encrypted either way. |
| `settings.connection.own.hint` | Run the server in the relay folder with Docker. Every device that should reach this one over the internet needs the same setting. Devices on the same Wi-Fi find each other either way. |
| `settings.connection.relay` | Relay address |
| `settings.connection.relay.placeholder` | http://192.168.1.20:3340 |
| `settings.connection.token` | Token |
| `settings.connection.token.placeholder` | NHAKO_RELAY_TOKEN from relay/.env |
| `settings.connection.token.show` | Show token |
| `settings.connection.token.hide` | Hide token |
| `settings.connection.lookup` | Lookup address |
| `settings.connection.lookup.placeholder` | Optional, for example http://192.168.1.20:3341 |
| `settings.connection.lookup.hint` | Leave it empty to use n0's public lookup. |
| `settings.connection.check` | Check |
| `settings.connection.save` | Save and connect |
| `settings.connection.saveAnyway` | Save anyway |
| `settings.connection.using` | Using your relay at {host} |
| `settings.connection.result.relay` | Relay |
| `settings.connection.result.lookup` | Lookup |
| `settings.connection.result.ok` | Answered in {ms} ms |
| `settings.connection.result.tokenRejected` | Turned the token down |
| `settings.connection.result.tokenNeeded` | Needs a token |
| `settings.connection.result.unreachable` | Can't reach it |
| `settings.connection.result.invalid` | Can't use this address |
| `settings.connection.notReached` | Some of it didn't answer. It may just be off right now. Save it anyway? |
| `settings.connection.notSaved.token` | Not saved. Check the token in relay/.env and try again. |
| `settings.connection.saved.own` | Now using your own server |
| `settings.connection.saved.public` | Back on the public servers |
| `server.problem.empty` | Enter an address. |
| `server.problem.scheme` | Start it with http:// or https:// |
| `server.problem.noHost` | Add the server name or IP address. |
| `server.problem.path` | Use just the server, for example http://192.168.1.20:3340 |
| `server.problem.extra` | Remove the user name and anything after ? or #. |
| `server.problem.tokenChars` | Use only letters, digits and symbols, with no spaces. |
| `server.problem.tokenLong` | Keep it under 512 characters. |
| `settings.desktop.startAtLogin` | Start at login |
| `settings.desktop.closeToTray` | Keep running in the tray when closed |
| `settings.desktop.contextMenu` | Show "Send with Nhako Beam" in the file manager |
| `settings.about.title` | About |
| `settings.about.version` | Version {version} |
| `settings.about.fingerprint` | This device's fingerprint |
| `settings.about.licenses` | Open source licenses |

## 9. Accessibility labels (screen readers)

| Key | Text |
|---|---|
| `a11y.orb` | {device}, {status}. Double tap to send files. |
| `a11y.orb.trusted` | {device}, trusted device, {status} |
| `a11y.beam` | {percent} percent, {speed} per second, about {time} left |
| `a11y.incoming` | Incoming files from {device}: {summary}, {size}. Actions: accept, decline. |
| `a11y.codeWords` | Code: {word1}, {word2}, {word3} |
| `a11y.progress.announce` | {percent} percent sent |

## 10. Added in Phase 2 (prototype)

Strings the prototype needed that were not in the first list.

| Key | Text |
|---|---|
| `onboarding.skip` | Skip |
| `send.clear` | Clear |
| `code.show.expiresIn` | Expires in {time} |
| `code.scan.type` | Type the words instead |
| `code.matched.continue` | Continue |
| `code.matched.send` | Send {summary} |
| `transfer.details` | Details |
| `transfer.files` | Files |
| `transfer.dismiss` | Dismiss |
| `error.diskFull` | This device ran out of space. |
| `error.diskFullPeer` | {device} ran out of space. |
| `settings.back` | Back |
| `settings.section.device` | This device |
| `settings.section.appearance` | Appearance |
| `common.cancel` | Cancel |
| `common.close` | Close |
| `common.done` | Done |

## 11. Added in Phase 5 (desktop app)

Strings for the real engine, your own server going wrong, and the desktop app.

| Key | Text |
|---|---|
| `home.banner.server.tokenRejected` | Your server turned this device away. Check the token in Settings. |
| `home.banner.server.unreachable` | Can't reach your server right now. Devices on the same Wi-Fi still work. |
| `home.banner.server.fix` | Settings |
| `code.enter.tokenRejected` | Your server turned this device away, so the other device couldn't be reached. Check the token in Settings. |
| `send.skipped.one` | 1 folder was left out. Folders can't be sent yet. |
| `send.skipped.many` | {n} folders were left out. Folders can't be sent yet. |
| `send.failed` | Couldn't send. |
| `transfer.openFailed` | Couldn't open it. |
| `settings.device.save.failed` | Can't use that folder. |
| `settings.section.desktop` | On this computer |
| `settings.desktop.startAtLogin` | Start when you log in |
| `settings.desktop.startAtLogin.hint` | Opens quietly in the tray, ready to receive files. |
| `settings.desktop.keepRunning` | Keep running when the window is closed |
| `settings.desktop.keepRunning.hint` | Devices can still send you files. To quit, use Quit in the tray menu. |
| `settings.desktop.failed` | Couldn't change that setting. |
| `settings.desktop.fileMenu.explorer` | Send with Nhako Beam in File Explorer |
| `settings.desktop.fileMenu.explorer.hint` | Right-click files to send them. On Windows 11 it is under Show more options. |
| `settings.desktop.fileMenu.files` | Send with Nhako Beam in Files |
| `settings.desktop.fileMenu.files.hint` | Right-click files, then Scripts, then Send with Nhako Beam. |

Written by the desktop app itself (not in `copy.ts`):

| Where | Text |
|---|---|
| Tray menu | Open Nhako Beam / Quit |
| File manager menu entry (Phase 6) | Send with Nhako Beam |
| Notification, file request | {device} wants to send you {file name, or "N files"} / Open Nhako Beam to accept or decline. |
| Notification, files arrived | Got {file name, or "N files"} from {device} / Saved to your Nhako Beam folder. |
| Notification, first close | Nhako Beam is still running / Devices can still send you files. To quit, use Quit in the tray menu. |
| Error, server change with transfers running | Wait for the transfers in progress to finish (or cancel them), then save again. |
| Error, a file to send was moved | Couldn't read {file}. It may have been moved or deleted. |
| Error, received file gone | The files aren't there any more. They may have been moved or deleted. |
| Command line, token turned down | Your server turned this device away, so the other device couldn't be reached. Check the token with: nhako server check |

## 12. Added in Phase 7 (Android app)

| Key | Text |
|---|---|
| `onboarding.save.body.android` | Files you receive go to your Download folder, where your Files and Gallery apps can find them. |
| `code.scan.cameraDenied` | Camera access is off. Turn it on in Settings, or type the words instead. |
| `code.scan.openSettings` | Open settings |
| `notify.channel.requests` | File requests |
| `notify.channel.transfers` | Transfers |
| `notify.request.title` | {device} wants to send you {what} |
| `notify.request.body` | Tap to accept or decline. |
| `notify.done.title` | Got {what} from {device} |
| `notify.done.body` | Saved to Download/Nhako Beam. |
| `notify.transfer.paused` | Paused |

The Android app reads every string from `copy.ts` (generated into `Copy.kt` by `android/tools/shared-assets.mjs`); a unit test fails if the two drift apart.

