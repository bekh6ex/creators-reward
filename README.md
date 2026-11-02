# Creators Reward

A privacy-first donation platform: viewers donate to the creators they read, what they read
decides how their own donation is split, up to a monthly limit they set, and nobody — not the
creator, not the payment provider, not the operator — can learn who read or funded whom.

**Status: pre-alpha.** Nothing here is production-ready: storage is in memory, the payment
provider is a stub, several known shortcuts (plaintext creator passwords, a hardcoded dev admin
token, no website ownership check) are documented in the code, and the API is not stable.

Repository layout, conventions, and development commands: [`AGENTS.md`](AGENTS.md).
Motivation: [`ABOUT_PROJECT.md`](ABOUT_PROJECT.md).

## Values

  * **Every transaction is a donation.** Nothing is ever gated behind payment: no
    metered access, no "unlock", no feature a Viewer loses by giving less. What a
    Viewer reads decides how much they donate, up to the monthly limit they set;
    once the total reaches the limit, every share is scaled down proportionally, so
    from then on reading only decides the split. It never creates a debt: any item
    can be cancelled until the month is finalised.
  * **A Viewer may set their monthly limit to 0 and everything works identically.**
    Same pages, same events, same history, same dashboards. The amount is invisible
    to every part of the product except the payout itself.
  * **A Creator can never learn whether a Viewer donates to them.** Not from the
    portal, not from statistics, not from any API, and not by inference from a
    payout. There is no "supporter" flag, anywhere, ever.
  * Viewer's privacy is the top priority
  * Viewer decides how much and whom to donate to
  * When Viewer starts reading new media registered on our platform, we store
    the events, but before any donation is sent allow the Viewer to specify the
    amounts/limits
  * We cannot decrypt Viewer data. Support receives only the event and user metadata — no user id is ever transmitted; anonymity holds by construction.
  * There is no time-boxed consent mechanism.
  * We do not ask a Viewer to provide information that we don't need (e.g. Profile Picture)
  * We do not ask to verify the identity, unless the law requires it. Identity checks
    belong to the money leg only and must never be joinable to
    the reading events.

## Potential creators

  * Independent/Small Media websites
  * Wikipedia authors 
  * Mobile application developers — specifically **open-source app and game
    developers**, who have no advertising inventory, no subscription tier, and
    nothing to gate.
  * Documentation authors and open-source maintainers — the constituency an AI
    agent actually reads.

## Requirements

  * **The goal:** it should not be possible to track what a Viewer reads
    * Not exact articles
    * Nor which media
    * **Nor through the payment layer.** No record may connect a Viewer to a Creator
      they donated to. The money model is therefore **pool-and-distribute**: many
      inbound donations, independent per-Creator payouts, no linking reference on
      either leg. Split-payment / destination-charge products are forbidden — they
      build that link by construction.
    * **The adversary includes us under legal compulsion**, not only outsiders. The
      aim is *inability*, not policy: we cannot be made to produce what we do not
      hold.
    * **If the goal turns out to be impossible** for some part of the system, we
      store as little as possible (nothing stored can be attributed to a Viewer) and
      make any remaining correlation controllable: possible only for a small group
      of people holding cryptographic keys.
    * **Benchmark:** donating to an adult-content Creator — or to a site that implies
      your sexuality, health, religion, or politics — must not be discoverable by the
      Creator, by us, by our payment provider, or by a government compelling any of
      us. Those are GDPR Art. 9 categories, and the harm is outing someone to an
      employer, a family, or a state that criminalises them.
  * Viewer can set a monthly limit of payment (e.g. "I cannot pay more than 50 eur per month")
    * If the limit is reached, payments will be adjusted proportionally
    * Limits can have several levels: besides the total, a Viewer can cap what a single
      Creator receives (e.g. at most 10 eur a month for one Creator within a 50 eur
      total). Exactly how the levels work is not designed yet
  * Viewer chooses which Creators they enable donations for: only events from those
    Creators are collected and count toward the donation


## Tech decisions

  * Data that can be used to track users must be stored encrypted and decrypted with user password when needed
    * URLs can be encrypted with the Viewer password. Plaintext URLs may exist in
      memory while an event is processed (ideally should not), but are never stored in a form that can be
      attributed to a Viewer
    * The list of Creators a Viewer enabled donations for is encrypted the same way
      as events
    * Timestamps of events should be stripped from the high-precision data to 15 min precision (resolution of timezones)
  * Tokens never contain a user id or any other PII in clear text, only metadata
  * Code is fully open source and can be audited by anyone (see [License](#license))


## Development

`nix develop` provides every tool; `just --list --list-submodules` lists the recipes.

  * Native dev stack (TLS, Caddy in front of every `*.crlocal` host, fake creator sites):
    [`dev/README.md`](dev/README.md)
  * The same stack in containers: [`dev/docker/README.md`](dev/docker/README.md)
  * End-to-end scenarios: [`e2e-tests/README.md`](e2e-tests/README.md)

## License

The repository uses three licences, with the full texts in [`LICENSES/`](LICENSES/):

| What | Licence |
|---|---|
| Code and everything not listed below: backend, viewer SPA, creator portal, events iframe, tools, tests, configuration | [AGPL-3.0-or-later](LICENSES/AGPL-3.0-or-later.txt) |
| Documentation: every `*.md` file | [CC-BY-4.0](LICENSES/CC-BY-4.0.txt) |
| What creators put on their own sites: [`embed.js`](backend/templates/events/web/embed.js) | [MIT](LICENSES/MIT.txt) |

Copyright © 2023 The creators-reward contributors.

[`REUSE.toml`](REUSE.toml) holds the same mapping per file in the
[REUSE](https://reuse.software) format, and `reuse lint` checks that every file is covered.

## Use of generative AI

**So far.** The first prototypes (from 2023) were written by hand. Since September 2026
most of the code, tests, and documentation have been written or edited by LLM coding agents (models from Anthropic, OpenAI, Google, Moonshot AI, Zhipu AI and MiniMax 
run through OpenCode harness), directed and reviewed by the maintainer.  [`AGENTS.md`](AGENTS.md) holds the instructions
the agents work from.
This repository starts from a single snapshot; the history before it is not public, so there is no
per-commit record of that work here.

**From now on.** Development continues the same way. Commits containing AI-generated or
AI-assisted work say so in an `Assisted-by:` trailer naming the tool, the model and what
it did.

**Responsibility.** A person decides what is built and how, reviews every change before it
is merged, and is answerable for every line in the repository, whoever or whatever typed it.

