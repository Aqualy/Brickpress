import { defineConfig } from '@playwright/test';
const baseURL = process.env.PRESS_TEST_URL ?? 'http://127.0.0.1:5173';
export default defineConfig({
  testDir: './tests',
  fullyParallel: false,
  workers: 1,
  timeout: 90_000,
  use: {
    baseURL,
    viewport: { width: 1440, height: 960 },
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure'
  },
  webServer: process.env.PRESS_TEST_URL
    ? undefined
    : { command: 'npm run dev -- --port 5173', url: baseURL, reuseExistingServer: !process.env.CI },
  reporter: 'list'
});
