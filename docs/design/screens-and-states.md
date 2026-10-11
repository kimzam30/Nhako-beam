# Nhako Beam screens and states

Version 0.1, Phase 1. Every screen, every state it can be in, and what moves between them. The Phase 2 prototype must cover every row marked **P** (prototype). Rows marked **later** are designed now but built after the MVP.

Platforms: **D** desktop (Windows, Linux), **A** Android. Copy for every state is in [ux-copy.md](ux-copy.md). Motion references point to sections in [motion-spec.md](motion-spec.md).

---

## Map

```
First run ──► Home (radar) ──┬──► Send flow ──► Waiting ──► Transfer ──► Done / Failed
                             ├──► Receive with code
                             ├──► Incoming request (can appear on top of anything)
                             ├──► History
                             └──► Settings ──► Trusted devices, Connection, About

Entry points outside the app:
  D: tray icon, file manager right-click "Send with Nhako Beam", drag onto the window or tray
  A: system share sheet "Nhako Beam", notification actions, quick settings tile (later)
```

---

## 1. First run

| # | State | D | A | P | Notes |
|---|---|---|---|---|---|
| 1.1 | Welcome | ✓ | ✓ | P | One sentence on what the app does, one button. No carousel. |
| 1.2 | Name this device | ✓ | ✓ | P | Prefilled from the OS (hostname / device model). Live preview of how the orb will look to others. |
| 1.3 | Permissions | | ✓ | P | Ask one at a time, each with a reason: Nearby devices (for LAN discovery), Notifications (transfer progress), Camera only when first scanning a QR. |
| 1.4 | Save location | ✓ | ✓ | P | Default Downloads/Nhako Beam. Change button. |
| 1.5 | Ready | ✓ | ✓ | P | Lands on Home with the radar animation (motion 4.1). |

Skipping is allowed on 1.2 to 1.4; defaults are used.

## 2. Home (radar)

| # | State | D | A | P | Notes |
|---|---|---|---|---|---|
| 2.1 | Searching | ✓ | ✓ | P | Sweep ring, own orb in the center. |
| 2.2 | Devices found | ✓ | ✓ | P | Trusted devices first (with a check badge), then nearby ones. |
| 2.3 | Nothing nearby | ✓ | ✓ | P | Calm hint plus two actions: "Send with a code" and "Receive with a code". |
| 2.4 | Offline (no network at all) | ✓ | ✓ | P | Banner. Radar still shows trusted devices dimmed. |
| 2.5 | Local network discovery blocked | ✓ | ✓ | P | For example Windows firewall or Android permission denied. Banner with a fix action. |
| 2.6 | Transfer running in background | ✓ | ✓ | P | Mini beam on the busy device orb plus a compact transfer bar at the bottom. |
| 2.7 | Files dragged over window | ✓ | | P | Drop overlay (motion 4.3). |
| 2.8 | Files waiting to be sent (Android, after picking or share sheet) | | ✓ | P | File stack at the top, orbs breathe as targets. |
| 2.9 | Many devices (more than 8) | ✓ | ✓ | | Radar shows the 8 closest; a "More devices" list row opens the full list. |
| 2.10 | Your own server has a problem | ✓ | ✓ | P | Banner: it turned the token down, or it doesn't answer, with a Settings action. The engine checks at start, on every rescan, every 5 minutes, every 30 s while something is wrong, and when a code fails. Added in Phase 5. |

Layouts: compact = radar full width with actions under it. Expanded = radar on the left, activity (current and recent transfers) on the right.

## 3. Send flow

