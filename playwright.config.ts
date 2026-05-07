import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 30000,
  use: {
    baseURL: 'http://localhost:1420',
    browserName: 'chromium',
    launchOptions: { args: ['--no-sandbox'] },
  },
});
