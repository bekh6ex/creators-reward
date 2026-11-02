import * as path from 'path';
import { expect, type Response } from '@playwright/test';
import { createBdd } from 'playwright-bdd';
import { DashboardPage, RegisterPage } from '../../pages';
import { CreatorPortal } from '../../pages/CreatorPortal';
import { NewsSite } from '../../pages/NewsSite';
import { BillingCeremony } from '../../support/billingCeremony';
import { devStack, urlOf } from '../../support/devStack';

const { Given, When, Then } = createBdd();

// The feature runs serially (@mode:serial) in one worker, so each scenario picks up where the
// previous one left off through this module's state.
const stamp = Date.now();
let ceremony: BillingCeremony | undefined;
let pageViews: Response[] = [];
let readingPeriod: number | undefined;
let totals: Awaited<ReturnType<BillingCeremony['run']>> | undefined;

const PASSWORD = 'TestPassword123!';

function cents(euros: string): number {
  const match = /^(\d+)\.(\d{2}) €$/.exec(euros);
  if (!match) throw new Error(`Not an amount like "0.20 €": ${euros}`);
  return Number(match[1]) * 100 + Number(match[2]);
}

function requireCeremony(): BillingCeremony {
  if (!ceremony) throw new Error('No billing keys were published in this run');
  return ceremony;
}

function requireReadingPeriod(): number {
  if (readingPeriod === undefined) throw new Error('No page views were recorded in this run');
  return readingPeriod;
}

Given('a creator has registered in the creator portal', async ({ page }) => {
  await new CreatorPortal(page, devStack().urls.creator).register(
    `creator-${stamp}@example.org`,
    PASSWORD,
  );
});

When('the creator connects the website {string}', async ({ page }, domain: string) => {
  await new CreatorPortal(page, devStack().urls.creator).addWebsite(domain);
});

Then(
  "the creator's websites list {string} as {string}",
  async ({ page }, domain: string, status: string) => {
    const portal = new CreatorPortal(page, devStack().urls.creator);
    expect(await portal.websiteStatus(domain)).toBe(status);
  },
);

Given('the key holders have published keys for the coming billing periods', async () => {
  // Shares are dev-only keys; test-results/ is gitignored and wiped at the start of a run.
  ceremony = new BillingCeremony(
    devStack(),
    path.resolve(__dirname, '../../test-results/billing-ceremony'),
  );
  await ceremony.publishKeys(Math.ceil(600 / ceremony.periodSeconds) + 1);
});

Given('a viewer has registered', async ({ page }) => {
  const registerPage = new RegisterPage(page);
  await registerPage.goto();
  await registerPage.register(`reader-${stamp}@example.org`, PASSWORD);
  await new DashboardPage(page).expectVisible();
});

When('the viewer enables donations on {string}', async ({ page }, host: string) => {
  // Both page views must land in one period for the charge to be predictable, so a period
  // about to end is waited out.
  const billing = requireCeremony();
  const remainingMs = billing.periodEndMs(billing.currentPeriod()) - Date.now();
  const neededMs = Math.max(20_000, (billing.periodSeconds * 1000) / 2);
  if (remainingMs < neededMs) await page.waitForTimeout(remainingMs + 500);

  const site = new NewsSite(page, urlOf(devStack(), host));
  await site.openHomePage();
  pageViews = [await site.enableDonations()];
});

When('the viewer reads an article on {string}', async ({ page }, host: string) => {
  pageViews.push(await new NewsSite(page, urlOf(devStack(), host)).readFirstArticle());
});

Then(
  '{int} page views are recorded for billing in the same billing period',
  async ({}, count: number) => {
    expect(pageViews.map((response) => response.status())).toEqual(Array(count).fill(202));
    const periods = new Set(
      pageViews.map((response) => (response.request().postDataJSON() as { period: number }).period),
    );
    expect(periods.size).toBe(1);
    readingPeriod = [...periods][0];
  },
);

Given("the billing period of the viewer's page views has ended", async () => {
  await requireCeremony().waitForBatch(requireReadingPeriod());
});

When('the key holders run the billing ceremony for that period', async () => {
  totals = await requireCeremony().run(requireReadingPeriod());
});

// Nothing shows a single viewer's charge yet: the settlement only sums the
// period's charges. On a fresh stack this viewer is the period's only reader, so the sum is
// their charge.
Then('the viewer is charged {string}', async ({}, amount: string) => {
  const settlement = await requireCeremony().settlement(requireReadingPeriod());
  expect(settlement.chargedCents).toBe(cents(amount));
});

Then('{string} is paid {string}', async ({}, host: string, amount: string) => {
  // Billing pays the site origin, not its creator yet.
  expect(totals?.perSiteMillicents[urlOf(devStack(), host)]).toBe(cents(amount) * 1000);
  const settlement = await requireCeremony().settlement(requireReadingPeriod());
  expect(settlement.paidOutCents).toBe(cents(amount));
});
