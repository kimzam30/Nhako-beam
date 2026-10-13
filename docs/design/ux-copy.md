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
| `home.dropHint` | Or drop files here. Drop them on a device to send right away. |
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
| `code.enter.body` | Type the four words shown on the other device. |
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
| `settings.trusted.empty` | No trusted devices yet. Trust one that is nearby, or save one after connecting with a code. |
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
| `settings.appearance.sounds` | Sounds |
| `settings.appearance.sounds.loud` | Loud |
| `settings.appearance.sounds.soft` | Soft |
| `settings.appearance.sounds.off` | Off |
| `settings.appearance.sounds.hint` | A click when you tap, and a chime when files arrive or finish. |
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
| `settings.about.diagnostics` | Copy diagnostics |
| `settings.about.diagnostics.hint` | Network details and recent logs, to paste into a bug report. Device names and internet addresses are left out. |
| `settings.about.diagnostics.copied` | Diagnostics copied |
| `settings.about.diagnostics.failed` | Couldn't copy the diagnostics |

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
| `send.skipped.one` | 1 item was left out. It couldn't be read, or it's an empty folder. |
| `send.skipped.many` | {n} items were left out. They couldn't be read, or they're empty folders. |
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
| `settings.desktop.fileMenu.explorer.hint` | Right-click files or folders to send them. On Windows 11 it is under Show more options. |
| `settings.desktop.fileMenu.files` | Send with Nhako Beam in your file manager |
| `settings.desktop.fileMenu.files.hint` | Right-click files or folders. In Files and Nemo it is under Scripts, in Dolphin under Actions. |

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
| `notify.channel.received` | Files received |
| `notify.request.title` | {device} wants to send you {what} |
| `notify.request.body` | Tap to accept or decline. |
| `notify.done.title` | Got {what} from {device} |
| `notify.done.body` | Saved to Download/Nhako Beam. |
| `notify.transfer.paused` | Paused |

The Android app reads every string from `copy.ts` (generated into `Copy.kt` by `android/tools/shared-assets.mjs`); a unit test fails if the two drift apart.

## 13. Added in Phase 8

History, the licenses screen, the "Find nearby devices" switch, phones that can be woken up, the
tray popover (desktop) and the Quick Settings tile (Android).

| Key | Text |
|---|---|
| `home.orb.asleep` | Asleep |
| `transfer.waking` | Waking {device} |
| `settings.network.lan.hint` | Lets devices on the same Wi-Fi see this one, and finds them. Turn it off on a network you don't trust. Trusted devices can still reach you over the internet. |
| `settings.network.lan.on` | Looking for nearby devices again |
| `settings.network.lan.off` | Nearby search is off |
| `settings.about.wake` | Wake-up when closed |
| `settings.about.wake.on` | On |
| `settings.about.wake.on.hint` | Your trusted devices can wake this phone to send it files, even with Nhako Beam closed. |
| `settings.about.wake.off` | Not set up |
| `settings.about.wake.off.hint` | Keep Nhako Beam open to receive. Wake-up needs Google Play services and a build with push set up (see wake/README.md). |
| `history.cleared` | History cleared |
| `history.sent` | To {device} |
| `history.received` | From {device} |
| `history.status.failed` | Didn't finish |
| `history.status.cancelled` | Cancelled |
| `history.status.declined` | Declined |
| `history.detail.when` | When |
| `history.detail.device` | Device |
| `history.detail.route` | Connection |
| `history.detail.size` | Size |
| `history.detail.duration` | Took |
| `history.detail.speed` | Average speed |
| `history.detail.partial` | {done} of {total} |
| `popover.devices` | Your devices |
| `popover.devices.hint` | Click a device to send files, or drop files on it. |
| `popover.quiet` | Nothing moving right now. |
| `popover.open` | Open Nhako Beam |
| `popover.finishSetup` | Open Nhako Beam to finish setting up. |
| `licenses.title` | Open source licenses |
| `licenses.intro` | Nhako Beam is built on these open source projects. Thank you to everyone who made them. |
| `licenses.count` | {count} packages |
| `licenses.search` | Search packages |
| `notify.ready.title` | Ready to receive |
| `notify.ready.body` | Your devices can find this phone for {minutes} more min. |
| `notify.ready.stop` | Stop |
| `notify.woken.title` | Ready to receive |
| `notify.woken.body` | One of your devices woke Nhako Beam to send you something. |
| `tile.receive` | Receive |
| `tile.receive.on` | For 5 minutes |

