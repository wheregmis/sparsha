import { expect, test } from "@playwright/test";

const baseURL = process.env.SPARSH_TODO_URL ?? "http://127.0.0.1:4175";

test("todo route hash changes rebuild the correct screen", async ({ page }) => {
  await page.goto(`${baseURL}#/about`);
  await expect(page).toHaveTitle(/Todo - Sparsh/);
  await expect(page.getByText("Todo Route Demo")).toBeVisible();

  await page.evaluate(() => {
    window.location.hash = "#/";
  });

  await expect(
    page.getByRole("textbox", { name: "Add a task..." }),
  ).toBeVisible();
});

test("todo web worker results surface back into the UI", async ({ page }) => {
  await page.goto(baseURL);

  const input = page.getByRole("textbox", { name: "Add a task..." });
  await input.click();
  await input.type("alpha beta gamma");

  await expect(page.getByText(/Background analyzer:/)).toContainText("3 words");
});
