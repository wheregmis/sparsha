import { expect, test } from "@playwright/test";

const baseURL =
  process.env.SPARSH_SHOWCASE_URL ?? "http://127.0.0.1:4176";

test("showcase components preview stays interactive and can switch routes", async ({
  page,
}) => {
  await page.goto(baseURL);
  await expect(page).toHaveTitle(/Sparsh Showcase/);
  await expect(
    page.getByText("Basic component preview", { exact: true }).first(),
  ).toBeVisible();

  const checkbox = page.getByRole("checkbox", {
    name: "Showcase interactive checkbox",
  });
  await expect(checkbox).toBeChecked();
  await checkbox.focus();
  await expect(checkbox).toBeFocused();
  await page.keyboard.press("Space");
  await expect(checkbox).not.toBeChecked();

  const singleLine = page.getByRole("textbox", {
    name: "Showcase single-line input",
  });
  await singleLine.fill("preview@sparsh.dev");
  await expect(singleLine).toHaveValue(/preview@sparsh\.dev$/);

  const multiline = page.getByRole("textbox", {
    name: "Showcase multiline input",
  });
  await expect(multiline).toBeVisible();
  await multiline.focus();
  await expect(multiline).toBeFocused();

  const virtualList = page.getByRole("list", { name: "Showcase virtualized list" });
  await expect(virtualList).toBeVisible();

  await page.evaluate(() => {
    window.location.hash = "#/rendering";
  });
  await expect(
    page.getByText("Manual rendering checks", { exact: true }).first(),
  ).toBeVisible();
  await expect(page).toHaveURL(/#\/rendering$/);
  await expect(page.getByText("Rendering atlas", { exact: true }).first()).toBeVisible();
});

test("showcase rendering route loads directly from the hash", async ({ page }) => {
  await page.goto(`${baseURL}#/rendering`);
  await expect(page).toHaveTitle(/Sparsh Showcase/);
  await expect(
    page.getByText("Pixel alignment", { exact: true }).first(),
  ).toBeVisible();
  await expect(page.getByText("Rendering atlas", { exact: true }).first()).toBeVisible();

  await page.reload();
  await expect(page).toHaveURL(/#\/rendering$/);
  await expect(
    page.getByText("Text rendering", { exact: true }).first(),
  ).toBeVisible();
});
