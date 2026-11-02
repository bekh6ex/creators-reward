import { expect, type Page, type Response } from '@playwright/test';

/** A fake news site (`fake-sites/news`) with the Creators Reward iframe embedded. */
export class NewsSite {
  constructor(
    private readonly page: Page,
    private readonly baseUrl: string,
  ) {}

  private get iframe() {
    return this.page.frameLocator('iframe[title="Creators Reward"]');
  }

  /**
   * Resolves with the response to the next page view the iframe counts for billing. Start
   * waiting before the navigation or click that triggers it.
   */
  private nextCountedPageView(): Promise<Response> {
    return this.page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        new URL(response.url()).pathname === '/api/events/web/usage',
    );
  }

  async openHomePage(): Promise<void> {
    await this.page.goto(this.baseUrl);
  }

  /** Clicks the iframe's own button; returns the page view counted once donations are on. */
  async enableDonations(): Promise<Response> {
    const button = this.iframe.getByRole('button', { name: 'Enable donations' });
    await expect(button).toBeEnabled();
    const counted = this.nextCountedPageView();
    await button.click();
    await expect(this.iframe.getByText('donations enabled for this site')).toBeVisible();
    return counted;
  }

  /** Opens the first headline on the current page; returns the page view it counted. */
  async readFirstArticle(): Promise<Response> {
    const headline = this.page.locator('article h2 a').first();
    const title = (await headline.innerText()).trim();
    const counted = this.nextCountedPageView();
    await headline.click();
    await expect(this.page.getByRole('heading', { level: 1, name: title })).toBeVisible();
    return counted;
  }
}
