import { defineConfig } from '@playwright/test';

const chromiumExecutable = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH;

export default defineConfig({
  testDir: './tests',
  timeout: 30000,
  use: {
    baseURL: 'http://localhost:1420',
    browserName: 'chromium',
    launchOptions: {
      args: ['--no-sandbox'],
      ...(chromiumExecutable ? { executablePath: chromiumExecutable } : {}),
    },
  },
});
