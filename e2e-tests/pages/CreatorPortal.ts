import { expect, type Page } from '@playwright/test';

/** The creator portal (`frontend/creator`), on its own host. */
export class CreatorPortal {
  constructor(
    private readonly page: Page,
    private readonly baseUrl: string,
  ) {}

  async register(email: string, password: string): Promise<void> {
    await this.page.goto(`${this.baseUrl}/register`);
    await this.page.getByLabel('Email').fill(email);
    await this.page.getByLabel('Password', { exact: true }).fill(password);
    await this.page.getByLabel('Repeat password').fill(password);
    await this.page.getByRole('button', { name: /create account/i }).click();
    await expect(this.page.getByRole('heading', { name: 'Your websites' })).toBeVisible();
  }

  async addWebsite(domain: string): Promise<void> {
    await this.page.goto(`${this.baseUrl}/websites/new`);
    await this.page.getByLabel('Domain').fill(domain);
    await this.page.getByRole('button', { name: 'Add website' }).click();
    await expect(this.page.getByRole('heading', { name: `${domain} is connected` })).toBeVisible();
  }

  /** The status badge of `domain` in the list of the creator's websites. */
  async websiteStatus(domain: string): Promise<string> {
    await this.page.goto(`${this.baseUrl}/websites`);
    const row = this.page.getByRole('row').filter({ hasText: domain });
    await expect(row).toHaveCount(1);
    return (await row.getByRole('cell').nth(2).innerText()).trim();
  }
}
