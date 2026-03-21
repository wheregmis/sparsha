import { expect, test } from "@playwright/test";

const baseURL =
  process.env.SPARSH_HYBRID_OVERLAY_URL ?? "http://127.0.0.1:4174";

test("hybrid overlay keeps DOM overlays interactive around the GPU surface", async ({
  page,
}) => {
  await page.goto(baseURL);
  await expect(page).toHaveTitle(/Hybrid Overlay - Sparsh/);

  const canvas = page.locator("canvas").first();
  await expect(canvas).toBeVisible();
  await expect(page.getByText("HYBRID OVERLAY")).toBeVisible();
  await expect(page.getByText(/Accent 1/)).toBeVisible();

  await page.mouse.click(720, 420);
  await expect(page.getByText(/Accent 2/)).toBeVisible();

  await page.setViewportSize({ width: 1180, height: 760 });
  await expect(canvas).toBeVisible();
  await expect(page.getByText("DOM + GPU")).toBeVisible();
});
