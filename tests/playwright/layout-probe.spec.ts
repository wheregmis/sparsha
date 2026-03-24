import { expect, test } from "@playwright/test";

const baseURL =
  process.env.SPARSH_LAYOUT_PROBE_URL ?? "http://127.0.0.1:4178";

test("layout probe renders the centered card on the first frame", async ({
  page,
}) => {
  await page.setViewportSize({ width: 960, height: 640 });
  await page.goto(baseURL);

  await expect(page).toHaveTitle(/Sparsha Layout Probe/);
  await expect(page.getByText("Layout Probe", { exact: true })).toBeVisible();
  await expect(
    page.getByText("Teal guide = expected centered card position.", {
      exact: true,
    }),
  ).toBeVisible();
  await expect(page.getByText("Centered Probe", { exact: true })).toBeVisible();
  await expect(page.getByText(/dx \+0 dy \+0 dw \+0 dh \+0/)).toBeVisible();
});
