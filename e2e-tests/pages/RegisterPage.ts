import { expect, type Page } from '@playwright/test';

export class RegisterPage {
  constructor(private readonly page: Page) {}

  get emailInput() {
    return this.page.getByLabel('Email');
  }

  get passwordInput() {
    return this.page.getByLabel('Password', { exact: true });
  }

  get repeatPasswordInput() {
    return this.page.getByLabel('Repeat password');
  }

  get submitButton() {
    return this.page.getByRole('button', { name: /create account/i });
  }

  get errorAlert() {
    return this.page.locator('.alert-error');
  }

  async goto(): Promise<void> {
    await this.page.goto('/register');
    await expect(this.submitButton).toBeVisible();
  }

  async register(email: string, password: string): Promise<void> {
    await this.emailInput.fill(email);
    await this.passwordInput.fill(password);
    await this.repeatPasswordInput.fill(password);
    await this.submitButton.click();
  }
}
