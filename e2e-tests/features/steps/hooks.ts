import type { ConsoleMessage } from '@playwright/test';
import { createBdd } from 'playwright-bdd';

const { Before, After } = createBdd();

/**
 * The viewer's `AuthProvider` unconditionally probes `GET /api/auth/session` on mount
 * to detect whether the visitor already has one (see `src/auth.tsx`). A guest lands on
 * `/register` or `/login` with no session and legitimately gets a 401 back — Chromium
 * still logs that as an error-type "Failed to load resource" console line even though
 * nothing is broken, so it must not fail a scenario that never touched authenticated
 * state yet.
 */
function isGuestSessionProbeFailure(msg: ConsoleMessage): boolean {
  if (!msg.text().startsWith('Failed to load resource:')) return false;
  try {
    return new URL(msg.location().url).pathname === '/api/auth/session';
  } catch {
    return false;
  }
}

Before(async ({ page }) => {
  // Collect client-side JS failures and assert in the After hook, because throwing
  // from inside an async event listener does not reliably fail the running scenario.
  const jsErrors: string[] = [];

  // Uncaught JS exceptions / unhandled promise rejections surfaced to the page.
  page.on('pageerror', (error) => {
    jsErrors.push(
      `Uncaught JS exception on ${page.url()}:\n${error.stack ?? `${error.name}: ${error.message}`}`,
    );
  });

  // console.error() calls from page scripts (excluding the expected guest-session probe).
  page.on('console', (msg) => {
    if (msg.type() !== 'error') return;
    if (isGuestSessionProbeFailure(msg)) return;
    const location = msg.location();
    jsErrors.push(
      `console.error on ${page.url()}:\n${msg.text()}\n  at ${location.url}:${location.lineNumber}:${location.columnNumber}`,
    );
  });

  // Expose the collected errors to the After hook via the page object.
  (page as unknown as { __jsErrors: string[] }).__jsErrors = jsErrors;
});

After(async ({ page }) => {
  const jsErrors = (page as unknown as { __jsErrors?: string[] }).__jsErrors ?? [];
  if (jsErrors.length > 0) {
    throw new Error(
      `${jsErrors.length} JavaScript error(s) detected during the scenario:\n\n` +
        jsErrors.join('\n\n'),
    );
  }
});
