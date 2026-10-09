# Changelog

All notable changes are listed here. The SDK follows [Semantic Versioning](https://semver.org/) once it leaves beta. The wire protocol is versioned separately: **Signet/1** only grows with optional fields.

## Unreleased

- **Platforms.** The lab binary uses the system data directory on Linux, macOS and Windows, finds `python` or `python3`, and downloads the model with curl or Python. Docs: `docs/content/getting-started/platforms.mdx`.

- **`signet setup`** detects the OS and the GPU, then asks for server, client, or both. Server use downloads nothing. Client use downloads the decision model and can install the Python environment into the data directory (`$SIGNET_HOME` or `~/.local/share/signet`). No personal paths. The lab crate builds inside this workspace.

- **Lab doors server** in `lab/server`: one binary, `signet`, with `server`, `link` and `forge`. Shared world in metres, one door per game, Forge asked on localhost. This wire is not the SDK Signet/1 protocol in `crates/signet-sdk`.
- **Decision model** `signet-forge-dm-qwen35-0.8b-clef-distill-v1`: Qwen3.5-0.8B with Clef's joint head, distilled from Clef 27B. Weights via Git LFS. Model card in that directory. The Forge page on the site points at it.

## 0.1.0-beta.1 · 2026-10-04

First public release.

- **Protocol:** Signet/1 (JSON lines over TCP, port 7777): welcome, state, events and numbered intents.
- **Rust SDK (`signet-sdk`):** protocol types, game rules, client-side prediction with reconciliation, an engine-agnostic client, native worlds (`.mvm`), translator traits and the conformance suite (T01–T07, R01–R02, M01–M04).
- **C interface (`signet-ffi`)** and **C# binding** (preview, not yet tested in Unity).
- **TypeScript / Node binding (`@signet/sdk`)** (preview, no prediction).
- **Dedicated server (Docker):** `signet:nexo` (native), `doom:E1M1` (Doom 1.9 shareware) and `openarena:<map>` (OpenArena 0.8.8).
- **Documentation** site (Next.js + Nextra).

Known gaps: no jumping in `doom:deathmatch`, 2.5D terrain only, OpenArena shader textures, the Minecraft gateway still sends unnumbered intents.
