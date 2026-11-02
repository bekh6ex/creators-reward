import { expect, type Page } from '@playwright/test';

export class DashboardPage {
  constructor(private readonly page: Page) {}

  get logoutButton() {
    return this.page.getByRole('button', { name: 'Logout' });
  }

  async expectVisible(): Promise<void> {
    await expect(this.page).toHaveURL(/\/dashboard$/);
    await expect(this.logoutButton).toBeVisible();
  }

  async logout(): Promise<void> {
    await this.logoutButton.click();
    await expect(this.page).toHaveURL(/\/$/);
    await expect(this.logoutButton).not.toBeVisible();
  }
}
