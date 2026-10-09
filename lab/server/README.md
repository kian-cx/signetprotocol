# Signet doors server (lab)

This is the lab server used to put unmodified games in one shared world (Minecraft, Garry's Mod, Lethal Company). It is **not** the SDK dedicated server in [`servidor/`](../../servidor).

Two different things are both called Signet/1 today:

| | SDK (`crates/signet-sdk`, port 7777) | This lab server (port 7800) |
|---|---|---|
| Wire | JSON lines, Spanish field names (`Intencion`, `Terreno`…) | Tab-separated lines: `J` `M` `A` `L`, terrain `W`/`B`, edits `E` |
| Who draws | A translator in the SDK sense | A **door** on the player's PC, after **Signet Forge** picks an id from that game's own catalog |
| Image | `docker build -f servidor/Dockerfile` | `docker build -t signet-doors lab/server` |

The decision model this server's Forge screen uses is [`models/signet-forge-dm-qwen35-0.8b-clef-distill-v1`](../../models/signet-forge-dm-qwen35-0.8b-clef-distill-v1). The screen itself (`signet forge`) still runs from the Python program next to the catalogs; those catalogs are extracted from each player's game and are **not** in this repository.

Game captures, host scripts and catalogs stay on the machine that owns the game. Do not add them here.

# Signet Server

One shared world, one **door** per game. Each player connects with their own, **unmodified** game; the server keeps
**universal entities** (what it is, which game it comes from, how it looks, where it is) and every door draws the
others its own way. The server never translates: the receiving game's Signet Forge decides how to draw things from
its own catalog.

Rust, standard library only (no downloads). One binary, `signet`, three modes:

- **server**: only the shared world. No doors, no game data: just universal entities and events.
- **link**: runs on each player's PC, next to their game. Its doors listen on 127.0.0.1 only; its uplink talks to a
  Server with the Signet protocol (`src/wire.rs`: one universal event per tab-separated line over TCP).
- **forge**: the decision screen on that same PC. The binary starts it; the model and the catalogs stay in
  `~/clm-bench` (Python and the GPU). `SIGNET_PYTHON` and `SIGNET_FORGE` override those paths.

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

$B                                              # local world: both doors on this PC (no Server)
$B server --bind 10.8.0.1:7800                  # the Server, on its VPN address
$B link --server 10.8.0.1:7800 --name kian      # a player's PC: its doors + uplink
$B forge                                        # this PC's decision screen (port 7796)
$B link --server 10.8.0.1:7800 --no-lc          # only the Minecraft door
$B link --no-mc --lc-port 17777                 # LC door on a test port
```

Flags: `--bind`, `--server`, `--name`, `--no-mc`, `--no-lc`, `--mc-bridge 127.0.0.1:7790`, `--lc-port 7777`,
`--lc-script FILE`.

- **Minecraft:** start the proto (`~/minecraft-signet/proto`), join `localhost:25568`.
- **Lethal Company:** close any LC host (Signet *is* the host on 7777), start the game in **LAN** mode and press
  **Join**. LAN "Join" always connects to 127.0.0.1:7777 and only sends the game version (no Steam identity).

## Signet Forge, one per PC

The doors never decide how to draw what comes from another game: the Link asks the PC's Signet Forge
(`~/clm-bench/signet_forge.py`, 127.0.0.1:7796) and each door draws a neutral default until it answers (armor stand,
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
cd ~/clm-bench && HF_HUB_OFFLINE=1 clef/.venv-unsloth/bin/python signet_forge.py      # live screen
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
(`~/lc-analisis/host_script.py lc.pcap > ~/lc-analisis/lc_host_script.txt`; derived from the game, never committed):
ConnectionApproved, the ship synchronisation, then the lobby paced like the real host. TimeSync is generated live.
The client's position, rotation, animation and chat are read; its player becomes a universal entity that the
Minecraft door draws.

Phase 2: spawn the other games' players in the ship from universal events (CreateObject + OnPlayerConnected with
the player prefab), so Lethal Company players see the Minecraft player drawn with the suit Forge chose.