| # | State | D | A | P | Notes |
|---|---|---|---|---|---|
| 3.1 | Choose files | ✓ | ✓ | P | Desktop: drag and drop or "Choose files" button (also folders). Android: system picker or share sheet. |
| 3.2 | Choose a device | ✓ | ✓ | P | Sheet with nearby, trusted and "Use a code instead". |
| 3.3 | Review (optional) | ✓ | ✓ | P | Shown only for more than 20 files or over 4 GB: file count, total size, remove items. Otherwise skipped. |
| 3.4 | Waiting for accept | ✓ | ✓ | P | Pulsing ring (motion 4.5). Cancel always visible. Trusted devices with auto-accept skip this. |
| 3.5 | Still waiting (60s) | ✓ | ✓ | P | Keep waiting or cancel. |
| 3.6 | Declined | ✓ | ✓ | P | Files return, short message. |
| 3.7 | Device went away before accepting | ✓ | ✓ | P | "Aisyah's laptop is no longer reachable" with Try again. |
| 3.8 | File unreadable | ✓ | ✓ | | Permission or missing file. Lets you skip that file and send the rest. |

## 4. Pairing with a code

| # | State | D | A | P | Notes |
|---|---|---|---|---|---|
| 4.1 | Show my code (sender) | ✓ | ✓ | P | QR + three words + countdown ring. Copy button. |
| 4.2 | Code about to expire (last 20s) | ✓ | ✓ | P | Ring turns warning color. "New code" button. |
| 4.3 | Code expired | ✓ | ✓ | P | Fades out, one tap for a new one. |
| 4.4 | Enter a code (receiver) | ✓ | ✓ | P | Three word slots with autocomplete; paste fills all. |
| 4.5 | Scan a code | | ✓ | P | Camera viewfinder. Desktop shows words only (no webcam scanning in MVP). |
| 4.6 | Wrong or unknown code | ✓ | ✓ | P | Gentle shake on the wrong word. When your own server turned this device's token down, it says so instead of "didn't match" (Phase 5). |
| 4.7 | Matched | ✓ | ✓ | P | Orbs meet (motion 4.7). Offer "Save as trusted device" with a short check value (the same on both screens, derived from both device keys) to compare. |
| 4.8 | Camera permission denied | | ✓ | | Fall back to typing words, with a link to settings. |

## 5. Incoming request

Can appear on top of any screen. On desktop also as a system notification with Accept / Decline buttons when the window is hidden.

| # | State | D | A | P | Notes |
|---|---|---|---|---|---|
| 5.1 | Request card | ✓ | ✓ | P | Sender orb and name, connection badge, file summary ("3 photos and 1 video"), total size. |
| 5.2 | Dragging toward accept / decline | ✓ | ✓ | P | Motion 4.6. |
| 5.3 | Not enough space for this transfer | ✓ | ✓ | P | Accept disabled, reason shown, "Change save location". |
| 5.4 | Several requests at once | ✓ | ✓ | | Cards stack (max 3 visible, offset 8 each). Top one is active. |
| 5.5 | Auto-accepted (trusted) | ✓ | ✓ | P | No card; a toast "Receiving from Aisyah's laptop" and the transfer view. |
| 5.6 | Request withdrawn by sender | ✓ | ✓ | P | Card fades out with a short toast. |

## 6. Transfer

| # | State | D | A | P | Notes |
|---|---|---|---|---|---|
| 6.1 | Connecting | ✓ | ✓ | P | Beam track visible, no fill, label "Connecting". |
| 6.2 | Transferring | ✓ | ✓ | P | Beam, percent, speed, time left, per-file list, connection badge. |
| 6.3 | Paused (by you) | ✓ | ✓ | P | Motion 4.9. |
| 6.4 | Paused by the other side | ✓ | ✓ | | "Aisyah paused this transfer". |
| 6.5 | Reconnecting | ✓ | ✓ | P | Warning banner, progress is kept. |
| 6.6 | Route changed (relayed to direct) | ✓ | ✓ | P | Badge swap, speed usually goes up. |
| 6.7 | Done | ✓ | ✓ | P | Motion 4.10. Open / Show in folder / Share. |
| 6.8 | Done with some files skipped | ✓ | ✓ | | Lists the skipped files and why. |
| 6.9 | Failed | ✓ | ✓ | P | Reason and Try again (resumes, does not restart). |
| 6.10 | Cancelled | ✓ | ✓ | P | By you or by the other side, said clearly which. |
| 6.11 | Disk full mid-transfer | ✓ | ✓ | P | Pauses automatically, tells both sides. |
| 6.12 | App in background | | ✓ | | Foreground service notification with progress, Pause and Cancel actions. |