Strings in the desktop app's Rust side (`desktop/src-tauri`), which notifications and the tray use:

| Where | Text |
|---|---|
| Tray menu (Linux) | Quick view / Open Nhako Beam / Quit |
| Notification, file request (buttons) | Accept / Decline. Body: Accept to save it in your Nhako Beam folder. |
| Notification, files arrived (buttons) | Open / Show in folder |

## 14. Added in Phase 11 (security)

Codes are four words now, so `code.enter.body` changed (section 4).

| Key | Text |
|---|---|
| `send.notAllowed` | {device} only takes files from its own devices |
| `send.notAllowed.hint` | Pair with a code first, then send again. |
| `send.notAllowed.pair` | Pair with a code |
| `error.notAllowed` | {device} only takes files from its own devices. Pair with a code, then send again. |
| `incoming.notYours` | Not one of your devices |
| `settings.section.receiving` | Receiving |
| `settings.receive.who` | Who can send to me |
| `settings.receive.who.yours` | Your devices |
| `settings.receive.who.everyone` | Everyone nearby |
| `settings.receive.who.yours.hint` | Devices you trust, and devices you connect with a code. Anyone else is told no, and you won't see their requests. |
| `settings.receive.who.everyone.hint` | Anyone on the same network can ask to send. You still decide each time. |
| `settings.receive.who.failed` | Couldn't save that. Try again. |
| `open.risky.title` | Open {file}? |
| `open.risky.body` | It's a program or script, so opening it runs it. Only open it if you expected it from {device}. |
| `open.risky.confirm` | Open anyway |

## 15. Added in Phase 12 (performance)

| Key | Text |
|---|---|
| `transfer.files.more` | Show {count} more |

## 16. Added in Phase 14 (folders, text, trust from Nearby)

Folders can be sent now, so `send.skipped.one` and `send.skipped.many` speak of items that couldn't be read or empty folders, and `settings.trusted.empty` mentions trusting a device that is nearby.

Summaries add folders: "Trip folder, 240 files", "2 folders, 31 files", "Trip folder and 3 photos". A request for one folder shows the folder's name with "240 files · 1.2 GB" under it. Text shows its first line ("Meet at 6 by the north gate.").

| Key | Text |
|---|---|
| `home.empty.noLan.title` | You're not on Wi-Fi |
| `home.empty.noLan.body` | Nearby devices show up on the same Wi-Fi. Away from it you can still reach your own devices: pair one with a code, or trust it next time you're together. |
| `home.empty.noLan.action` | Use a code |
| `home.banner.noLan` | You're on mobile data. Only your own devices show up here. |
| `home.action.folder` | Folder |
| `home.action.text` | Text |
| `home.action.clipboard` | Clipboard |
| `send.clipboard.empty` | Your clipboard has no text to send. |
| `send.text.tooLong` | That's too long to send as text. Save it in a file and send that. |
| `send.choose.trust` | Trust |
| `staged.text` | Text |
| `staged.link` | Link |
| `compose.title` | Send text |
| `compose.placeholder` | Type a note, or paste a link |
| `compose.paste` | Paste |
| `compose.next` | Choose a device |
| `compose.sendTo` | Send to {device} |
| `compose.tooLong` | Too long by {over}. Send it as a file instead. |
| `text.from` | Text from {device} |
| `text.to` | Text to {device} |
| `text.link.from` | Link from {device} |
| `text.link.to` | Link to {device} |
| `text.copy` | Copy |
| `text.show` | Show |
| `text.open` | Open link |
| `text.copied` | Copied |
| `text.copied.from` | Copied text from {device} |
| `text.copied.link` | Copied a link from {device} |
| `text.copyFailed` | Couldn't copy it. Select the text and copy it yourself. |
| `device.send.files` | Send files |
| `device.send.folder` | Send a folder |
| `device.send.text` | Send text |
| `device.yours` | One of your devices |
| `device.notYours` | Nearby, not one of your devices |
| `device.fingerprint` | Fingerprint |
| `device.trust` | Trust this device |
| `device.trust.hint` | You'll each save the other as one of your devices, and find each other anywhere, over the internet too, without a code. |
| `trust.wait.title` | Waiting for {device} |
| `trust.wait.reaching` | Reaching {device} |
| `trust.wait.body` | Accept on {device}, then check that both screens show |
| `trust.ask.title` | Add {device} to your devices? |
| `trust.ask.body` | {device} asked to be one of your devices, with this one as one of its own. You'll find each other anywhere and send without a code. Only say yes if it's yours. |
| `trust.ask.check` | Check that both screens show |
| `trust.ask.yes` | Add |
| `trust.ask.no` | Not now |
| `trust.ended.declined` | {device} said no. |
| `trust.ended.outdated` | {device} needs a newer Nhako Beam first. |
| `trust.ended.unreachable` | Couldn't reach {device}. Try again when it's nearby. |
| `trust.ended.busy` | {device} is busy. Try again in a moment. |
| `trust.ended.withdrawn` | {device} stopped asking. |
| `code.matched.nearby` | {device} is one of your devices now |
| `code.matched.nearby.body` | You'll find each other anywhere now, over the internet too. |
| `settings.trusted.nearby` | Nearby, not yours yet |
| `history.detail.text` | Text |
| `notify.request.text` | some text |
| `notify.request.link` | a link |
| `notify.copied.title` | Copied {what} from {device} |
| `notify.trust.title` | {device} wants to add this phone to its devices |
| `notify.trust.body` | Open Nhako Beam to check that both screens show the same code. |

