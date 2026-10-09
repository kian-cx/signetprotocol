# Adding a game to Signet

What a game needs before its players can share the Signet world, what each piece does, and where every game stands
today.

**Short answer:** a catalog for Signet Forge and a door in the Signet Link are the two pieces every game needs. Most
games need two more:

- a **way into the game** that we control: our own server, the game's own dedicated server with an addon, or an
  emulated host;
- a **coordinate mapping**: units, axes, yaw and spawn.

Before any of that, the game must pass the rules check on its game card.

```
 the game  ◄──►  [3] way in (ours)  ◄──►  [2] door, in the Link  ◄──►  Signet Server (shared world)
                                              │
                                              └── asks ──►  Signet Forge  ◄──  [1] catalog of the game
```

## 0. Game card: may we do it at all?

`python3 ~/clm-bench/catalog_kit.py` writes `catalogs/card_<game>.json`, which Signet Forge Catalog shows at step 2.
The card's **tier** decides the path:

| Tier | Meaning | Way in |
|---|---|---|
| A | We can be the server: our own server, or the game's dedicated server | Our server, or the dedicated server plus an addon |
| B | One player hosts, over a known network library | Emulate the host with one adapter per library, built from a capture of our own LAN game |
| C | Unknown or closed netcode | Research first, no promise |
| out | Anti-cheat, or cloud only | **Never.** No injection, no evasion. VAC games only on our own LAN servers. |

Rules for every game:
- 127.0.0.1 only;
- no changes to the game's files or memory;
- game data (captures, catalogs, generated lists) stays on the PC and is never committed.

## 1. Catalog, for Signet Forge

**What it is:** everything the game can draw, each entry with the id the game uses to create it. Signet Forge picks
from this list when it decides how this game draws things from other games.

**How it is made:**
1. Signet Forge Catalog runs a **reader for the game's engine**: `~/clm-bench/readers/<engine>.py`, contract in
   `readers/CONTRACT.md`.
2. It writes `~/clm-bench/catalogs/catalog_<game>.json`.
3. Hand descriptions go in `catalogs/descriptions_<game>.json`.

A new engine needs a new reader. The reader must pass `python -m readers.check readers/<x>.py "<Game>"` before it is
used.

```bash
cd ~/clm-bench && uv run --no-project --python 3.13 --offline --with UnityPy --with rich python signet_catalog.py
```

**Kinds Signet Forge asks for:**

| Signet Forge asks | Catalog kind it uses | Used for |
|---|---|---|
| `player` | `player`, or `creature` if the game has no player models | How a player from another game looks here |
| `material` | `structure` (blocks, brush surfaces…) | How the shared ground is painted here |

Watch for gaps. If the catalog lacks something the world uses, add it as an extra. Example: Minecraft's block list
has no grass, leaves, water or lava, so `MC_TERRAIN` in `signet_forge.py` adds them. Also drop entries the game
cannot actually show; example: GMod surfaces whose texture is missing.

**After the first run:** Signet Forge's DM proposes a choice for each material and player look. You approve or change
it once in Signet Forge's screen, and it is kept forever in `forge_decisions.json`. This is not code, but a game
looks right only after it is done.

## 2. Door, in the Signet Link

**What it is:** a Rust module `src/doors/<game>.rs` in this repo, started by a Link flag (`--gmod`, `--no-mc`…).

**Where it hooks in:**
- start it in `src/main.rs` (link role);
- declare it in `src/doors/mod.rs`;
- if it has a Signet Forge target, add the target name, e.g. `garry_s_mod`.

**What a door does:**

| Job | Minecraft (`mc.rs`) | GMod (`gmod.rs`) |
|---|---|---|
| Talk to the way in | TCP bridge to our proto, port 7790 | Local HTTP on 7795, polled by the addon |
| Map coordinates | 1 block = 1 m; spawn (0.5, 64, 0.5) = origin | Source units × 0.0254; axes (x, z, −y); yaw −90 − yaw |
| Report this game's players | `MC …` lines → Joined/Moved/Left | `P …` lines → Joined/Moved/Left |
| Draw other games' players | Asks Signet Forge `player`; `SPAWN eid minecraft:<creature>` | Asks Signet Forge `player`; the addon spawns that model |
| Paint the shared ground | Asks Signet Forge `material`; `WORLD/BOX/WORLDEND` | Asks Signet Forge `material`; `/world` boxes → `signet_box` |
| Ground edits out | Placed or broken block → `Event::Edit` | Physgun carry → `Event::Edit` (from + to) |
| Ground edits in | `PATCH` of the changed region only | Rebuilds only the boxes that changed |
| Clean exit (Ctrl-C) | `REMOVE` drawn players, `SAY` | The addon removes props when the Link stops answering |

**Rules:**
- the door never decides how to draw something: it asks Signet Forge and draws a neutral default until Signet Forge
  answers;
- it only publishes its own entities;
- it listens on 127.0.0.1 only.

## 3. Way in: something we control that the game connects to

The door cannot talk to an unmodified game by itself. Something the game already accepts has to stand in between,
and the card's tier decides what it is:

