import { createBdd } from 'playwright-bdd';
import { RegisterPage, LoginPage, DashboardPage } from '../../pages';

const { Given, When, Then } = createBdd();

interface ViewerCredentials {
  email: string;
  password: string;
}

// The backend's viewer store is in-memory and never reset between scenario runs, so
// each run needs its own email — a fixed one would 409 on the second run.
function newViewerCredentials(): ViewerCredentials {
  const stamp = Date.now();
  return {
    email: `viewer-${stamp}@example.localhost`,
    password: 'TestPassword123!',
  };
}

let credentials: ViewerCredentials;

Given('the visitor is on the registration page', async ({ page }) => {
  const registerPage = new RegisterPage(page);
  await registerPage.goto();
});

When('the visitor registers a new account', async ({ page }) => {
  credentials = newViewerCredentials();
  const registerPage = new RegisterPage(page);
  await registerPage.register(credentials.email, credentials.password);
  const dashboard = new DashboardPage(page);
  await dashboard.expectVisible();
});

When('the visitor logs out', async ({ page }) => {
  const dashboard = new DashboardPage(page);
  await dashboard.logout();
});

When('the visitor logs in with the same credentials', async ({ page }) => {
  const loginPage = new LoginPage(page);
  await loginPage.goto();
  await loginPage.login(credentials.email, credentials.password);
});

Then('the viewer dashboard is shown', async ({ page }) => {
  const dashboard = new DashboardPage(page);
  await dashboard.expectVisible();
});