## 7. History (later in MVP, designed now)

| # | State | D | A | P |
|---|---|---|---|---|
| 7.1 | List grouped by day, sent and received | ✓ | ✓ | |
| 7.2 | Empty | ✓ | ✓ | |
| 7.3 | Item details (files, device, route, duration, average speed) | ✓ | ✓ | |
| 7.4 | Clear history | ✓ | ✓ | |

## 8. Settings

| # | Section | D | A | P | Notes |
|---|---|---|---|---|---|
| 8.1 | This device: name, save location | ✓ | ✓ | P | The desktop app uses the system folder picker (Phase 5). On Android (Phase 7) files always go to Download/Nhako Beam, where the Files and Gallery apps find them. |
| 8.2 | Trusted devices: list, rename, auto-accept toggle, remove | ✓ | ✓ | P | Remove asks for confirmation. |
| 8.3 | Appearance: theme (System, Light, Dark), motion (System, Full, Reduced) | ✓ | ✓ | P | Motion toggle previews live. |
| 8.4 | Connection: public servers (n0) or your own (relay address, token, optional lookup address). Check shows each server's result; Save checks first. A wrong token is never saved; a server that doesn't answer can be saved on purpose ("Save anyway"). Switching back to Public saves at once. | ✓ | ✓ | P | Built in Phase 4 on the mock engine. In the desktop app (Phase 5), saving restarts the engine on the new servers, and is refused while transfers are running. LAN discovery on or off is still to come. |
| 8.5 | Desktop: start at login, close to tray, right-click menu integration | ✓ | | | Start at login and keep running when closed are in "On this computer" (Phase 5). The file manager menu switch came in Phase 6 (Explorer on Windows, Files on Linux). |
| 8.6 | About: version, licenses, device fingerprint | ✓ | ✓ | | Version and fingerprint come from the engine (Phase 5). Licenses still to come. |

## 9. Desktop extras

| # | Surface | Notes |
|---|---|---|
| 9.1 | Tray popover (360 x 520) | Compact home: devices row, current transfer, "Open Nhako Beam". Files can be dropped on the tray icon on Windows. Phase 5 ships a tray menu (Open Nhako Beam, Quit); the popover is still to come. |
| 9.2 | File manager menu | "Send with Nhako Beam" opens the device chooser directly with the files loaded. Built in Phase 6: Explorer (Windows 11: under Show more options) and Files under Scripts (Linux). A running app takes the files over from the new launch. |
| 9.3 | System notifications | Incoming request (with actions), transfer done, failed. Phase 5 shows incoming requests and finished transfers while the window is closed or in the background, without action buttons for now. |

## 10. Android extras

| # | Surface | Notes |
|---|---|---|
| 10.1 | Share sheet target | Opens straight into state 2.8. Built in Phase 7: one file or many, of any kind. |
| 10.2 | Progress notification | Ongoing, with Pause and Cancel. Collapses to "Sent 4 files to Aisyah's laptop" when done. Phase 7: shown by a foreground service while anything is moving (the newest transfer, with Pause or Resume and Cancel, and "+N" when more are going). It goes away when the last one ends; received files get their own "Got ... from ..." notification while the app is out of sight. |
| 10.3 | Quick settings tile (later) | "Receive" makes the device visible for 5 minutes. |
| 10.4 | Request notification | Phase 7: "{device} wants to send you {what}" with Decline and Accept, while the app is out of sight. Removed once the request is answered or withdrawn. |
| 10.5 | Received files | Phase 7: added to the media index after they arrive, so photos and videos show in the Gallery. "Open" opens one file, or the folder in the Files app for several; "Share" hands them to the system share sheet. |
