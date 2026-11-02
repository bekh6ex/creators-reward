# VIEWER SPA KNOWLEDGE BASE

## OVERVIEW

Viewer SPA — Preact + Vite + Tailwind 4 / daisyUI 5. Auth is real: `@serenity-kit/opaque` via `authApi.ts` talks to `/api/auth/*`. Enabled donations are real: `enabledDonationsApi.ts` fetches records the events iframe sealed in the browser to the viewer's public key, opens them with `hpke.ts`, groups record ids per site, and `disableDonations` deletes them. The viewer's dashboard and reading history have no backend yet: the Dashboard page renders in-memory dummy data from `src/api.ts`. Vitest + Testing Library cover every component.

## STRUCTURE

```
frontend/viewer/
├── index.html          # single #app div
├── .oxlintrc.json      # oxlint rules (react, jsx-a11y, import, typescript, unicorn)
├── knip.ts             # dead-code config; ignores daisyui/tailwindcss (CSS-only deps)
├── src/
│   ├── index.tsx       # LocationProvider > AuthProvider > Router; all 7 routes wired
│   ├── style.css       # @import "tailwindcss" + @plugin "daisyui" — the only stylesheet
│   ├── routes.ts       # Routes const + creatorInfoRoute(id) helper
│   ├── types.ts        # Money, Creator, CreatorProfile, ViewItem, EnabledDonation
│   ├── api.ts          # dummy data + pure helpers (format/total/monthKey). Mock backend.
│   ├── auth.tsx        # AuthProvider + useAuth(); real session via authApi.ts
│   ├── authApi.ts      # Real HTTP client: OPAQUE register/login/logout/session to /api/auth/*
│   ├── opaqueClient.ts # @serenity-kit/opaque WASM loader + re-export
│   ├── viewerKeys.ts   # X25519 key pair, seal/unseal under the export key, IndexedDB storage
│   ├── hpke.ts         # HPKE (RFC 9180) base-mode receiver on WebCrypto: X25519/HKDF-SHA256/AES-256-GCM
│   ├── enabledDonationsApi.ts # GET/DELETE /api/events/web/enabled-donations: open records, group ids per site, revoke
│   ├── auth.test.tsx   # AuthProvider + authApi integration tests
│   ├── test/
│   │   ├── setup.ts    # jest-dom matchers + afterEach(cleanup)
│   │   ├── render.tsx  # renderWithLocation / renderLoggedOut / renderLoggedIn
│   │   └── hpkeFixture.ts # record sealed by rust-hpke + its key (regenerate: print_browser_fixture)
│   ├── components/     # AuthenticatedHeader, Menu, Link, LoginForm
│   ├── layouts/
│   │   └── AuthenticatedLayout.tsx   # header + <aside> + <main>, redirects when logged out
│   └── pages/
│       ├── Home/           # "/" landing: pitch + LoginForm + register link
│       ├── Login/          # "/login", wraps LoginForm
│       ├── Register/       # "/register", validates + warns password is unrecoverable
│       ├── Dashboard/      # views table, working creator/month filters, delete, totals
│       ├── EnabledDonations/ # sites with donations enabled (decrypted in the browser), since when; Disable deletes every record id of the site
│       ├── CreatorInfo/    # public creator card + CreatorInfoRoute useRoute() adapter
│       └── _404.tsx        # NotFound, wired as the Router `default`
```

Every component has a co-located `*.test.tsx` (`_404` is covered by `pages/NotFound.test.tsx`).

## WHERE TO LOOK

| Task | File | Detail |
|---|---|---|
| Wire a route | `src/index.tsx` | `<Route path={Routes.X} component={Y}/>` |
| Route constants | `src/routes.ts` | `Routes` object or `creatorInfoRoute` helper |
| Dummy data / formatting | `src/api.ts` | swap these for real calls when the backend lands |
| Auth API (real network) | `src/authApi.ts` | OPAQUE protocol wrapper; relative `/api` URLs |
| OPAQUE WASM loader | `src/opaqueClient.ts` | `@serenity-kit/opaque` ready-promise helper |
| Shared types | `src/types.ts` | mirrors the backend's JSON DTOs |
| Session state | `src/auth.tsx` | `useAuth()` throws outside `AuthProvider`; calls `fetchSession()` on mount |
| Add a page | `src/pages/<Name>/index.tsx` | follow `Dashboard`/`EnabledDonations` |
| Authed shell / guard | `src/layouts/AuthenticatedLayout.tsx` | redirects to `/` when logged out |
| Theme / global CSS | `src/style.css` | daisyUI theme config goes in the `@plugin` block |

## CONVENTIONS

