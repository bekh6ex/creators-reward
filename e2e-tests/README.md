# Creator's Reward — E2E Tests

Playwright + [playwright-bdd](https://vitalets.github.io/playwright-bdd/) end-to-end tests for the
viewer SPA, the creator portal and billing, written as Gherkin scenarios (see `.agents/skills/gherkin-scenarios/SKILL.md` for the
scenario-writing conventions).

## Setup

```bash
just install
```

The config launches the Chromium at `$PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` when it is set. Only
without it does Playwright need its own browser: `pnpm exec playwright install chromium`.
Run `just --list` in this directory (or `just e2e-tests::<recipe>` from the root) to see all recipes.

## Prerequisites

Tests run against the [worktree-dev stack](../.agents/skills/worktree-dev/SKILL.md):

```bash
nix develop
just dev::start-detach
```

`features/reading-is-billed.feature` additionally needs:

- **A freshly started stack.** It connects `website1.crlocal`, which one creator can claim once per
  backend run, and publishes billing keys for the coming periods, which cannot be replaced. A
  second run against the same backend fails.
- **Short billing periods**, so a period closes during the run. The steps read
  `CR_BILLING_PERIOD` from the slot's `env` and refuse anything but `<n>s` up to `300s`:

  ```bash
  just dev::stop; CR_BILLING_PERIOD=60s just dev::start-detach
  ```

- **`cargo`** (from `nix develop`): the key holders' steps run the real `billing-ceremony` tool
  (`dev/README.md` → Billing). Its shares go to `test-results/billing-ceremony/`.

Its three scenarios run in order (`@mode:serial`) and share state: a creator connects
`website1.crlocal`; a viewer enables donations there and reads an article, and both page views
are recorded; once the period ends, the ceremony charges `0.20 €` (2 page views at the default
10 cents) and pays the site the same. Nothing reports a single viewer's charge yet, so the
charge check is the period's sum, which is this viewer's charge only because a fresh stack has
no other reader. A run takes about 1.5 minutes, most of it waiting for the period
to end.

The config auto-resolves `baseURL` from `dev/.state/current-slot` + `urls.json`. To point at a
different instance instead, set `BASE_URL` explicitly:

```bash
BASE_URL=https://app.crlocal:18090 just test
```

## Running tests

```bash
just test              # generate + run all scenarios (headless chromium)
just test-headed       # headed
just test-debug        # Playwright inspector
just test-ui           # Playwright UI mode
just test-report       # view the last HTML report

./run-e2e-tests.sh     # sanity-checked wrapper (pnpm presence + version) around `pnpm test`
```

## Quality gate

```bash
just qa                # tsc --noEmit + prettier --check + bddgen (feature/step wiring)
just format-fix        # auto-format
```

## Structure

```
e2e-tests/
├── playwright.bdd.config.ts   # BDD config; baseURL from the dev-stack harness or $BASE_URL
├── pages/                     # Page objects (viewer pages, CreatorPortal, NewsSite)
├── support/                   # Dev-stack state (urls.json, env) and the billing ceremony tool
└── features/
    ├── *.feature              # Gherkin scenarios
    └── steps/
        ├── *.steps.ts         # Step definitions
        └── hooks.ts           # Fails a scenario on uncaught JS exceptions / console.error
```
