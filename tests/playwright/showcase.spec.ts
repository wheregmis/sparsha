import { expect, test, type Locator, type Page } from "@playwright/test";

const baseURL =
  process.env.SPARSH_SHOWCASE_URL ?? "http://127.0.0.1:4176";
const TOUCH_IDENTIFIER = 1;

const viewports = [
  { name: "desktop", width: 1440, height: 1024, stackedShell: false },
  { name: "tablet", width: 900, height: 1180, stackedShell: true },
  { name: "mobile", width: 390, height: 844, stackedShell: true },
] as const;

async function openShowcase(
  page: Page,
  viewport: (typeof viewports)[number],
  hash = "",
) {
  await page.setViewportSize({ width: viewport.width, height: viewport.height });
  await page.goto(`${baseURL}${hash}`);
  await expect(page).toHaveTitle(/Sparsh(?:a)? Showcase/);
}

async function expectNoHorizontalOverflow(page: Page) {
  const hasOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth > window.innerWidth + 1,
  );
  expect(hasOverflow).toBeFalsy();
}

async function expectStackedShell(page: Page) {
  const sidebarHeading = page.getByText("In scope", { exact: true }).first();
  const contentHeading = page
    .getByText("Basic component preview", { exact: true })
    .last();
  await expect(sidebarHeading).toBeVisible();
  await expect(contentHeading).toBeVisible();
  const sidebarBox = await sidebarHeading.evaluate((element) => {
    const rect = element.getBoundingClientRect();
    return { x: rect.x, y: rect.y };
  });
  const contentBox = await contentHeading.evaluate((element) => {
    const rect = element.getBoundingClientRect();
    return { x: rect.x, y: rect.y };
  });
  expect(Math.abs(sidebarBox.x - contentBox.x)).toBeLessThan(48);
  expect(sidebarBox.y).toBeLessThan(contentBox.y);
}

async function scrollUntilVisible(page: Page, locatorText: string) {
  return scrollLocatorUntilVisible(
    page,
    page.getByText(locatorText, { exact: true }).last(),
    locatorText,
    1200,
  );
}

async function scrollLocatorUntilVisible(
  page: Page,
  locator: Locator,
  description: string,
  deltaY: number,
) {
  for (let attempt = 0; attempt < 4; attempt += 1) {
    if (await locator.isVisible()) {
      return;
    }
    await page.mouse.wheel(0, deltaY);
    await page.waitForTimeout(100);
  }
  throw new Error(`Could not scroll "${description}" into view after 4 attempts.`);
}

async function swipeUp(page: Page, startY = 730, distance = 360) {
  await page.evaluate(
    ({ startY, distance, touchIdentifier }) => {
      const root = document.querySelector(".sparsha-dom-root");
      if (!(root instanceof HTMLElement) || typeof Touch === "undefined") {
        return;
      }
      const touchId = touchIdentifier;
      const startX = window.innerWidth * 0.5;
      const createTouch = (clientY: number) =>
        new Touch({
          identifier: touchId,
          target: root,
          clientX: startX,
          clientY,
          pageX: startX,
          pageY: clientY,
          screenX: startX,
          screenY: clientY,
        });
      const dispatch = (type: string, touches: Touch[], changedTouches: Touch[]) => {
        root.dispatchEvent(
          new TouchEvent(type, {
            bubbles: true,
            cancelable: true,
            touches,
            changedTouches,
            targetTouches: touches,
          }),
        );
      };
      const steps = 6;
      const startTouch = createTouch(startY);
      dispatch("touchstart", [startTouch], [startTouch]);
      for (let step = 1; step <= steps; step += 1) {
        const progress = step / steps;
        const y = startY - distance * progress;
        const moveTouch = createTouch(y);
        dispatch("touchmove", [moveTouch], [moveTouch]);
      }
      const endTouch = createTouch(startY - distance);
      dispatch("touchend", [], [endTouch]);
    },
    { startY, distance, touchIdentifier: TOUCH_IDENTIFIER },
  );
  await page.waitForTimeout(100);
}

for (const viewport of viewports) {
  test(`${viewport.name}: showcase components preview stays interactive and can switch routes`, async ({
    page,
  }) => {
    await openShowcase(page, viewport);
    await expect(
      page.getByText("Basic component preview", { exact: true }).first(),
    ).toBeVisible();
    await expectNoHorizontalOverflow(page);
    if (viewport.stackedShell) {
      await expectStackedShell(page);
    }
    await page.waitForTimeout(1500);

    const checkbox = page.getByRole("checkbox", {
      name: "Showcase interactive checkbox",
    });
    await expect(checkbox).toBeChecked();
    await checkbox.focus();
    await expect(checkbox).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(checkbox).not.toBeChecked();

    const singleLine = page.getByRole("textbox", {
      name: "Showcase single-line input",
    });
    await page.keyboard.press("Tab");
    await expect(singleLine).toBeFocused();

    const multiline = page.getByRole("textbox", {
      name: "Showcase multiline input",
    });
    await page.keyboard.press("Tab");
    await expect(multiline).toBeFocused();

    await scrollUntilVisible(page, "Virtualized list");
    const twoAxisScroll = page.getByText("Two-axis scroll", { exact: true }).last();
    const virtualListHeading = page.getByText("Virtualized list", { exact: true }).last();
    await expect(twoAxisScroll).toBeVisible();
    await expect(virtualListHeading).toBeVisible();

    const virtualList = page.getByRole("list", { name: "Showcase virtualized list" });
    await expect(virtualList).toBeVisible();
    await expect(page.getByText("Animations", { exact: true }).first()).toBeVisible();

    const renderingButton = page.getByRole("button", { name: "Rendering" });
    await scrollLocatorUntilVisible(
      page,
      renderingButton,
      "Rendering route button",
      -1200,
    );
    await renderingButton.click({ force: true });
    await expect(
      page.getByText("Manual rendering checks", { exact: true }).first(),
    ).toBeVisible();
    await expect(page).toHaveURL(/#\/rendering$/);
    await expectNoHorizontalOverflow(page);
    await expect(page.getByText("Rendering atlas", { exact: true }).first()).toBeVisible();
  });

  test(`${viewport.name}: showcase rendering route loads directly from the hash`, async ({
    page,
  }) => {
    await openShowcase(page, viewport, "#/rendering");
    await expect(
      page.getByText("Pixel alignment", { exact: true }).first(),
    ).toBeVisible();
    await expect(page.getByText("Rendering atlas", { exact: true }).first()).toBeVisible();
    await expectNoHorizontalOverflow(page);

    await page.reload();
    await expect(page).toHaveURL(/#\/rendering$/);
    await expect(
      page.getByText("Text rendering", { exact: true }).first(),
    ).toBeVisible();
  });
}

test("mobile: touch swipe scrolls components content", async ({ page }) => {
  const mobileViewport = viewports.find((viewport) => viewport.name === "mobile");
  if (!mobileViewport) {
    throw new Error("Missing mobile viewport configuration.");
  }
  await openShowcase(page, mobileViewport);
  const sectionHeading = page.getByText("Basic component preview", { exact: true }).last();
  await expect(sectionHeading).toBeVisible();
  const initialY = await sectionHeading.evaluate(
    (element) => element.getBoundingClientRect().y,
  );
  await swipeUp(page);
  const nextY = await sectionHeading.evaluate((element) => element.getBoundingClientRect().y);
  expect(nextY).toBeLessThan(initialY - 16);
});
