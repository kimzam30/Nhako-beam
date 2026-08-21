<div align="center">

# ⚡ Nhako Beam

**Open-source, cross-platform direct file transfer — no cloud, no limits, no subscriptions.**

[![Status](https://img.shields.io/badge/status-planning-blue)](https://github.com/yourorg/nhako-beam)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20Android-lightgrey)]()
[![Docker](https://img.shields.io/badge/docker-ready-2496ED?logo=docker)](https://hub.docker.com/r/yourorg/nhako-beam-relay)

> 🚧 **This project is currently in the planning phase.** Architecture and specifications are being finalized. Contributions and feedback are welcome.

</div>

---

## 📖 Overview

**Nhako Beam** is an open-source alternative to proprietary direct-transfer applications like Blip. It enables fast, secure, and unlimited file transfers directly between devices — without uploading files to cloud storage, without file size caps, and without requiring user accounts.

Built with a privacy-first architecture, Nhako Beam streams files peer-to-peer whenever possible, and falls back to a self-hosted relay only when network conditions require it. The entire stack is open-source, free forever, and deployable in seconds via Docker.

### Why Nhako Beam?

| Feature | Proprietary Alternatives | Nhako Beam |
|---------|------------------------|------------|
| Open Source | ❌ Closed | ✅ Fully open |
| Self-hosted relay | ❌ Not available | ✅ Docker in one command |
| Cross-platform | ✅ Limited | ✅ Windows, Linux, Android |
| File size limits | ⚠️ Freemium tiers | ✅ Unlimited (disk-bound) |
| Cloud storage required | ❌ Yes (intermediate) | ✅ Direct stream, no cloud |
| LAN acceleration | ✅ Yes | ✅ mDNS direct connect |
| Free forever | ⚠️ Freemium / Ads | ✅ Truly free (MIT License) |

---

## 🏗️ Architecture

Nhako Beam is built on a modular, three-layer architecture designed for resilience, speed, and privacy.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CLIENT LAYER                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────────┐   │
│  │   Windows    │  │    Linux     │  │            Android               │   │
│  │  (Tauri)     │  │  (Tauri)     │  │         (Kotlin)                 │   │
│  └──────┬───────┘  └──────┬───────┘  └──────────────┬───────────────────┘   │
│         │                 │                         │                       │
│         └─────────────────┴─────────────────────────┘                       │
│                           │                                                │
│                    ┌────────┴────────┐                                      │
│                    │   Core Engine   │  ← Chunking, Crypto, QUIC, Resume   │
│                    │    (Rust)       │                                      │
│                    └────────┬────────┘                                      │
└─────────────────────────────┼──────────────────────────────────────────────┘
                              │
                              │  WebSocket Signaling / QUIC Relay
                              ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              RELAY LAYER                                     │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    Nhako Beam Relay (Docker)                         │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────────┐  │   │
│  │  │   Signaling  │  │ NAT Traversal│  │      Relay / TURN-like     │  │   │
│  │  │   Server     │  │   Helper     │  │      Packet Forwarder      │  │   │
│  │  │  (WebSocket) │  │  (ICE-like)  │  │      (QUIC / TCP)          │  │   │
│  │  └──────────────┘  └──────────────┘  └────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  Deployment: Docker / Docker Compose / Bare Metal (single binary)           │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **Direct First**: Attempt peer-to-peer connection via LAN (mDNS) or WAN (NAT traversal) before touching any relay.
2. **Relay as Fallback**: The self-hosted Docker relay only handles signaling and byte-forwarding when P2P is impossible. It never stores decrypted data.
3. **Chunked Streaming**: Files are broken into encrypted chunks with cryptographic checksums. Transfers resume automatically from the last acknowledged chunk.
4. **Zero Trust**: All data is encrypted end-to-end using the Noise Protocol (or TLS 1.3 with mutual authentication). The relay sees only opaque packets.

---

## 🚀 Planned Features

### Phase 1 — Core Protocol (In Planning)
- [ ] **Chunked File Streaming**: Break files into resumable, cryptographically verified chunks
- [ ] **End-to-End Encryption**: Noise Protocol / TLS 1.3 with per-device keypairs
- [ ] **QUIC Transport**: Fast, reliable UDP-based transfer with built-in congestion control
- [ ] **Auto-Resume**: SQLite-backed transfer state; reconnect and resume from any chunk
- [ ] **LAN Direct Connect**: mDNS/Bonjour discovery for same-network zero-latency transfers

### Phase 2 — Relay & Infrastructure (In Planning)
- [ ] **Self-Hosted Relay**: Dockerized signaling + relay server (Go / Rust)
- [ ] **NAT Traversal**: ICE-like candidate exchange for WAN P2P
- [ ] **TURN-like Fallback**: Encrypted packet forwarding when direct connection fails
- [ ] **Multi-Relay Support**: Client can switch between private, community, or public relays
- [ ] **Rate Limiting & Abuse Prevention**: Configurable quotas and token-based auth

### Phase 3 — Desktop Clients (In Planning)
- [ ] **Windows Client**: Native Tauri app with system tray and Explorer context menu
- [ ] **Linux Client**: Tauri app with AppImage / .deb packaging and Nautilus integration
- [ ] **Drag & Drop UI**: Simple, fast interface with real-time progress
- [ ] **Background Service**: Persistent WebSocket connection for instant availability

### Phase 4 — Android Client (In Planning)
- [ ] **System Share Sheet Integration**: "Beam with Nhako" from any app
- [ ] **Foreground Transfer Service**: Survives app backgrounding with notification progress
- [ ] **Auto-Accept**: Trusted device pairing for hands-free reception
- [ ] **Material You Design**: Native Android UI with dynamic theming

### Phase 5 — Polish & Ecosystem (In Planning)
- [ ] **Community Relay Directory**: Decentralized list of public relay instances
- [ ] **Transfer History**: Local, encrypted log of sent/received files
- [ ] **Clipboard Sharing**: Send text snippets and URLs alongside files
- [ ] **Plugin API**: Extend with custom transfer hooks and integrations

---

## 🐳 Docker Deployment (Planned)

Nhako Beam is designed to be self-hosted in seconds. The relay server ships as a lightweight Docker image.

### Quick Start

```bash
# Pull and run the relay
docker run -d \
  --name nhako-relay \
  -p 8080:8080 \
  -p 3478:3478/udp \
  -p 3478:3478/tcp \
  -v nhako-data:/data \
  -e RELAY_AUTH_MODE=token \
  ghcr.io/yourorg/nhako-beam-relay:latest
```

### Docker Compose (Recommended)

```yaml
version: '3.8'

services:
  relay:
    image: ghcr.io/yourorg/nhako-beam-relay:latest
    container_name: nhako-relay
    restart: unless-stopped
    ports:
      - "8080:8080"        # WebSocket signaling
      - "3478:3478/udp"    # QUIC relay
      - "3478:3478/tcp"    # TCP fallback
    volumes:
      - ./relay-data:/data
    environment:
      - RELAY_MAX_BANDWIDTH=1gbit
      - RELAY_BUFFER_SIZE=512MB
      - RELAY_AUTH_MODE=token
      - RELAY_LOG_LEVEL=info
    networks:
      - nhako-net

networks:
  nhako-net:
    driver: bridge
```

### Deployment Models

| Model | Use Case | Setup |
|-------|----------|-------|
| **Private Relay** | Personal / family use | Single Docker container on home server or VPS |
| **Community Relay** | Small teams or friend groups | Shared instance with token-based access |
| **Public Relay** | Open ecosystem | Listed in community directory with rate limits |

---

## 🛠️ Tech Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| **Protocol Core** | Rust | Memory safety, async performance, single-binary deployment |
| **Relay Server** | Go or Rust | High concurrency, small Docker image, native QUIC support |
| **Desktop UI** | Tauri (Rust + Web) | Tiny bundle size, native OS integration, cross-compile to Win/Linux |
| **Android** | Kotlin + Jetpack Compose | Native performance, system share sheet, foreground services |
| **Transport** | QUIC (UDP) + TCP fallback | Modern, fast, NAT-friendly with built-in security |
| **Crypto** | Noise Protocol / rustls | Lightweight, formally verified, no certificate authority required |
| **Discovery** | mDNS (zeroconf) + WebSocket | LAN discovery without infrastructure; WAN via relay |
| **Checksums** | BLAKE3 | Fast, parallelizable, cryptographically secure |

---

## 📂 Repository Structure (Planned)

```
nhako-beam/
├── relay/                  # Dockerized relay + signaling server
│   ├── Dockerfile
│   ├── docker-compose.yml
│   └── src/
├── core/                   # Shared Rust library (protocol, crypto, chunking)
│   └── src/
├── desktop/                # Tauri desktop application
│   ├── src-tauri/          # Rust backend
│   └── src/                # Web frontend (Svelte/Vue)
├── android/                # Kotlin Android application
│   └── app/
├── docs/                   # Architecture docs, protocol specs, deployment guides
├── .github/
│   └── workflows/          # CI/CD for Docker images, releases, and cross-compilation
├── LICENSE
└── README.md
```

---

## 🗺️ Roadmap

| Phase | Milestone | Target |
|-------|-----------|--------|
| **0** | Protocol specification & Rust core scaffold | Q4 2026 |
| **1** | Relay server + Docker image + basic signaling | Q4 2026 |
| **2** | Linux desktop MVP (send/receive, CLI) | Q1 2027 |
| **3** | Windows desktop parity | Q1 2027 |
| **4** | Android MVP (share sheet, basic transfers) | Q2 2027 |
| **5** | LAN direct connect (mDNS) + resume polish | Q2 2027 |
| **6** | Public beta + community relay network | Q3 2027 |
| **7** | v1.0 stable release | Q4 2027 |

---

## 🤝 Contributing

Nhako Beam is in active planning and we welcome contributors of all skill levels.

### How to Contribute

1. **Star & Watch** the repository to stay updated
2. **Open an Issue** to discuss features, report bugs, or ask questions
3. **Review the Architecture** — feedback on the protocol and security model is especially valuable at this stage
4. **Submit a PR** once scaffolding begins

### Areas We Need Help With

- 🔐 Cryptographic protocol review
- 🌐 NAT traversal and ICE implementation
- 🎨 UI/UX design for desktop and Android
- 🐳 Docker and DevOps optimization
- 📖 Documentation and protocol specifications

Please read our [Contributing Guide](CONTRIBUTING.md) (coming soon) and [Code of Conduct](CODE_OF_CONDUCT.md) before submitting.

---

## 📜 License

Nhako Beam is licensed under the **MIT License**.

```
MIT License

Copyright (c) 2026 Nhako Beam Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

## 🙏 Acknowledgments

- Inspired by the direct-transfer philosophy of applications like **Blip**
- Built on the shoulders of open protocols: **QUIC**, **Noise**, **BLAKE3**, **WebSocket**
- Thanks to the Rust, Go, and open-source communities for the incredible tooling

---

<div align="center">

**[⭐ Star this repo](https://github.com/yourorg/nhako-beam)** to follow our progress from planning to production.

*Built with ⚡ by the community, for the community.*

</div>
