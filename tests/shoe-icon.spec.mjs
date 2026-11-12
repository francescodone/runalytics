import { test, expect } from "@playwright/test";

async function openShoe(page) {
  await page.goto("/__shoe_icon_poc.html");
  await expect(page.getByRole("button", { name: "Animate running shoe", exact: true })).toBeEnabled({ timeout: 30000 });
}

async function expectIdle(page) {
  await expect.poll(() => page.evaluate(() => window.shoeIcon.state.animating)).toBe(false);
  const initial = await page.evaluate(() => window.shoeIcon.state);
  await page.waitForTimeout(220);
  const settled = await page.evaluate(() => window.shoeIcon.state);
  expect(settled.pendingFrame).toBe(false);
  expect(settled.renderCount).toBe(initial.renderCount);
  expect(settled.rotation).toEqual([-.04, -.22, -.08]);
  expect(settled.height).toBe(.1);
}

test.describe("shoe icon PoC", () => {
  test.use({ reducedMotion: "no-preference" });

  for (const viewport of [{ width: 1440, height: 900 }, { width: 390, height: 844 }, { width: 320, height: 640 }]) {
    test(`renders a framed shoe at ${viewport.width}px`, async ({ page }) => {
      const errors = [];
      page.on("pageerror", error => errors.push(error.message));
      await page.setViewportSize(viewport);
      await openShoe(page);
      const pixels = await page.evaluate(() => {
        const source = document.querySelector("#stage canvas");
        const probe = document.createElement("canvas");
        probe.width = source.width;
        probe.height = source.height;
        const context = probe.getContext("2d");
        context.drawImage(source, 0, 0);
        const rgba = context.getImageData(0, 0, probe.width, probe.height).data;
        let coral = 0, minimumX = probe.width, maximumX = 0, minimumY = probe.height, maximumY = 0;
        for (let index = 0; index < rgba.length; index += 4) {
          if (rgba[index] > rgba[index + 1] * 1.15 && rgba[index] > rgba[index + 2] * 1.08 && rgba[index + 3] > 200) {
            coral++;
            const horizontal = index / 4 % probe.width;
            const vertical = Math.floor(index / 4 / probe.width);
            minimumX = Math.min(minimumX, horizontal);
            maximumX = Math.max(maximumX, horizontal);
            minimumY = Math.min(minimumY, vertical);
            maximumY = Math.max(maximumY, vertical);
          }
        }
        return { coral, minimumX, maximumX, minimumY, maximumY, width: probe.width, height: probe.height, overflow: document.documentElement.scrollWidth > innerWidth };
      });
      expect(pixels.coral).toBeGreaterThan(1200);
      expect(pixels.minimumX).toBeGreaterThan(8);
      expect(pixels.maximumX).toBeLessThan(pixels.width - 8);
      expect(pixels.minimumY).toBeGreaterThan(8);
      expect(pixels.maximumY).toBeLessThan(pixels.height - 8);
      expect(pixels.overflow).toBe(false);
      await expectIdle(page);
      await page.screenshot({ path: `/tmp/runalytics-shoe-${viewport.width}.png`, fullPage: true });
      expect(errors).toEqual([]);
    });
  }

  test("hover, click, replay, interruption and stop leave no render loop", async ({ page }) => {
    await openShoe(page);
    const shoe = page.getByRole("button", { name: "Animate running shoe", exact: true });
    await shoe.hover();
    await expect.poll(() => page.evaluate(() => window.shoeIcon.state.animating)).toBe(true);
    const start = await page.evaluate(() => window.shoeIcon.state.renderCount);
    await expect.poll(() => page.evaluate(() => window.shoeIcon.state.renderCount)).toBeGreaterThan(start + 3);
    await expectIdle(page);
    await shoe.click();
    await expect.poll(() => page.evaluate(() => window.shoeIcon.state.animating)).toBe(true);
    await page.keyboard.press("Escape");
    await expectIdle(page);
    await page.getByRole("button", { name: "Replay animation" }).click();
    await expect.poll(() => page.evaluate(() => window.shoeIcon.state.animating)).toBe(true);
    await page.getByRole("button", { name: "Stop animation" }).click();
    await expectIdle(page);
    await shoe.focus();
    await page.keyboard.press("Enter");
    await expect.poll(() => page.evaluate(() => window.shoeIcon.state.animating)).toBe(true);
    await expectIdle(page);
    await page.evaluate(() => { for (let index = 0; index < 12; index++) window.shoeIcon.play(); });
    await expectIdle(page);
  });

  test("touch activation plays once and finishes", async ({ browser }) => {
    const context = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, reducedMotion: "no-preference" });
    const page = await context.newPage();
    try {
      await page.goto("http://127.0.0.1:1500/__shoe_icon_poc.html");
      await expect(page.getByRole("button", { name: "Animate running shoe", exact: true })).toBeEnabled({ timeout: 30000 });
      await page.getByRole("button", { name: "Animate running shoe", exact: true }).tap();
      await expect.poll(() => page.evaluate(() => window.shoeIcon.state.animating)).toBe(true);
      await expectIdle(page);
    } finally {
      await context.close();
    }
  });

  test("color controls and transparent 1024px PNG export work", async ({ page }) => {
    await openShoe(page);
    await page.getByRole("button", { name: "Sage", exact: true }).click();
    await expect(page.getByRole("button", { name: "Sage", exact: true })).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator("#finish")).toHaveText("Sage knit / ivory foam");
    const image = await page.locator("#thumbnail").evaluate(async element => {
      await element.decode();
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 192;
      const context = canvas.getContext("2d");
      context.drawImage(element, 0, 0);
      return { width: element.naturalWidth, cornerAlpha: context.getImageData(0, 0, 1, 1).data[3] };
    });
    expect(image).toEqual({ width: 192, cornerAlpha: 0 });
    const downloaded = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download transparent PNG" }).click();
    const download = await downloaded;
    expect(download.suggestedFilename()).toBe("daily-runner-sage-1024.png");
    const stream = await download.createReadStream();
    const chunks = [];
    for await (const chunk of stream) chunks.push(chunk);
    const png = Buffer.concat(chunks);
    expect(png.subarray(1, 4).toString()).toBe("PNG");
    expect(png.readUInt32BE(16)).toBe(1024);
    expect(png.readUInt32BE(20)).toBe(1024);
    await expectIdle(page);
  });

  test("reduced motion, live preference changes and disposal cancel playback", async ({ page }) => {
    await openShoe(page);
    await page.evaluate(() => window.shoeIcon.play());
    await expect.poll(() => page.evaluate(() => window.shoeIcon.state.animating)).toBe(true);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await expectIdle(page);
    await page.getByRole("button", { name: "Replay animation" }).click();
    await expectIdle(page);
    await page.getByRole("button", { name: "Chalk", exact: true }).click();
    await expect(page.locator("#edition")).toHaveText("Chalk / soft-knit edition");
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.evaluate(() => { window.shoeIcon.play(); window.shoeIcon.dispose(); window.shoeIcon.play(); });
    const state = await page.evaluate(() => window.shoeIcon.state);
    expect(state.disposed).toBe(true);
    expect(state.animating).toBe(false);
    expect(state.pendingFrame).toBe(false);
  });
});