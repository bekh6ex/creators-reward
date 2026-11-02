# Creator Portal

The web app for people who **publish**. Website owners register here, prove they own their sites, watch what those sites earn, and tell us where to send the money.

The companion app in [`../viewer`](../viewer) is the other half: it belongs to people who *read* and pay.

## What it does

1. **Register, log in, recover a password** — ordinary email + password. Unlike the viewer app, a forgotten password here is recoverable (see [Why password recovery works here](#why-password-recovery-works-here)).
2. **Add a website** — enter a domain and get a `<meta>` tag to paste into your home page. We fetch the page and look for it, the same idea as Google Search Console or Analytics.
3. **See your websites** — every site with its verification state, when it was added, and what it has earned.
4. **Add an EU bank account** — IBAN and BIC, validated before we accept them, plus a history of what has been paid and what is still pending. Payouts start with the EU; other regions are part of the target audience and come later.
5. **See statistics per website** — reads, readers and earnings, broken down by month and by page.
6. **Describe yourself** — the name and description readers see before deciding to support you.

## Running it

```bash
just install
just dev              # http://localhost:5174
```

The viewer app uses the default port 5173, so this one is pinned to 5174 and both can run side by side.
Run `just --list` in this directory (or `just frontend::creator::<recipe>` from the root) to see all recipes.

Other commands:

```bash
just check            # typecheck + lint + dead-code + tests — the gate
just test             # tests only
just build            # production build into dist/
```

## What talks to the backend

Accounts and websites are real: registering, logging in and out, and the website list and "add a website" call the creator service (`/api/*` on this app's host, served by `backend/services/creator`) through [`src/creatorApi.ts`](src/creatorApi.ts). Run the whole stack (`just dev::start-detach`, see `dev/README.md`) and open `https://creator.crlocal:<port>`; plain `just dev` has no backend behind it.

Everything else — payouts, the bank account, statistics, the profile, password recovery — is still dummy data in [`src/api.ts`](src/api.ts).

Consequences worth knowing while clicking around:

- **The backend keeps creator accounts in memory**, with passwords in plain text for now. A backend restart forgets every account and website.
- **A website is connected as soon as you add it.** Ownership is not checked yet, so the first account to add a domain gets it. You still get the meta tag the check will look for later.
- **A reload keeps you logged in**, but resets the bank account and profile you entered.
- **No email is sent.** The password-reset page shows you the link it would have emailed so the flow can be walked end to end.
- **The statistics are invented**, but their shape is not: reader counts are totals, never lists.

## Why password recovery works here

The viewer app does not offer a password reset yet: there, the password derives the key that encrypts a reader's history, so a plain reset would make that history permanently unreadable. Whether a recovery that keeps the history readable is possible without weakening privacy is still being researched. That constraint does not apply to a creator account — nothing here is encrypted with the creator's password — so this portal offers the ordinary "email me a reset link" flow.

## Privacy, from this side of the fence

The project's core promise is that what a reader reads cannot be traced back to them. That shapes what this portal is allowed to show you:

- Statistics are **aggregates**. `Readers` is a count. There is no list of who read anything, and no way to ask for one.
- Per-view detail lives encrypted under each reader's own key, so the server itself cannot break it down further.

If you are adding a feature here, that is the line to stay behind.

## Stack

Preact + preact-iso, Vite, Tailwind 4 with daisyUI 5, TypeScript in full strict mode, Vitest with Testing Library, oxlint and knip. Identical to the viewer app.

It is Preact, not React — import from `preact` directly, and avoid React-only libraries, which would drag in a compatibility layer.

See [AGENTS.md](AGENTS.md) for conventions, gotchas and the layout of the code.
