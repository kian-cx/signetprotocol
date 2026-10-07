# Contributing to Signet Protocol

Thank you, contributions are welcome! The beta mostly needs **translators**: map importers, viewers and gateways for more games and engines. You do not need permission to start; open an issue or a discussion if you want to talk first.

By taking part you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to help

- **Translators:** importers, viewers and gateways for games and engines.
- **Native worlds and game modes** every game can play.
- **Bug reports and tests**, especially on engines we have not tried (C# in Unity is the biggest gap).
- **Documentation and translations** of it (`docs/content`).
- **Protocol proposals** (see [GOVERNANCE.md](GOVERNANCE.md)).

Not sure where to start? Look for issues labelled `good first issue`, or ask in [Discussions](https://github.com/signetprotocol/signetprotocol/discussions).

## How to contribute

1. Fork the repository and create a branch from `main`.
2. Make your change, small and focused. One idea per pull request.
3. Run the checks below.
4. Open a pull request and fill in the template. Maintainers review as soon as they can; please be patient, this is a young project.

## Before opening a PR

1. `cargo test --release --workspace` passes (CI runs it on every pull request).
2. If you contribute an importer, export a map to JSON and run the conformance suite:
   `cargo run --release --example conformidad -- your_map.json` (codes T01–T07, R01–R02).
3. If you contribute a translator, include its `signet.json` manifest (see `Manifiesto`) and pass codes M01–M04.
4. Follow the repository style: comments explain the *why*. New public documentation is written in English.

## Non-negotiable rules

- **Do not upload any game files** (maps, textures, sounds, models, executables), or links to unofficial copies. Translators read the player's own installation.
- **No injection into online games and no anti-cheat evasion.** Valid integrations are gateways over servers you control, open-source engines, mods the game officially allows, or reimplementations.
- **Respect the game's license and EULA.** If it is unclear whether an integration is allowed, open it as a proposal before writing code.

## Protocol changes

Signet/1 only grows with **optional fields that have a default value**, so older clients and servers keep working. A breaking change requires Signet/2 and an approved proposal. The process is described in [GOVERNANCE.md](GOVERNANCE.md).

## Contact

- Questions and ideas: [GitHub Discussions](https://github.com/signetprotocol/signetprotocol/discussions)
- Maintainer on X: [@kian_cx](https://x.com/kian_cx)

## License

By contributing you agree to publish your contribution under Apache-2.0. Contributions are accepted under the terms of section 5 of the license (inbound = outbound); no separate agreement is needed.