- **Navigation**: `useLocation()` for programmatic nav; `useRoute()` for path params. preact-iso intercepts same-origin `<a href>` clicks, so plain anchors navigate client-side and keep state.
- **Styling**: daisyUI component classes + Tailwind utilities in `className`. `src/style.css` is the only stylesheet.
- **daisyUI**: consult the daisyUI 5 docs (https://daisyui.com/llms.txt) before writing markup. Default variants unless a colour is meaningful; `primary` at most once per page; semantic `base-*` colours so themes work.
- **Testing**: query by role/label/text, never by class name. Use `renderLoggedIn` for anything inside `AuthenticatedLayout`, `renderLoggedOut` for auth pages.
- **Imports**: import from `preact`/`preact/hooks` directly; `jsxImportSource: preact`. `verbatimModuleSyntax` is on, so type-only imports need `import type`.
- **Types**: `tsconfig.json` enables every strictness flag the compiler offers, notably `noUncheckedIndexedAccess` (index lookups are `T | undefined`) and `exactOptionalPropertyTypes`.
- **State**: `useState`/`useEffect` only. No signals, redux, or zustand.

## ANTI-PATTERNS

- Don't add CSS Modules or Sass — express styling with daisyUI/Tailwind classes.
- Don't pull in a React-only UI library; it would drag in a `@preact/compat` alias layer and a second renderer instance.
- Don't render a `<button>` without an explicit `type`, or a `.map()` element without a `key` — both fail `pnpm run lint`.
- Don't mutate arrays with `sort`/`reverse`; the linter requires `toSorted`/`toReversed`.
- Don't leave empty placeholder files behind — delete them.
- Don't weaken `tsconfig.json`, `.oxlintrc.json`, or `knip.ts` to make a gate pass.

## COMMANDS

Every command below is also a `just` recipe; run `just --list` in this directory
(or `just frontend::viewer::<recipe>` from the root) to invoke it.

```bash
just install              # pnpm install
just dev                  # Vite dev server
just check                # typecheck + lint + knip + test — run this before reporting done
just typecheck            # tsc --noEmit
just lint                 # oxlint (--fix available as lint-fix)
just knip                 # unused files / exports / dependencies
just test                 # Vitest (single run); test-watch to iterate
just test-coverage        # Vitest + V8 coverage; exits non-zero below 85%
just build                # production build
```

`just check` / `pnpm run check` is the gate. `tsc` catches types, unused code, and unchecked index access; `oxlint` catches what types cannot (missing `key`, missing `button type`, hook dependency mistakes, a11y); `knip` catches dead files and exports; the test step runs under coverage and fails below 85%.

## NOTES

- Stack: Preact 10.29, preact-iso 2.12, Vite 8.2 + @preact/preset-vite, TS 7.0 (target ES2025, moduleResolution bundler, checkJs), Tailwind 4.3 + daisyUI 5.7, Vitest 5 + @testing-library/preact + jsdom 30, oxlint 1.81, knip 6.34.
- `pnpm run check` is green: 99 tests (2026-10-28), coverage 96.49% statements / 89.23% branches / 96.52% functions / 97.39% lines, and `pnpm run build` exits 0.
- **Coverage is enforced at 85%** on all four metrics (`vite.config.ts` → `test.coverage.thresholds`), and `check` runs `test:coverage`, so falling below the bar fails the gate. `coverage.include` covers all of `src/**` deliberately: Vitest otherwise measures only files some test imported, so an untested file is invisible and the percentage rises the *less* you test. Only `src/test/**` (scaffolding) and `src/types.ts` (type-only, nothing executable) are excluded.
- `src/index.tsx` is counted and sits at 0% — it wires all 7 routes, but nothing can import it because `App` and the `render()` call share the file. It is left counted rather than excluded so the gap stays visible; the creator portal split its `App` into `src/app.tsx` precisely so routing could be tested, and doing the same here is the way to close it.
- `tsconfig.json` excludes `coverage` next to `dist`. `checkJs` is on and the HTML reporter emits vendored Istanbul JS (`prettify.js`, `sorter.js`), which fails `tsc --noEmit` with hundreds of errors — on the *second* run only, since the directory does not exist during the first typecheck.
- `src/api.ts` is the seam for the real backend: the Dashboard seeds `useState` from `initialViews`, so edits are local and never leak between tests. The Enabled donations page test mocks `enabledDonationsApi` instead.
- `hpke.ts` exists because WebCrypto has no HPKE and npm HPKE libraries want the raw private key; it only ever calls X25519 `deriveBits` on the non-extractable `CryptoKey`. `hpke.test.ts` opens a record sealed by rust-hpke, and also imports the iframe's sender `../../../backend/templates/events/web/hpke.js` and opens what it seals, so a change on any of the three sides that breaks interop fails the gate. That import is the one place this package reaches outside itself; `checkJs` typechecks the file through it. The site is shown by host only, and there is no per-creator limit column: the enable event carries no limit.
- Dashboard filters run client-side because the creator of a view is inside the encrypted `payload` — the server cannot filter on it. Only `from`/`to` are server-side in the spec.
- `Register` calls the real `register/start`/`register/finish` endpoints; a duplicate email is detected server-side (real `409` from the backend's `CredentialStore`).
- The OPAQUE `exportKey` (not `sessionKey` — they differ) is used once, in `authApi.ts`, to seal (register) or unseal (login) the viewer's X25519 private key (`viewerKeys.ts`: HKDF-SHA256 → AES-256-GCM `wrapKey`). It is never stored. The unsealed private key is kept in IndexedDB (`creators-reward`/`keys`) as a non-extractable `CryptoKey`; `AuthProvider` deletes it on logout and when there is no session, and logs out a session whose key is missing. `viewerKeys.test.ts` runs in the `node` environment with `fake-indexeddb`, because jsdom has no `CryptoKey` X25519 support worth testing against.
- Running the viewer standalone via `pnpm dev` (without the Caddy dev stack) cannot reach the relative `/api` URLs — the backend runs on a different origin/port.
- Accepted gaps: no rate limiting; register's `409` reveals email existence while login is enumeration-resistant; a stale `events_token` cookie after a backend restart names a viewer the new process has no record of, so its enabled-donation records are not found (do not reuse cookies across backend restarts).
- Not surfaced in the UI: view pagination via `cursor`.
- `skipLibCheck` stays on: every error it hides is in third-party `.d.ts` (`@preact/preset-vite` wants `@types/babel__core`; jest-dom and vitest declare `Assertion` with different type parameters).
- knip cannot follow `@plugin`/`@import` inside CSS, so `daisyui` and `tailwindcss` are listed in `ignoreDependencies` — they are genuinely used by `src/style.css`.
- `@testing-library/preact@3.2.4` wants `@testing-library/dom@^8` while jest-dom 7 wants `>=10` — pnpm warns, suite is green.
