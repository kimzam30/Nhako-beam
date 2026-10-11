# nhako (command line)

Send files between your devices from a terminal. It uses the same engine, codes and protocol as the app, so a terminal and the app can send to each other.

## Build

Rust 1.91 or newer.

```bash
cargo build --release -p nhako-cli     # binary: target/release/nhako (nhako.exe on Windows)
```

It runs on Linux and Windows. On Windows, the config folder is `%APPDATA%\kimzam30\nhakobeam\config` and files go to `Downloads\Nhako Beam`.

## Use

```bash
# Send with a code. Prints three words for the other device.
nhako send holiday.mp4 notes.pdf

# On the other device, type the words.
nhako receive river lantern seven

# Pair once and save each other as trusted (run one on each device).
nhako pair                       # shows a code
nhako pair river lantern seven   # enters it

# Then send by name, no code needed.
nhako send report.pdf --to "Work phone"

# Wait for files from nearby and trusted devices (asks before accepting).
nhako receive

# See who's around.
nhako devices
```

Other commands: `nhako whoami`, `nhako name "Hakim's laptop"`, `nhako trust remove DEVICE`, `nhako trust auto-accept DEVICE on|off`.

## Your own server

Run the Docker setup in [`relay/`](../relay/README.md), then:

```bash
export NHAKO_RELAY_TOKEN=...      # from relay/.env
nhako server set http://192.168.1.20:3340 --lookup http://192.168.1.20:3341
nhako server                      # shows what's saved (never the token)
nhako server check                # does it answer, is the token right
nhako server public               # back to n0's free servers
```

`server set` checks first. A wrong token is not saved; a server that doesn't answer is saved anyway (it may just be off) and the command exits with 1. The desktop app keeps its own settings folder (`~/.config/io.github.kimzam30.nhakobeam`), so set the server there in Settings > Connection. To run the command line as the same device instead, close the app and pass `--config-dir` with that folder.

## Options

| Option | What it does |
|---|---|
| `--local-only` | Same network only. No relay and no internet lookups. Codes still work on the same Wi-Fi. |
| `--relay URL` | Use your own relay for this run only. `--relay-token` (or `NHAKO_RELAY_TOKEN`) and `--lookup URL` go with it. To keep it, use `nhako server set`. |
| `--relay-only` | Never connect directly, always go through the relay. For testing a relay. |
| `--no-mdns` | Don't look for devices on the same network. Useful for testing the internet path. |
| `--save-dir DIR` | Where received files go for this run. The default is `Downloads/Nhako Beam`. |
| `--config-dir DIR` | Where the device key, name and trusted devices live. The default is the platform config folder (`~/.config/nhakobeam` on Linux). Use different folders to run two devices on one machine. |
| `--json` | Print every engine event as one JSON line on stdout, the same events the app gets. |
| `-v` | Debug logs on stderr. |

Exit code is 0 when the transfer finished and 1 otherwise (declined, cancelled, failed, code expired).

## Notes

- Ctrl+C cancels the transfer and tells the other device.
- Folders aren't supported yet. Send the files inside.
- Received files never overwrite anything: a second `photo.jpg` is saved as `photo (1).jpg`.
- Partial files are saved as `name.nhakopart` until their hash checks out.
