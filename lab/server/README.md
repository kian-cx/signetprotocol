# Signet Server

One shared world, one **door** per game. Each player connects with their own, **unmodified** game; the server keeps
**universal entities** (what it is, which game it comes from, how it looks, where it is) and every door draws the
others its own way. The server never translates: the receiving game's Signet Forge decides how to draw things from
its own catalog.

Rust, standard library only (no downloads). One binary, `signet`, three modes:

- **server**: only the shared world. No doors, no game data: just universal entities and events.
- **link**: runs on each player's PC, next to their game. Its doors listen on 127.0.0.1 only; its uplink talks to a
  Server with the Signet protocol (`src/wire.rs`: one universal event per tab-separated line over TCP).
- **forge**: the decision screen on that same PC. Paths come from `signet setup`, kept in the data
  directory (`$SIGNET_HOME`, else `~/.local/share/signet` on Linux, `~/Library/Application Support/signet` on macOS, `%LOCALAPPDATA%\signet` on Windows).
- **setup**: detects the system and records whether this machine is a server, a client, or both.

## Platforms

The wire, the ports and the commands are the same on Linux, macOS and Windows. Build with `cargo build --release` on the machine that will run it, or run the server in Docker (the image is Linux; Docker Desktop provides that on macOS and Windows).

| | Linux | macOS | Windows |
|---|---|---|---|
| Server | binary or Docker | binary or Docker Desktop | binary or Docker Desktop |
| Link | same binary, doors on `127.0.0.1` | same | same; allow `signet.exe` on localhost if the firewall asks |
| Data directory | `~/.local/share/signet` | `~/Library/Application Support/signet` | `%LOCALAPPDATA%\signet` |
| Override | `SIGNET_HOME` on all three | | |
| Python for Forge | `python3` | `python3` | `python` or `python3` |
| venv interpreter | `venv/bin/python` | `venv/bin/python` | `venv\Scripts\python.exe` |
| Model download | `curl`, or Python if curl is absent | same | same (Windows 10 and later include `curl.exe`) |
| Forge GPU | NVIDIA driver | no NVIDIA GPU; doors keep the neutral stand-in | NVIDIA driver |
| Ctrl-C | stops and cleans up | same | same (Ctrl-C; closing the console is not a second signal) |

`signet setup` prints which of these it found. Server mode never downloads the model. Client mode downloads it only with `--download`.

```
 player A's PC                                                          anywhere (this PC, LAN, cloud VM)
 Minecraft ──► proto ──7790──► Link [door minecraft] ─┐
                                                      ├── Signet protocol ──► Server (world)
 player B's PC                                        │   through YOUR tunnel
 Lethal Company ──LAN Join 127.0.0.1:7777──► Link [door lethalcompany (host)] ─┘
```

**The tunnel is up to each user.** Signet does not ship a VPN: put the Server wherever you want and reach it through
your own OpenVPN / WireGuard / Tailscale. The Server listens on the address you give it (default 127.0.0.1); bind it to
the VPN address, not to a public one. Optional shared token: `SIGNET_TOKEN` on the Server and on every Link.

Ids: the Server numbers each Link (n); a Link's entities travel as `n << 32 | local id`, and a Link may only publish
events about its own entities. When a Link drops, its entities leave; when it comes back it re-sends them.

**Adding a game:** see [docs/ADDING_A_GAME.md](docs/ADDING_A_GAME.md) (game card, catalog for Signet Forge, door in the Link, the way into the game, tests, and where each game stands).

## Run

```
cargo build --release
B=./target/release/signet

$B setup --role server                          # this machine only hosts the world
$B setup --role client --games gmod --download  # Link + Forge; fetches the 1.8 GB model
$B server --bind 10.8.0.1:7800                  # the Server, on its VPN address
$B link --server 10.8.0.1:7800 --name PLAYER    # a player's PC: its doors + uplink
$B forge                                        # this PC's decision screen (port 7796)
```

Flags: `--bind`, `--server`, `--name`, `--no-mc`, `--no-lc`, `--mc-bridge 127.0.0.1:7790`, `--lc-port 7777`,
`--lc-script FILE`.

- **Minecraft:** a proto you run yourself must speak the bridge on `127.0.0.1:7790`. Join that server from the
  official client. The proto is not part of this install.
- **Lethal Company:** the door stays off until you pass `--lc-script` (or save one with `signet setup --lc-script`).
  The script comes from a capture of your own LAN game. Signet does not ship it. Then start the game in **LAN**
  mode and press **Join** (`127.0.0.1:7777`).

## Signet Forge, one per PC

The doors never decide how to draw what comes from another game: the Link asks the PC's Signet Forge
(127.0.0.1:7796) and each door draws a neutral default until it answers (armor stand,
stone). SF decides with the student DM over the receiving game's catalog (made by Signet Forge Catalog), shows every
decision live, and pushes your approvals to the doors at once; they are kept forever (`forge_decisions.json`).

```
Link -> SF   ASK  target kind key name description        target = minecraft_26_3, garry_s_mod, ...
SF -> Link   USE  target kind key choice source p          source = approved | dm
```

| door | asks SF | draws |
|---|---|---|
| Minecraft | `player` (from other games), `material` (the ground) | `SPAWN eid minecraft:<creature> ... label`, `WORLD/BOX/WORLDEND` |
| Garry's Mod | `player` | the addon's prop with that player model |

```
signet setup --role client --download --install-python   # model and the Python environment
signet forge                                          # the screen, using the paths setup saved
signet link --server HOST:7800 --gmod                  # (--forge ADDR, --no-forge)
signet forge                                          # the screen that answers that Link
```

## Docker (the Server only)

```
docker build -t signet .                              # static binary on a scratch image (~1.2 MB), runs as nobody
docker run -d --name signet --restart unless-stopped -p 127.0.0.1:7800:7800 signet
docker run -d ... -p 10.8.0.1:7800:7800 -e SIGNET_TOKEN=... signet   # on a VM: publish on the VPN address
docker logs -f signet
```

Inside the container it listens on 0.0.0.0:7800; *where* it is reachable is decided by `-p` (keep it on 127.0.0.1 or
your VPN address, not a public one). Links run on the players' PCs, outside Docker, next to their games.

## Pieces

| file | what |
|---|---|
| `src/wire.rs` | the Signet protocol: HELLO/WELCOME/DENIED, J/M/A/L events, PING |
| `src/net.rs` | Server (Link sessions, id ownership, token) and the Link's uplink (reconnects) |
| `src/world.rs` | universal entities and events (Joined / Moved / Action / Left); frame: metres, Y up, origin = shared spawn |
| `src/utp.rs` | Unity Transport host side: tokens, ping/pong, reliable pipeline 3 (acks, resends), unreliable pipeline 1 |
| `src/ngo.rs` | Netcode for GameObjects: batches (magic 0x1160 + xxh64), BytePacker integers |
| `src/doors/mc.rs` | Minecraft door over the proto bridge: `MC …`/`MCEV …` in, `LC id x y z yaw` out |
| `src/doors/lc.rs` | Lethal Company door: host emulation |

## Lethal Company door, phase 1

The host's answers come from a **script** made from a local capture of your own LAN game
(a script made from a capture of your own LAN game; derived from the game, never committed):
ConnectionApproved, the ship synchronisation, then the lobby paced like the real host. TimeSync is generated live.
The client's position, rotation, animation and chat are read; its player becomes a universal entity that the
Minecraft door draws.

Phase 2: spawn the other games' players in the ship from universal events (CreateObject + OnPlayerConnected with
the player prefab), so Lethal Company players see the Minecraft player drawn with the suit Forge chose.
