import { expect, test } from "@playwright/test";

const baseURL = process.env.SPARSH_COUNTER_URL ?? "http://127.0.0.1:4177";

test("counter starter paints the Material-style shell and increments", async ({
  page,
}) => {
  await page.setViewportSize({ width: 430, height: 760 });
  await page.goto(baseURL);

  await expect(page).toHaveTitle(/Sparsha Counter/);
  await expect(
    page.getByText("Sparsha Demo Home Page", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("You have pushed the button this many times:", {
      exact: true,
    }),
  ).toBeVisible();
  await expect(page.getByText("0", { exact: true }).last()).toBeVisible();

  await page.getByRole("button", { name: "+" }).click({ force: true });
  await expect(page.getByText("1", { exact: true }).last()).toBeVisible();
});
