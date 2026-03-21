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

  await checkbox.focus();
  await expect(checkbox).toBeFocused();
  await page.keyboard.press("Tab");
  await page.keyboard.type("a");
  await expect(singleLine).toHaveValue("a");
  await page.keyboard.press("Tab");
  await page.keyboard.type("b");
  await expect(email).toHaveValue("b");
  await page.keyboard.press("Tab");
  await page.keyboard.type("c");
  await expect(notes).toHaveValue("c");

  await page.evaluate(async (text) => {
    await navigator.clipboard.writeText(text);
  }, "Milestone 4 paste");
  await singleLine.focus();
  await page.keyboard.press(process.platform === "darwin" ? "Meta+A" : "Control+A");
  await page.keyboard.press(pasteShortcut);
  await expect(singleLine).toHaveValue(/Milestone 4 paste$/);

  const virtualList = page.getByRole("list", {
    name: "Kitchen sink virtualized list",
  });
  await expect(virtualList).toBeVisible();
  const before = await virtualList.textContent();
  const firstVirtualRow = page.getByText(/Virtual row \d+/).last();
  await expect(firstVirtualRow).toBeVisible();
  const rowBox = await firstVirtualRow.boundingBox();
  if (!rowBox) {
    throw new Error("virtualized list row bounding box was unavailable");
  }
  await page.mouse.move(rowBox.x + 16, rowBox.y + rowBox.height / 2);
  await page.mouse.wheel(0, 1400);
  await expect
    .poll(async () => await virtualList.textContent())
    .not.toBe(before);
});
