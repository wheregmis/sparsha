import { expect, test } from "@playwright/test";

const baseURL =
  process.env.SPARSH_KITCHEN_SINK_URL ?? "http://127.0.0.1:4173";
const pasteShortcut = process.platform === "darwin" ? "Meta+V" : "Control+V";

test("kitchen sink web flow matches the native interaction model", async ({
  context,
  page,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"], {
    origin: baseURL,
  });

  await page.goto(baseURL);
  await expect(page).toHaveTitle(/Kitchen Sink - Sparsh/);
  await expect(page.locator(".sparsh-dom-root")).toBeVisible();

  const checkbox = page.getByRole("checkbox", {
    name: "Focusable checkbox in the same tab order",
  });
  const singleLine = page.getByRole("textbox", {
    name: "Single-line input with clipboard + undo",
  });
  const email = page.getByRole("textbox", {
    name: "Email address...",
  });
  const notes = page.getByRole("textbox", {
    name: /Multiline notes/,
  });

  await page.mouse.click(40, 40);
  await page.keyboard.press("Tab");
  await expect(checkbox).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(singleLine).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(email).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(notes).toBeFocused();

  await page.evaluate(async (text) => {
    await navigator.clipboard.writeText(text);
  }, "Milestone 4 paste");
  await singleLine.click();
  await page.keyboard.press(pasteShortcut);
  await expect(singleLine).toHaveValue("Milestone 4 paste");

  const scrollView = page.getByRole("group", {
    name: "Kitchen sink scroll list",
  });
  const before = await scrollView.getAttribute("aria-valuetext");
  await scrollView.hover();
  await page.mouse.wheel(0, 1400);
  await expect
    .poll(async () => await scrollView.getAttribute("aria-valuetext"))
    .not.toBe(before);
});
