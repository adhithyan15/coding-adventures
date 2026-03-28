import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright configuration for the todo app e2e tests.
 *
 * === Strategy ===
 *
 * Playwright launches the Vite dev server automatically (via webServer),
 * then runs browser tests against it. Each test gets a fresh browser context
 * (isolated cookies, localStorage, IndexedDB), so tests don't interfere.
 *
 * We test in Chromium, Firefox, and WebKit (Safari) to catch cross-browser
 * issues. IndexedDB behavior varies slightly between engines, making this
 * coverage essential for an offline-first app.
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: "html",

  use: {
    baseURL: "http://localhost:5173",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "firefox",
      use: { ...devices["Desktop Firefox"] },
    },
    {
      name: "webkit",
      use: { ...devices["Desktop Safari"] },
    },
  ],

  // Start the Vite dev server before running tests
  webServer: {
    command: "./node_modules/.bin/vite --port 5173",
    url: "http://localhost:5173",
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
});
