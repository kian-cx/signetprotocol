# Signet Protocol · beta

[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![SDK](https://img.shields.io/badge/SDK-0.1.0--beta.1-orange)](CHANGELOG.md)
[![Protocol](https://img.shields.io/badge/protocol-Signet%2F1-purple)](docs/content/protocol/index.mdx)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen)](CONTRIBUTING.md)
[![X](https://img.shields.io/badge/X-@kian__cx-black)](https://x.com/kian_cx)

**Every game, one world.** Signet Protocol is an open protocol and SDK that lets players with *different* games join the same world. A neutral server owns the geometry, the rules and the bodies; every player sees and controls it with **their own** game through a translator.

The goal is a **global protocol for communication between all games**: any game can join any world, and viewers, mechanics and content can be exchanged between games, so a player can bring their own game to someone else's world and the other way around.

This repository contains the SDK to write those translators, plus the clients and the server that use them.

| Part | Path | Status |
|---|---|---|
| Rust core (`signet-sdk`): Signet/1 protocol, rules, prediction, client, native worlds, translators, conformance | `crates/signet-sdk` | beta |
| C interface (`signet.h` + `signet.dll/.so/.dylib`) | `crates/signet-ffi` | beta |
| C# (Unity / .NET) on top of the C interface | `bindings/csharp` | preview |
| TypeScript / Node (`@signet/sdk`): types and a client without prediction | `bindings/typescript` | preview |
| Dedicated server (Docker) | `servidor` | beta |
| Documentation (Next.js) | `docs` | beta |

## Five minutes

```bash
cargo run --release --example cliente_minimo -- <server-ip>      # joins, walks and reports
cargo run --release --example visor_ascii   -- <server-ip>      # a viewer in your terminal
cargo run --release --example conformidad   -- --servidor <server-ip>
```

Host a server:

```bash
docker build -f servidor/Dockerfile -t signet-server .
docker run -d --name signet -p 7777:7777 -e SIGNET_WORLD=signet:nexo signet-server
```

Read the documentation online at **[signetprotocol.io](https://signetprotocol.io)**, or locally:

```bash
cd docs && npm install && npm run dev   # http://localhost:3000
```

## Principles

1. **The server is in charge.** Clients send intents; the server decides positions, hits and deaths.
2. **Only neutral data travels.** Heights, positions, ids and events go over the wire, never a game's files.
3. **Every player uses their own copy.** Translators read the player's own game files. Neither the SDK nor the servers distribute third-party content.
4. **No cheating, no injection into online games.** Only open-source engines, officially allowed mods, gateways over servers you control, or reimplementations.

## A note on naming

The Signet/1 wire vocabulary is in Spanish (`Intencion`, `avance`, `Terreno`…), and so are the Rust identifiers that mirror it. The documentation explains every field in English, and the C, C# and TypeScript APIs are in English. An English wire vocabulary is planned for Signet/2.

## Versions

| Component | Version |
|---|---|
| SDK (`signet-sdk`, `signet-ffi`, `@signet/sdk`) | **0.1.0-beta.1** |
| Wire protocol | **Signet/1** (append-only: it only grows with optional fields) |

See the [CHANGELOG](CHANGELOG.md). During the beta, minor versions may change the SDK API; the Signet/1 wire format stays compatible.

## Community: contributions are welcome

Signet Protocol is meant to be built **by everyone who wants their game to talk to every other game**. You do not need to ask for permission to start.

- **Write a translator:** an importer, a viewer or a gateway for a game or engine ([guides](docs/content/guides)).
- **Share a world, a game mode or a mechanic** so every game can play it.
- **Improve the protocol** with a proposal (see [GOVERNANCE.md](GOVERNANCE.md)).
- **Report bugs, test on your engine, translate the docs.**

Where to talk:

| What | Where |
|---|---|
| Questions, ideas, "how should we run this?" | [GitHub Discussions](https://github.com/signetprotocol/signet/discussions) |
| Bugs and concrete proposals | [Issues](https://github.com/signetprotocol/signet/issues/new/choose) |
| News and direct contact with the maintainer | [@kian_cx on X](https://x.com/kian_cx) |

Start with [CONTRIBUTING.md](CONTRIBUTING.md). Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md). For security problems, see [SECURITY.md](SECURITY.md).

## License: use it, fork it, build on it

The whole SDK, the protocol specification and the server are released under **[Apache-2.0](LICENSE)**. Anyone can use them, including commercially, modify them, redistribute them and base their own project on them, as long as they keep the license and copyright notices (see [NOTICE](NOTICE)). Apache-2.0 also gives you an explicit patent license from the contributors.

Each translator you write can use its own license and declare it in its `signet.json`.

"Signet Protocol" is a trademark of its owner. You may say your software is *compatible with Signet/1* once it passes the [conformance suite](docs/content/sdk/conformance.mdx); please do not present a modified fork as the official project. Games, their trademarks and their files belong to their respective owners.