| Way in | Example | What we write |
|---|---|---|
| **Our own server** speaking the game's protocol | Minecraft: `~/minecraft-signet/proto` (Rust, `localhost:25568`) | A minimal server: login, chunks, entities, block events, and a text bridge to the door |
| **The game's dedicated server plus an addon**, using the game's official mod API | GMod: srcds on 127.0.0.1 with `sv_lan 1`, plus the Lua addon `gmod/signet` | The addon (polls the door, spawns props and boxes, reports players and the physgun) and the launcher `gmod/run_server.sh` |
| **Emulated host** (tier B) | Lethal Company: `src/doors/lc.rs` is the host on UDP 7777 | A transport/netcode adapter (`utp.rs`, `ngo.rs`) and a host script built from a capture of our own LAN game |

## 4. Tests

- **End-to-end test with a fake way in**, as in `tests/e2e.py`, `e2e_late.py` and `e2e_edits.py`. It must cover:
  players both ways, the ground painted, edits both ways, and a Link that joins late. Logs go to `SIGNET_TEST_DIR`.
- **A unit test of the coordinate mapping** (round trip).
- **A live check:** both games open side by side, a recording, and the crops in `~/signet-media`.

## Where each game stands (2026-10-08)

| Game | Tier | Catalog | Door | Way in | Sees others | Seen by others | Ground | Edits |
|---|---|---|---|---|---|---|---|---|
| Minecraft 26.3 | A | ✔ 92 creatures, 516 blocks + terrain extras | ✔ `mc.rs` | ✔ our proto | ✔ | ✔ | ✔ | ✔ both ways |
| Garry's Mod | A | ✔ 86 NPCs, 87 player models, 1893 surfaces | ✔ `gmod.rs` | ✔ LAN srcds + addon | ✔ | ✔ | ✔ | ✔ both ways |
| Lethal Company | B | ✔ 33 creatures, 7 suits | ◐ `lc.rs` phase 1 | ◐ host emulation | ✘ (phase 2) | ✔ | ✘ | ✘ |
| The Forest | B | ✔ 35 creatures, 226 items, 163 structures, 1 player | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |

Lethal Company is not in the current demo; the Link runs with `--no-lc`.

## Next: The Forest

What we know:
- Unity 5.6.5p4 with Photon Bolt over udpkit.
- One player hosts through Steam.
- The decompiled code is in `~/forest-analisis` (local only).
- The catalog is done.

Found in the decompiled code (2026-10-08). Sources are local only: `~/forest-analisis/acs`, plus `bolt`, `udpkit` and
`udpkit_common`, made with ilspycmd.

- **The client can join a dedicated server over plain UDP.**
  - The game's server browser has a **LAN** tab, which uses Steam's LAN list (`RequestLANServerList`). It accepts
    app 242760 or 556450, the Dedicated Server.
  - On join the client uses Bolt with `DotNetPlatform`, which is a normal UDP socket, not the Steam relay. It
    connects to the server's address with a `CoopJoinDedicatedServerToken` (server password, admin password, Steam
    ticket blob). In `CoopSteamClientStarter.Connect` and `CoopSteamNGUI.OnClientNewGameDS`.
- **The official dedicated server requires Steam authentication for every client.**
  - `CoopServerCallbacks.Connected` calls `SteamGameServer.SendUserConnectAndAuthenticate` and drops the connection
    with "No Steam Auth" when it fails.
  - So we cannot put other games' players into the official server as extra clients, and we will not fake Steam
    tickets.
- **Way in, then: the Link is the dedicated server**, an emulated host like Lethal Company's.
  - To be listed, it answers Steam's LAN query.
  - The player's real client connects to it on 127.0.0.1; as the server, we decide whom to accept.
  - The udpkit layer is small: commands CONNECT 1, ACCEPTED 2, REFUSED 3, DISCONNECTED 4, PING 5 (`UdpConnection`).
  - Bolt's entities, events and states come from `bolt.dll` plus `bolt.user.dll` (prefabs, state serializers).
- **What the script is built from:** a capture of one session between our own client and the official Dedicated
  Server on 127.0.0.1, as with Lethal Company.
  - The Dedicated Server is Steam app 556450, Windows only, about 1.7 GB.
  - It runs under Proton/Wine with a LAN config.

Missing:

1. **Way in:**
   - install the Dedicated Server on LAN and capture one join;
   - write the udpkit/Bolt host (`forest.rs`, plus a host script made from the capture).
2. **Drawing others without an addon.** The Forest has no official mod API. Bolt can only create its
   `BoltPrefabs` (635 in the catalog's network section), so other players have to be drawn as one of those, such as
   a player or a cannibal (`IMutantState.ai_mask` picks the variant). Signet Forge needs to choose from the network
   prefabs, not from every item.
3. **Coordinate mapping:** Unity is left-handed, so mirror X, as with Lethal Company.
4. **Ground:** The Forest has a fixed island. Painting the shared ground there means placing Bolt structures
   (`structure` in the catalog), probably only a few. Expect "players only" first.
5. **Door `src/doors/forest.rs`**, a `--forest` flag, and an e2e test with a fake Bolt client.
