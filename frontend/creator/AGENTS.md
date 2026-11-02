# CREATOR PORTAL KNOWLEDGE BASE

## OVERVIEW

Creator portal SPA — Preact + Vite + Tailwind 4 / daisyUI 5, the same stack as `frontend/viewer`. Where the viewer app is for people who *read*, this one is for people who *publish*: registering, proving they own a website, watching what it earns, and getting paid.

**Accounts and websites are real; the rest is faked.** Register, login, logout, session restore, and the website list/add call the creator service (`/api/*` on the portal's own host, source `backend/services/creator`) through `src/creatorApi.ts`, the only module that calls `fetch`. Payouts, the bank account, statistics, the profile and password recovery are still dummy data in `src/api.ts`.

## STRUCTURE

```
frontend/creator/
├── index.html          # single #app div
├── .oxlintrc.json      # oxlint rules (react, jsx-a11y, import, typescript, unicorn)
├── knip.ts             # dead-code config; ignores daisyui/tailwindcss (CSS-only deps)
├── src/
│   ├── index.tsx       # render(<App/>) only — kept tiny so app.tsx stays importable in tests
│   ├── app.tsx         # LocationProvider > AuthProvider > PortalProvider > Router; all 11 routes
│   ├── style.css       # @import "tailwindcss" + @plugin "daisyui" — the only stylesheet
│   ├── routes.ts       # Routes const + websiteRoute(id) / resetPasswordRoute(token)
│   ├── types.ts        # Money, Website, WebsiteStats, BankAccount, Payout, CreatorProfile
│   ├── creatorApi.ts   # the creator API client (fetch); CreatorApiError carries the HTTP status
│   ├── api.ts          # dummy data (payouts, stats, profile) + pure helpers
│   ├── auth.tsx        # AuthProvider + useAuth(); restores the session from the cookie (isLoading)
│   ├── store.tsx       # PortalProvider + usePortal(); websites (from the API)/account/profile shared across pages
│   ├── test/
│   │   ├── setup.ts    # jest-dom matchers + afterEach(cleanup)
│   │   └── render.tsx  # renderLoggedOut / renderLoggedIn(ui, {websites, bankAccount})
│   ├── components/     # Link, Menu, AuthenticatedHeader, LoginForm, VerificationBadge, CopyableCode
│   ├── layouts/
│   │   └── AuthenticatedLayout.tsx   # header + <aside> + <main>, redirects when logged out
│   └── pages/
│       ├── Home/           # "/" pitch + login form
│       ├── Login/          # "/login"
│       ├── Register/       # "/register"
│       ├── ForgotPassword/ # "/forgot-password" — asks for email, then reveals the demo reset link
│       ├── ResetPassword/  # "/reset-password/:token" + ResetPasswordRoute useRoute() adapter
│       ├── Websites/       # "/websites" list: status badge, added date, earnings
│       ├── AddWebsite/     # "/websites/new" two-step: form, then the meta tag to install
│       ├── WebsiteDetail/  # "/websites/:id" verification card + statistics
│       ├── Payouts/        # "/payouts" EU bank account + payout history
│       ├── Profile/        # "/profile" public name/description + private contact
│       └── _404.tsx        # NotFound, wired as the Router `default`
```

Every component has a co-located `*.test.tsx` (`_404` is covered by `pages/NotFound.test.tsx`), plus `src/api.test.ts` for the pure helpers and `src/app.test.tsx` for routing.

## WHERE TO LOOK

| Task | File | Detail |
|---|---|---|
| Wire a route | `src/app.tsx` | `<Route path={Routes.X} component={Y}/>` |
| Route constants | `src/routes.ts` | `Routes` object, `websiteRoute`, `resetPasswordRoute` |
| Creator API calls | `src/creatorApi.ts` | `/api/*`; throws `CreatorApiError` with the status |
| Remaining dummy data | `src/api.ts` | payouts, statistics, profile — swap for real calls |
| Shared portal state | `src/store.tsx` | `usePortal()` — websites, bank account, profile |
| Session state | `src/auth.tsx` | `useAuth()` throws outside `AuthProvider` |
| IBAN / BIC / domain rules | `src/api.ts` | `validateIban`, `validateBic`, `validateDomain` |
| Verification tag | `src/api.ts` | `metaTag(token)`; the token comes from the API |
| Test fixtures / fetch stubs | `src/test/fixtures.ts` | `demoWebsites`, `respondWith` (in order), `routeFetch` (by path) |
| Add a page | `src/pages/<Name>/index.tsx` | follow `Websites`/`Payouts` |
| Authed shell / guard | `src/layouts/AuthenticatedLayout.tsx` | redirects to `/` when logged out |
| Theme / global CSS | `src/style.css` | daisyUI theme config goes in the `@plugin` block |

## CONVENTIONS

- **Navigation**: `useLocation()` for programmatic nav; `useRoute()` for path params. preact-iso intercepts same-origin `<a href>` clicks, so plain anchors navigate client-side and keep state.
- **Route order matters.** preact-iso's `Router` keeps the **first** matching child. `Routes.AddWebsite` (`/websites/new`) is declared before `Routes.Website` (`/websites/:id`) or the detail page would render for a site whose id is literally `new`. `src/app.test.tsx` locks this.
- **Every `<form>` sets `noValidate`.** The browser's own constraint validation blocks submit on a `type="email"`/`type="url"` mismatch and shows a native tooltip, so the app's styled `role="alert"` messages would never render and `onSubmit` would never run. Validation is ours; input types stay for mobile keyboards and semantics.
- **Styling**: daisyUI component classes + Tailwind utilities in `className`. `src/style.css` is the only stylesheet.
- **daisyUI**: consult the daisyUI 5 docs (https://daisyui.com/llms.txt) before writing markup. Default variants unless a colour is meaningful; `primary` at most once per page; semantic `base-*` colours so themes work.
- **Testing**: query by role/label/text, never by class name. `renderLoggedIn` for anything inside `AuthenticatedLayout`, `renderLoggedOut` for auth pages. Seed fixtures with `renderLoggedIn(ui, {websites, bankAccount})`.
- **Imports**: import from `preact`/`preact/hooks` directly; `jsxImportSource: preact`. `verbatimModuleSyntax` is on, so type-only imports need `import type`.
- **Types**: every strictness flag the compiler offers, notably `noUncheckedIndexedAccess` and `exactOptionalPropertyTypes`.
- **State**: `useState`/`useEffect`/`useMemo` only. No signals, redux, or zustand.

## ANTI-PATTERNS

- Don't remove `noValidate` from a form — it silently disables every validation message on that page.
- Don't declare a literal path after the `:param` route that would swallow it.
- Don't add CSS Modules or Sass — express styling with daisyUI/Tailwind classes.
- Don't pull in a React-only UI library; it would drag in a `@preact/compat` alias layer and a second renderer instance.
- Don't render a `<button>` without an explicit `type`, or a `.map()` element without a `key` — both fail `pnpm run lint`.
- Don't mutate arrays with `sort`/`reverse`; the linter requires `toSorted`/`toReversed`.
- Don't use an array + `.includes()` for a membership set; oxlint requires `Set` + `.has()`.
- Don't weaken `tsconfig.json`, `.oxlintrc.json`, or `knip.ts` to make a gate pass.
- Don't put per-page data in page-level `useState` if another page must see it — that is what `store.tsx` is for.

## COMMANDS

Every command below is also a `just` recipe; run `just --list` in this directory
(or `just frontend::creator::<recipe>` from the root) to invoke it.

```bash
just install              # pnpm install
just dev                  # Vite dev server on :5174 (viewer takes the default 5173)
just check                # typecheck + lint + knip + test — run this before reporting done
just typecheck            # tsc --noEmit
just lint                 # oxlint (--fix available as lint-fix)
just knip                 # unused files / exports / dependencies
just test                 # Vitest (single run); test-watch to iterate
just test-coverage        # Vitest + V8 coverage; exits non-zero below 85%
just build                # production build
```

`just check` / `pnpm run check` is the gate. `tsc` catches types and unused code; `oxlint` catches what types cannot (missing `key`, missing `button type`, hook dependency mistakes, a11y); `knip` catches dead files and exports; the test step runs under coverage and fails below 85%.

## NOTES

- Stack matches the viewer exactly: Preact 10.29, preact-iso 2.12, Vite 8.2 + @preact/preset-vite, TS 7.0 (target ES2025, moduleResolution bundler, checkJs), Tailwind 4.3 + daisyUI 5.7, Vitest 5 + @testing-library/preact + jsdom 30, oxlint 1.81, knip 6.34.
- `pnpm run check` is green: 141 tests across 22 files, coverage 97.83% statements / 95.85% branches / 96.46% functions / 98% lines.
- **Coverage is enforced at 85%** on all four metrics (`vite.config.ts` → `test.coverage.thresholds`), and `check` runs `test:coverage`, so falling below the bar fails the gate. `coverage.include` covers all of `src/**` deliberately: Vitest otherwise measures only files some test imported, so an untested file is invisible and the percentage rises the *less* you test. Only `src/test/**` (scaffolding) and `src/types.ts` (type-only, nothing executable) are excluded.
- `src/index.tsx` is counted and sits at 0%; it is the five-line `render(<App/>)` bootstrap, and `App` itself lives in `app.tsx` at 100%. Nothing is hidden by excluding it, because it is not excluded.
- `tsconfig.json` excludes `coverage` next to `dist`. `checkJs` is on and the HTML reporter emits vendored Istanbul JS (`prettify.js`, `sorter.js`), which fails `tsc --noEmit` with hundreds of errors — on the *second* run only, since the directory does not exist during the first typecheck.
- **No ownership check yet.** The backend connects a website as soon as it is added (`status: "verified"`, `lastCheckedAt: null`), so the first account to claim a domain gets it. It already mints a random per-site token, and the portal shows the meta tag so it can be installed ahead of the check. The `pending`/`failed` states and badges stay for that check.
- **Tests have no backend.** `src/test/setup.ts` stubs `fetch` to answer 401 for every request, which reads as "not logged in"; a test that needs the API stubs it again with `routeFetch`/`respondWith`. `renderLoggedIn` seeds `demoWebsites`, so it never fetches. Logged-out renders check the session first, so their redirects are async — use `waitFor`/`findBy*`.
- Statistics are keyed by website id and only the fixture id `1` has numbers; websites from the API (UUIDs) show empty statistics.
- **Password recovery exists here but not (yet) in the viewer.** The viewer's password derives its history encryption key, so a plain reset would destroy the data; a recovery that keeps it readable is open research. Nothing in this portal is encrypted with the creator's password, hence the ordinary email-reset flow. `Register.test.tsx` asserts this portal does *not* show the viewer's "we cannot reset it" warning.
- `ForgotPassword` reveals the reset link on screen (`/reset-password/demo-reset-token`) because no mail is sent. `ResetPassword` rejects the token `expired` so the failure path stays reachable.
- IBAN validation is real: ISO 7064 MOD-97-10 check digits, plus a per-country length table covering the 27 EU member states. A valid non-EU IBAN (e.g. `GB82WEST12345698765432`) is rejected deliberately — payouts are EU-only for now (EU first; other regions are target audience too, but later).
- Statistics are aggregates by design: `viewers` is a count, never a list, because per-view detail is encrypted with the viewer's key. Keep it that way.
- `initialBankAccount` is `null` so the "add an account" flow is the default experience; `initialPayouts` still has history so the table is not empty.
- The session survives a reload (`GET /api/auth/session`, cookie `cr_creator_session`); websites reload from the API. The bank account and profile are still in memory and reset on reload. The backend keeps creator data in memory too, so a backend restart logs everyone out.
- Duplicated from `frontend/viewer` rather than shared: `tsconfig.json`, `.oxlintrc.json`, `knip.ts`, `style.css`, `Link`, `Menu`, `AuthenticatedHeader`, `AuthenticatedLayout`, `test/setup.ts`. There is no pnpm workspace; extracting a shared package is a deliberate follow-up, not an oversight.
- `skipLibCheck` stays on: every error it hides is in third-party `.d.ts` (`@preact/preset-vite` wants `@types/babel__core`; jest-dom and vitest declare `Assertion` with different type parameters).
- knip cannot follow `@plugin`/`@import` inside CSS, so `daisyui` and `tailwindcss` are in `ignoreDependencies` — they are genuinely used by `src/style.css`.
- `@testing-library/preact@3.2.4` wants `@testing-library/dom@^8` while jest-dom 7 wants `>=10` — pnpm warns, suite is green.
