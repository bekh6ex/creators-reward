import { execFile } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import { promisify } from 'util';
import { repoRoot, type DevStack } from './devStack';

const run = promisify(execFile);

// Hardcoded in the backend's admin gateway (dev only).
const ADMIN_TOKEN = 'dev-admin-token';

interface Totals {
  period: number;
  perSiteMillicents: Record<string, number>;
  records: number;
  dropped: number;
}

interface SettledLogEntry {
  event: 'settled';
  period: number;
  chargedCents: number;
  paidOutCents: number;
}

/**
 * The key holders' side of billing, driven through the real `billing-ceremony` tool the way
 * `dev/README.md` → Billing describes it: keys and shares are generated offline, only public
 * keys, batches and totals cross the admin gateway.
 */
export class BillingCeremony {
  private readonly adminUrl: string;
  private readonly keysDir: string;
  private readonly sharesDir: string;

  /** Seconds per billing period; the stack must use fixed-length periods. */
  readonly periodSeconds: number;

  constructor(
    stack: DevStack,
    private readonly workDir: string,
  ) {
    const match = /^(\d+)s$/.exec(stack.env.CR_BILLING_PERIOD ?? '');
    const seconds = match ? Number(match[1]) : NaN;
    if (!(seconds > 0 && seconds <= 300)) {
      throw new Error(
        `The billing scenarios need short billing periods, the stack runs with ` +
          `CR_BILLING_PERIOD=${stack.env.CR_BILLING_PERIOD}. Restart it with ` +
          '`CR_BILLING_PERIOD=60s ./dev/start.sh --detach --wait-ready`.',
      );
    }
    this.periodSeconds = seconds;
    this.adminUrl = `http://127.0.0.1:${stack.env.CR_ADMIN_PORT}`;
    this.keysDir = path.join(workDir, 'keys');
    this.sharesDir = path.join(workDir, 'shares');
  }

  currentPeriod(): number {
    return Math.floor(Date.now() / 1000 / this.periodSeconds);
  }

  periodEndMs(period: number): number {
    return (period + 1) * this.periodSeconds * 1000;
  }

  /** Generates keys for the next `count` periods and uploads their public halves. */
  async publishKeys(count: number): Promise<void> {
    fs.rmSync(this.workDir, { recursive: true, force: true });
    await this.tool(
      'keygen',
      ...['--keys-out', this.keysDir, '--shares-out', this.sharesDir],
      ...['--period-length', `${this.periodSeconds}s`, '--period', 'current'],
      ...['--count', String(count)],
    );
    const keys = fs.readdirSync(this.keysDir).map((file) => path.join(this.keysDir, file));
    await this.tool(
      'upload-keys',
      '--api',
      this.adminUrl,
      ...keys.flatMap((key) => ['--key', key]),
    );
  }

  /** Waits until the backend has closed `period` and its batch awaits the ceremony. */
  async waitForBatch(period: number): Promise<void> {
    const waitMs = this.periodEndMs(period) - Date.now();
    if (waitMs > 0) await new Promise((resolve) => setTimeout(resolve, waitMs));
    for (let attempt = 0; attempt < 30; attempt++) {
      const response = await this.admin(`/periods/${period}/batch`);
      if (response.ok) return;
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
    throw new Error(`Period ${period} was not closed within 30 s of its end`);
  }

  /** Fetches the batch, opens it with two of the three shares and uploads the totals. */
  async run(period: number): Promise<Totals> {
    const batch = path.join(this.workDir, `batch-${period}.json`);
    const totals = path.join(this.workDir, `totals-${period}.json`);
    await this.tool(
      'fetch-batch',
      '--api',
      this.adminUrl,
      '--period',
      String(period),
      '--out',
      batch,
    );
    await this.tool(
      'tally',
      ...['--batch', batch, '--out', totals],
      ...['--share', path.join(this.sharesDir, `${period}-holder-1.json`)],
      ...['--share', path.join(this.sharesDir, `${period}-holder-2.json`)],
    );
    await this.tool('upload-totals', '--api', this.adminUrl, '--totals', totals);
    return JSON.parse(fs.readFileSync(totals, 'utf8')) as Totals;
  }

  /** The settlement the admin log recorded for `period`. */
  async settlement(period: number): Promise<SettledLogEntry> {
    const response = await this.admin('/log');
    const log = (await response.json()) as Array<{ event: string; period: number }>;
    const settled = log.find((entry) => entry.event === 'settled' && entry.period === period);
    if (!settled) throw new Error(`Period ${period} has not been settled`);
    return settled as SettledLogEntry;
  }

  private admin(route: string): Promise<Response> {
    return fetch(`${this.adminUrl}/admin/billing${route}`, {
      headers: { Authorization: `Bearer ${ADMIN_TOKEN}` },
    });
  }

  private async tool(...args: string[]): Promise<void> {
    await run('cargo', ['run', '-q', '-p', 'billing_ceremony', '--', ...args], {
      cwd: path.join(repoRoot, 'backend'),
      env: { ...process.env, CR_ADMIN_TOKEN: ADMIN_TOKEN },
    });
  }
}
