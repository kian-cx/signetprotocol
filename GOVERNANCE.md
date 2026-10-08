# Governance

Signet Protocol is a young open project. Today it has **one maintainer**, [@kian-cx](https://github.com/kian-cx) ([X](https://x.com/kian_cx)), who owns the name and has the final say while the community forms. The goal is to grow the maintainer group and share decisions as people contribute regularly.

This document is a starting point and is open to change: if you think we should run the project differently, open a [Discussion](https://github.com/signetprotocol/signet/discussions) and propose it.

## Roles

- **Contributor:** anyone who sends an issue, pull request, translator or documentation.
- **Maintainer:** can review and merge. People who contribute regularly and follow the project rules can be invited by the existing maintainers.
- **Translator author:** owns their translator, its license and its `signet.json`; they do not need to be a maintainer.

## How decisions are made

- Small changes (fixes, docs, new translators): a maintainer review is enough.
- Changes to the SDK API: discussed in the pull request; wait a few days for feedback when it is not urgent.
- **Protocol changes** follow the proposal process below.

## Protocol proposals

1. Open an issue with the *Protocol proposal* template: use case, new fields with their default values, effect on existing translators.
2. Discussion is open for at least **14 days** so other engines can weigh in.
3. A maintainer accepts or rejects it with a written reason.
4. Accepted changes to Signet/1 must be **additive**: optional fields with a default, never renaming or removing. Breaking changes go to Signet/2.

## Compatibility and naming

- You can say your software is *compatible with Signet/1* once it passes the conformance suite.
- Forks and derived projects are welcome (Apache-2.0). Please do not present a modified fork as the official Signet Protocol, and pick your own name for it.

## Open questions for the community

These are not decided yet and we would like your input:

- Should the protocol live under a foundation or neutral organisation as it grows?
- How do we choose maintainers and review proposals fairly across engines?
- Signet/2: an English wire vocabulary and what else should change.
- Where should official translators and shared worlds live?
