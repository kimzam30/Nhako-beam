# Nhako Beam server (Docker)

Your own relay and address lookup server, so your devices don't depend on n0's public ones.

| Service | What it does | Port |
|---|---|---|
| `relay` | Passes encrypted traffic between two devices when they can't connect directly. It can't read your files: they are end to end encrypted between the two devices. With a token, only your devices can use it. | 3340 (TCP) |
| `lookup` | Lets devices find each other by device key ("device X can be reached through relay Y"). Records are signed by each device, so the server can't fake them. | 3341 (TCP) |

Both are n0's own open source servers (`iroh-relay` and `iroh-dns-server` 1.3.0), built from crates.io at a pinned version. Nothing here costs money: it runs on any computer with Docker.

## Start it

You need Docker with Compose (Docker Desktop on Windows, or Docker Engine on Linux).

```bash
cd relay
cp .env.example .env
openssl rand -hex 24          # copy the output into .env as NHAKO_RELAY_TOKEN
docker compose up -d --build  # the first build takes about 4 minutes
docker compose ps             # both should say "Up"
```

Quick check from the same computer:

```bash
curl http://localhost:3340/ping          # relay: 200
curl http://localhost:3341/healthcheck   # lookup: OK
```

## Point your devices at it

Use this computer's address on your network (for example `192.168.1.20`; `ip addr` on Linux, `ipconfig` on Windows). `localhost` only works on the computer running Docker.

**App:** Settings > Connection > Your own. Relay address `http://192.168.1.20:3340`, the token from `.env`, lookup address `http://192.168.1.20:3341`. Save checks both servers first and tells you if the token is wrong.

**Command line:**

```bash
export NHAKO_RELAY_TOKEN=...   # from .env, so it stays out of your shell history
nhako server set http://192.168.1.20:3340 --lookup http://192.168.1.20:3341
nhako server check             # any time later
nhako server public            # back to n0's servers
```

Every device that should reach the others over the internet needs the same setting. Devices on the same Wi-Fi find each other either way.

To prove a transfer really goes through your relay, add `--relay-only` (no direct paths) and `--no-nearby` on both sides:

```bash
nhako --relay-only --no-nearby send big.iso          # on one device
nhako --relay-only --no-nearby receive WORD WORD WORD # on the other
```

## Everyday

```bash
docker compose logs -f relay      # what it's doing
docker compose up -d              # after changing .env
docker compose up -d --build      # after changing config/ (it's built into the image)
docker compose down               # stop (lookup records are kept in a volume)
docker compose down -v            # stop and forget the lookup records
```

If your firewall blocks other devices, allow TCP 3340 and 3341 on this computer. Change the ports in `.env` if something else uses them.

## Files

| File | What |
|---|---|
| `Dockerfile` | Builds both servers into one small image (distroless, runs as a non-root user). |
| `docker-compose.yml` | Runs them as two services, read-only, with no extra Linux capabilities. |
| `config/relay.toml` | Relay: plain HTTP on 3340, token required. |
| `config/lookup.toml` | Lookup: HTTP on 3341, publishing rate limited per IP address. |
| `.env.example` | The token and the ports. Copy to `.env`, which is git-ignored. |

To move to a newer iroh, change `IROH_VERSION` in the Dockerfile to the version in the app's `Cargo.lock` and rebuild. Both sides should run the same version.

## What this setup is for, and what it isn't

- **It's for your own computer or home network.** Everything is plain HTTP. That's fine inside your network, because the files themselves are always end to end encrypted, but the token travels in the clear.
- **Going public** (devices on other networks using it) needs the server reachable from the internet and HTTPS, which means a domain name and either a server with a public address or port forwarding on your router. A rented server or domain costs money, so that step waits for your go ahead. The relay supports Let's Encrypt certificates, so the change is a config switch plus opening ports 80, 443 and 7842/udp, not new code.
- **Limits:** the lookup server limits publishing per IP address, not lookups. Code guessing is held back by the cost built into codes (see `docs/protocol/nhako-1.md`), not by this server.
