import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  workers: 2,
  reporter: "list",
  use: {
    baseURL: "http://127.0.0.1:1500",
    browserName: "chromium",
    channel: process.env.PLAYWRIGHT_WEBKIT ? undefined : "chrome",
    viewport: { width: 390, height: 844 },
    colorScheme: "light",
    reducedMotion: "reduce",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  webServer: {
    command: "npx vite --host 127.0.0.1 --port 1500 --strictPort",
    url: "http://127.0.0.1:1500",
    reuseExistingServer: true,
  },
});