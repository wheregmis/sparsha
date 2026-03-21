import { defineConfig } from "@playwright/test";

const use = {
  browserName: "chromium",
  headless: true,
  trace: "retain-on-failure",
  screenshot: "only-on-failure",
  video: "retain-on-failure",
  viewport: {
    width: 1440,
    height: 1024,
  },
};

if (process.env.CHROME_PATH) {
  use.launchOptions = {
    executablePath: process.env.CHROME_PATH,
  };
}

export default defineConfig({
  testDir: "./tests/playwright",
  timeout: 30_000,
  expect: {
    timeout: 10_000,
  },
  fullyParallel: false,
  reporter: [["list"]],
  use,
});