## 17. Added in Phase 15 (share via link, updates, file manager folders)

`settings.desktop.fileMenu.explorer.hint`, `settings.desktop.fileMenu.files` and `settings.desktop.fileMenu.files.hint` changed: the menu sends folders now, and on Linux it's in Files, Nemo and Dolphin.

| Key | Text |
|---|---|
| `send.choose.link` | Share via link |
| `send.choose.link.hint` | For a device without Nhako Beam, on the same Wi-Fi |
| `send.choose.link.offLan` | Connect to Wi-Fi to share with a link |
| `link.title` | Share via link |
| `link.how` | On the other device, scan this or type the address into a browser. It needs to be on the same Wi-Fi. You choose who gets in. |
| `link.qr` | QR code with the link |
| `link.copy` | Copy address |
| `link.copied` | Address copied |
| `link.endsIn` | Ends in {time} |
| `link.endsAfter` | Ends when the downloads finish |
| `link.stop` | Stop sharing |
| `link.stopped` | Stopped sharing |
| `link.ended` | The link has ended |
| `link.visitors` | Browsers |
| `link.visitors.none` | Nobody has opened it yet. |
| `link.visitor.asking` | Wants to download |
| `link.visitor.allow` | Let in |
| `link.visitor.deny` | No |
| `link.visitor.waiting` | Let in, hasn't downloaded yet |
| `link.visitor.downloading` | Downloading, {percent} |
| `link.visitor.done` | Downloaded |
| `link.visitor.doneMany` | Downloaded {n} times |
| `link.visitor.denied` | Turned away |
| `link.asks` | {browser} wants to download what you shared |
| `link.asks.body` | From {address} on this network. |
| `link.plain` | This is a plain web page, so others on this Wi-Fi could see what's downloaded. Between two Nhako Beam devices everything is encrypted. |
| `link.active` | Sharing a link |
| `link.active.asking` | {n} waiting to get in |
| `update.available` | Nhako Beam {version} is out |
| `update.download` | Download |
| `update.later` | Not now |
| `settings.about.updates` | Check for new versions |
| `settings.about.updates.hint` | Once a day, Nhako Beam asks GitHub if there's a newer version. Nothing about you or your devices is sent. |
| `settings.about.updates.failed` | Couldn't change that setting |

### The page browsers see (Share via link)

Served by the engine (`core/src/share_page.html` and `share_gone.html`), so these words live there, not in `copy.ts`. `{from}` is this device's name.

| Where | Text |
|---|---|
| Opening | Opening |
| Waiting | Waiting for {from} / Ask them to tap Let in on their screen. |
| Let in | {from} shared with you / 3 files, 12.4 MB and text |
| Everything | Download everything (12.4 MB, .zip) |
| Each file | Download |
| More than 500 files | 740 more in the .zip |
| Text | Copy / Copied |
| Turned away | {from} said no / If that was a mistake, ask them to share again. |
| Too many | Too many people are asking / Try again in a minute. |
| Ended | This link has ended / Ask for a new one. |
| Footer | Shared over this Wi-Fi with Nhako Beam. Downloads go where your browser keeps them. This is a plain web page, so others on this Wi-Fi could see what's downloaded. |
| Wrong address | Nothing is shared here now / The link may have ended, or a letter in it is off. Ask for a new one. |

### Notifications (desktop app, Rust side)

| When | Text |
|---|---|
| A browser asks | {browser} wants to download what you shared / From {address} on this network. Buttons: Let in, No |
