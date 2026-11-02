# Product

<!-- impeccable:product-schema 1 -->

## Platform

adaptive

The desktop app (Windows, Linux) is a Svelte 5 web UI in a Tauri 2 window; the Android app is Kotlin and Jetpack Compose. The two are twins: same behavior, copy, colors and motion, each with its own platform manners.

## Users

One person moving files between their own devices: a laptop, a desktop PC, a phone. Usually mid-task (a photo from the phone to the laptop, a document from work PC to home PC), at home or at work, on the same Wi-Fi or across the internet. The job is "get this thing over there, now", without accounts, cables or size limits.

## Product Purpose

Nhako Beam sends files straight between your own devices. Success is a transfer that starts in a couple of seconds and finishes at full network speed, with honest progress, and devices that find each other on their own.

## Positioning

Direct device to device, with no cloud copy, no size limit and no account. Free, at RM0. Same Wi-Fi goes direct; otherwise a relay carries the encrypted stream. Pairing a new device takes a four-word code or a QR.

## Operating Context

- Home is the surface used most: it shows this device, the devices around it, and the ways to send (files, folder, text, clipboard, a code).
- Files can be dropped onto a device on Home to send right away.
- Desktop also lives in the system tray with a small popover.

## Capabilities and Constraints

- Engine in Rust on iroh; the JSON contract is shared by desktop and Android.
- All UI strings come from one copy file shared by both apps.
- Design tokens (color, type, spacing, motion) come from one source built for CSS, TS and Kotlin.

## Brand Commitments

- Name: Nhako Beam. The mascot is a voxel lightning bolt with a face, built from the logo's pixels; it stands as a trophy on the shelf in Home's room.
- No gradients anywhere, in UI or graphics. No em dashes in copy. Plain, human language.
- Everything is animated, with a reduced-motion mode.
- Calm by default; Home's room is the one place allowed to be colorful, detailed and alive (owner decision).

## Evidence on Hand

- Brand kit: `docs/brand/` (logo SVG/PNG, posters, launch videos). Mascot: `design/mascot/`. Sounds: `design/sounds/`.
- No testimonials, user counts or benchmarks for marketing; do not invent any.

## Product Principles

1. Honest status: progress, speed and route are real engine numbers.
2. Direct manipulation: things follow the pointer or finger and can be grabbed mid-animation.
3. One brand, native manners on each platform.
4. The core (finding devices, moving files) must never be in doubt.

## Accessibility & Inclusion

Every text pair meets WCAG AA in both themes (checked by `design/tokens/check-contrast.mjs`). Reduced motion turns movement into fades. Large system text must not break layouts.
