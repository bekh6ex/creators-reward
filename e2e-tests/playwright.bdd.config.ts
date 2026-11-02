import * as fs from 'fs';
import * as path from 'path';
import { defineConfig, devices } from '@playwright/test';
import { defineBddConfig } from 'playwright-bdd';

const testDir = defineBddConfig({
  features: 'features/**/*.feature',
  steps: 'features/steps/**/*.ts',
  tags: process.env.TAGS,
});

/**
 * Resolves the viewer's base URL.
 *
 * `BASE_URL` wins when set (CI, or a manually pointed run). Otherwise this reads the
 * worktree-dev harness state (`dev/.state/current-slot` + `dev/.state/<slot>/urls.json`,
 * see `.agents/skills/worktree-dev/SKILL.md`) so tests always hit the real `app.crlocal`
 * hostname the running stack allocated — never a hardcoded port, never `localhost`.
 */
function resolveBaseURL(): string {
  if (process.env.BASE_URL) return process.env.BASE_URL;

  const devStateDir = path.resolve(__dirname, '../dev/.state');
  try {
    const slot = fs.readFileSync(path.join(devStateDir, 'current-slot'), 'utf8').trim();
    const urls = JSON.parse(fs.readFileSync(path.join(devStateDir, slot, 'urls.json'), 'utf8')) as {
      app: string;
    };
    return urls.app;
  } catch {
    throw new Error(
      'Could not resolve BASE_URL: set the BASE_URL env var, or start the dev stack first ' +
        '(`./dev/start.sh --detach --wait-ready`). See .agents/skills/worktree-dev/SKILL.md.',
    );
  }
}

export default defineConfig({
  testDir,
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [['html', { open: 'never' }], ['list']],
  use: {
    baseURL: resolveBaseURL(),
    // The dev stack's TLS cert is a locally-generated CA (dev/cert/ca.crt); trusting it
    // per-run is simpler than importing it into every headless browser profile.
    ignoreHTTPSErrors: true,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        headless: true,
        launchOptions: {
          chromiumSandbox: false,
          args: [
            '--disable-gpu',
            '--disable-dev-shm-usage',
            '--no-zygote',
            '--disable-setuid-sandbox',
            '--disable-software-rasterizer',
          ],
          executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH || undefined,
        },
      },
    },
  ],
});
