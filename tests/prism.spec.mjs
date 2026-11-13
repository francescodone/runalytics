import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { installDeskFixture } from "./desk.fixture.mjs";

test.beforeEach(async ({ page }) => installDeskFixture(page));

for (const theme of ["light", "dark"]) for (const width of [320, 1440]) {
  test(`inline messages share a surface ${theme} ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 844 });
    await page.goto(`/desk.html?theme=${theme}#training/week`);
    await page.getByRole("button", { name: "Schedule race or test", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "Schedule race or test", exact: true });
    const note = dialog.locator(".context-note").first();
    await expect(note).toBeVisible();
    await page.evaluate(() => { window.__deskFail = "schedule_benchmark"; });
    await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
    const error = dialog.getByRole("alert");
    await expect(error).toContainText("Unable to save right now");
    const styles = locator => locator.evaluate(element => {
      const style = getComputedStyle(element);
      return { padding: style.padding, radius: style.borderRadius, background: style.backgroundColor, fontSize: style.fontSize, lineHeight: style.lineHeight, border: style.borderColor, fits: element.scrollWidth <= element.clientWidth };
    });
    const noteStyle = await styles(note);
    const errorStyle = await styles(error);
    expect({ ...errorStyle, border: noteStyle.border }).toEqual(noteStyle);
    expect(errorStyle.border).not.toBe(noteStyle.border);
    expect(errorStyle.fits).toBe(true);
    await expect(dialog.getByLabel("Distance (km)")).toHaveValue("5");
    await dialog.screenshot({ path: testInfo.outputPath("inline-messages.png") });
  });
}

test("mobile touch clicks give one light haptic for buttons and cards", async ({ browser }) => {
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, reducedMotion: "reduce" });
  const page = await context.newPage();
  await installDeskFixture(page);
  await page.addInitScript(() => { window.isTauri = true; });
  await page.goto("/desk.html#health/pace");
  await expect(page.getByRole("group", { name: "Pace workout type" })).toBeVisible();
  await page.evaluate(() => {
    window.__hapticCalls = [];
    const original = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = (command, args) => {
      if (command === "plugin:haptics|impact_feedback") {
        window.__hapticCalls.push(args);
        return Promise.resolve();
      }
      return original(command, args);
    };
  });
  const calls = () => page.evaluate(() => window.__hapticCalls);
  const long = page.getByRole("button", { name: "Long", exact: true });
  await long.tap();
  await expect.poll(calls).toEqual([{ style: "light" }]);
  await long.press("Enter");
  expect(await calls()).toHaveLength(1);
  await long.click();
  expect(await calls()).toHaveLength(1);
  await long.evaluate(element => { element.disabled = true; });
  const disabledBounds = await long.boundingBox();
  await page.touchscreen.tap(disabledBounds.x + disabledBounds.width / 2, disabledBounds.y + disabledBounds.height / 2);
  expect(await calls()).toHaveLength(1);
  await long.evaluate(element => { element.disabled = false; });
  await page.getByRole("heading", { name: "Health", exact: true }).tap();
  expect(await calls()).toHaveLength(1);
  const bounds = await long.boundingBox();
  const touch = await context.newCDPSession(page);
  await touch.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: bounds.x + 12, y: bounds.y + 12 }] });
  expect(await calls()).toHaveLength(1);
  await touch.send("Input.dispatchTouchEvent", { type: "touchCancel", touchPoints: [] });
  expect(await calls()).toHaveLength(1);
  await page.goto("/desk.html#journal");
  await expect(page.locator(".activity-row").first()).toBeVisible();
  await page.evaluate(() => {
    window.__hapticCalls = [];
    const original = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = (command, args) => {
      if (command === "plugin:haptics|impact_feedback") { window.__hapticCalls.push(args); return Promise.resolve(); }
      return original(command, args);
    };
  });
  await page.locator(".activity-row").first().tap();
  await expect.poll(calls).toEqual([{ style: "light" }]);
  await context.close();
});

test("mobile browser haptics use a brief pulse and unsupported hardware stays usable", async ({ browser }) => {
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, reducedMotion: "reduce" });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await installDeskFixture(page);
  await page.goto("/desk.html#health/pace");
  await expect(page.getByRole("group", { name: "Pace workout type" })).toBeVisible();
  await page.evaluate(() => {
    window.isTauri = false;
    window.__vibrationCalls = [];
    Object.defineProperty(navigator, "vibrate", { configurable: true, value: duration => { window.__vibrationCalls.push(duration); return true; } });
  });
  const long = page.getByRole("button", { name: "Long", exact: true });
  const speed = page.getByRole("button", { name: "Speed", exact: true });
  await long.tap();
  expect(await page.evaluate(() => window.__vibrationCalls)).toEqual([8]);
  await speed.click();
  expect(await page.evaluate(() => window.__vibrationCalls)).toEqual([8]);
  await page.evaluate(() => { Object.defineProperty(navigator, "vibrate", { configurable: true, value: undefined }); });
  await long.tap();
  await expect(long).toHaveAttribute("aria-pressed", "true");
  await page.evaluate(() => {
    window.isTauri = true;
    const original = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = (command, args) => command === "plugin:haptics|impact_feedback" ? Promise.reject(new Error("Haptics unavailable")) : original(command, args);
  });
  await speed.tap();
  await expect(speed).toHaveAttribute("aria-pressed", "true");
  expect(errors).toEqual([]);
  await context.close();
});

for (const theme of ["light", "dark"]) for (const width of [320, 390, 1440]) {
  test(`setup messages share app surfaces ${theme} ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 844 });
    await page.goto(`/desk.html?theme=${theme}#journal`);
    await page.evaluate(() => window.__TAURI_INTERNALS__.invoke("disconnect_provider", { provider: "coros" }));
    await page.getByRole("navigation", { name: "Workspaces" }).getByRole("link", { name: "Training", exact: true }).click();
    const message = page.locator(".setup-band");
    await expect(message).toBeVisible();
    await expect(message).toHaveCSS("border-radius", "8px");
    await expect(message).toHaveCSS("padding", "20px");
    await expect(message).not.toHaveCSS("background-image", "none");
    expect(await message.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
    await expect(message.getByRole("button", { name: "Connect COROS" })).toBeVisible();
    await message.screenshot({ path: testInfo.outputPath("setup-message.png") });
    await page.emulateMedia({ contrast: "more" });
    await expect(message).toHaveCSS("background-image", "none");
  });
}

test("training toolbar tooltips escape clipping on hover and keyboard focus", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/desk.html#training/week");
  const button = page.getByRole("button", { name: "Previous week" });
  const tooltip = button.locator(".control-tooltip");
  const toolbar = page.locator(".training-toolbar");
  await button.hover();
  await expect(tooltip).toBeVisible();
  await expect(toolbar).toHaveCSS("overflow", "visible");
  await expect(toolbar).toHaveCSS("z-index", "30");
  const geometry = await tooltip.evaluate(element => ({ tooltip: element.getBoundingClientRect().bottom, toolbar: element.closest(".desk-toolbar").getBoundingClientRect().bottom }));
  expect(geometry.tooltip).toBeGreaterThan(geometry.toolbar);
  await page.mouse.move(0, 0);
  await page.keyboard.press("Tab");
  await button.focus();
  await expect(tooltip).toBeVisible();
  await expect(toolbar).toHaveCSS("z-index", "30");
  await page.keyboard.press("Escape");
  await expect(tooltip).toBeHidden();
});

for (const theme of ["light", "dark"]) {
  for (const width of [820, 1440]) {
    test(`desktop activity review lands softly without stretching ${theme} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 900 });
      await page.emulateMedia({ reducedMotion: "no-preference" });
      await page.goto(`/desk.html?theme=${theme}#journal`);
      const row = page.locator(".activity-row").first();
      await row.evaluate(element => {
        element.addEventListener("click", () => {
          window.__activityMorph = { source: element.getBoundingClientRect().toJSON(), frames: [] };
          const capture = () => {
            const inspector = document.querySelector('.desk-inspector[aria-label="Activity review"]');
            if (inspector) window.__activityMorph.frames.push(inspector.getBoundingClientRect().toJSON());
            if (window.__activityMorph.frames.length < 40) requestAnimationFrame(capture);
          };
          requestAnimationFrame(capture);
        }, { once: true });
      });
      await row.click();
      const inspector = page.locator('.desk-inspector[aria-label="Activity review"]');
      await expect.poll(() => page.evaluate(() => window.__activityMorph.frames.length)).toBe(40);
      const { source, frames } = await page.evaluate(() => window.__activityMorph);
      expect(frames.at(-1).x).toBeGreaterThan(source.x + source.width);
      expect(frames.every(frame => frame.height > frames.at(-1).height * .97 && frame.width > frames.at(-1).width * .97)).toBe(true);
      expect(frames.every(frame => Math.abs(frame.height - frames.at(-1).height) < 1)).toBe(true);
      expect(frames.some(frame => frame.y > frames.at(-1).y + .2)).toBe(true);
      expect(Math.abs(frames[0].y - frames.at(-1).y)).toBeLessThan(24);
      await expect(row).toBeVisible();
      await expect(inspector).toBeFocused();
      await page.screenshot({ path: testInfo.outputPath("activity-morph-open.png") });
      const closing = await inspector.evaluate(async element => {
        element.querySelector('button[aria-label="Back"]').click();
        const frames = [];
        for (let index = 0; index < 24; index++) {
          await new Promise(requestAnimationFrame);
          if (element.isConnected) frames.push({ ...element.getBoundingClientRect().toJSON(), opacity: Number(getComputedStyle(element).opacity) });
        }
        return frames;
      });
      expect(closing.some(frame => frame.opacity > 0 && frame.opacity < .9)).toBe(true);
      expect(closing.every(frame => frame.height > frames.at(-1).height * .97)).toBe(true);
      await expect(inspector).toHaveCount(0);
      await expect(row).toBeFocused();
      await page.keyboard.press("Enter");
      await expect(inspector).toBeFocused();
      await page.keyboard.press("Escape");
      await expect(inspector).toHaveCount(0);
      await expect(row).toBeFocused();
    });
  }
}

test("compact activity review lands softly without stretching", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/desk.html#journal");
  const row = page.locator(".activity-row").first();
  await row.evaluate(element => {
    element.addEventListener("click", () => {
      window.__activityMorph = { source: element.getBoundingClientRect().toJSON(), frames: [] };
      const capture = () => {
        const inspector = document.querySelector('.desk-inspector[aria-label="Activity review"]');
        if (inspector) window.__activityMorph.frames.push(inspector.getBoundingClientRect().toJSON());
        if (window.__activityMorph.frames.length < 32) requestAnimationFrame(capture);
      };
      requestAnimationFrame(capture);
    }, { once: true });
  });
  await row.click();
  const inspector = page.locator('.desk-inspector[aria-label="Activity review"]');
  await expect.poll(() => page.evaluate(() => window.__activityMorph.frames.length)).toBe(32);
  const { frames } = await page.evaluate(() => window.__activityMorph);
  expect(frames.every(frame => frame.height > frames.at(-1).height * .97 && frame.width > frames.at(-1).width * .97)).toBe(true);
  expect(frames.every(frame => Math.abs(frame.height - frames.at(-1).height) < 1)).toBe(true);
  expect(frames.some(frame => frame.y > frames.at(-1).y + .2)).toBe(true);
  expect(Math.abs(frames[0].x - frames.at(-1).x)).toBeLessThan(32);
  expect(Math.abs(frames[0].y - frames.at(-1).y)).toBeLessThan(24);
  await expect(inspector).toBeFocused();
  const closing = await inspector.evaluate(async element => {
    element.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    const frames = [];
    for (let index = 0; index < 24; index++) {
      await new Promise(requestAnimationFrame);
      if (element.isConnected) frames.push({ ...element.getBoundingClientRect().toJSON(), opacity: Number(getComputedStyle(element).opacity) });
    }
    return frames;
  });
  expect(closing.some(frame => frame.opacity > 0 && frame.opacity < .9)).toBe(true);
  expect(closing.every(frame => frame.height > frames.at(-1).height * .97)).toBe(true);
  await expect(inspector).toHaveCount(0);
  await expect(row).toBeFocused();
});

test("desktop activity review respects reduced motion", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/desk.html#journal");
  const row = page.locator(".activity-row").first();
  await row.click();
  const inspector = page.locator('.desk-inspector[aria-label="Activity review"]');
  await expect(inspector).toHaveCSS("transform", "none");
  await expect(row).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(inspector).toHaveCount(0);
  await expect(row).toBeFocused();
});

test.describe("compact navigation latency", () => {
  test.use({ hasTouch: true });

  for (const width of [320, 390, 768]) for (const theme of ["light", "dark"]) for (const reducedMotion of ["no-preference", "reduce"]) {
  test(`mobile horizon is ready within two frames of touch navigation ${width}px ${theme} ${reducedMotion}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 844 });
    await page.emulateMedia({ reducedMotion });
    await page.goto(`/desk.html?theme=${theme}#training/month`);
    await expect(page.locator(".month-board")).toBeVisible();
    await page.evaluate(() => {
      window.addEventListener("hashchange", () => {
        requestAnimationFrame(() => requestAnimationFrame(() => {
          const ribbon = document.querySelector(".mobile-date-ribbon");
          window.__mobileHorizonReady = Boolean(ribbon && ribbon.getBoundingClientRect().width > 0 && !ribbon.closest("[inert]"));
        }));
      }, { once: true });
    });
    await page.getByRole("link", { name: "Week", exact: true }).tap();
    await expect.poll(() => page.evaluate(() => window.__mobileHorizonReady)).toBe(true);
    await expect(page.locator('[data-reveal="waiting"], [data-reveal="visible"]')).toHaveCount(0);
    if (width === 390 && reducedMotion === "no-preference") await page.screenshot({ path: testInfo.outputPath("instant-mobile-week.png") });
  });
  }

  test("mobile workspaces retain mounted filters when returning", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto("/desk.html#journal");
    await expect(page.locator(".activity-library")).toBeVisible();
    await page.getByRole("button", { name: "Filters", exact: true }).tap();
    const search = page.getByRole("searchbox", { name: "Search activities" });
    await search.fill("easy");
    await search.evaluate(element => { window.__retainedSearch = element; });
    await page.getByRole("link", { name: "Training", exact: true }).tap();
    await expect(page.locator(".mobile-date-ribbon")).toBeVisible();
    await page.getByRole("link", { name: "Activities", exact: true }).tap();
    await expect(search).toBeVisible();
    await expect(search).toHaveValue("easy");
    expect(await search.evaluate(element => element === window.__retainedSearch)).toBe(true);
    await expect(page.locator('.motion-change[data-transition-state="exiting"]')).toHaveCount(0);
  });

  test("mobile navigation responds on touch down and cancels without navigating", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto("/desk.html#training/week");
    await expect(page.locator(".mobile-date-ribbon")).toBeVisible();
    const link = page.getByRole("link", { name: "Activities", exact: true });
    const bounds = await link.boundingBox();
    const session = await page.context().newCDPSession(page);
    await session.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: bounds.x + bounds.width / 2, y: bounds.y + bounds.height / 2 }] });
    await expect(link).toHaveAttribute("data-aura-pressed", "true");
    await expect(page).toHaveURL(/#training\/week$/);
    await session.send("Input.dispatchTouchEvent", { type: "touchCancel", touchPoints: [] });
    await expect(link).not.toHaveAttribute("data-aura-pressed", "true");
    await expect(page).toHaveURL(/#training\/week$/);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await session.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: bounds.x + bounds.width / 2, y: bounds.y + bounds.height / 2 }] });
    await expect(link).toHaveAttribute("data-aura-pressed", "true");
    await expect(link).toHaveCSS("scale", "1");
    await session.send("Input.dispatchTouchEvent", { type: "touchCancel", touchPoints: [] });
    await session.detach();
    await link.tap();
    await expect(page.locator(".workspace-heading h1")).toHaveText("Activities");
  });

  test("mobile horizon changes reuse the loaded schedule", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto("/desk.html#training/week");
    await expect(page.locator(".mobile-date-ribbon")).toBeVisible();
    const initialCalls = await page.evaluate(() => window.__deskCalls.filter(call => call.command === "calendar_events").length);
    for (const [name, selector] of [["Month", ".month-board"], ["Plan", ".plan-studio"], ["Week", ".mobile-date-ribbon"]]) {
      await page.getByRole("link", { name, exact: true }).tap();
      await expect(page.locator(selector)).toBeVisible();
      await expect(page.getByText("Loading schedule", { exact: true })).toHaveCount(0);
      expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "calendar_events").length)).toBe(initialCalls);
    }
  });

  test("mobile retained activity history stays visible during revalidation", async ({ page }) => {
    await page.goto("/desk.html#journal");
    await expect(page.locator(".activity-row")).toHaveCount(9);
    await page.getByRole("link", { name: "Training", exact: true }).tap();
    await expect(page.locator(".mobile-date-ribbon")).toBeVisible();
    await page.evaluate(() => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        if (command === "all_activities") await new Promise(resolve => { window.__releaseActivityReload = resolve; });
        return invoke(command, args);
      };
    });
    await page.getByRole("link", { name: "Activities", exact: true }).tap();
    await expect.poll(() => page.evaluate(() => typeof window.__releaseActivityReload)).toBe("function");
    await expect(page.locator(".activity-row:visible")).toHaveCount(9);
    await expect(page.getByText("Loading training history", { exact: true })).toHaveCount(0);
    await page.evaluate(() => window.__releaseActivityReload?.());
  });

  test("mobile retained training adopts refreshed plan data", async ({ page }) => {
    await page.goto("/desk.html#training/plan");
    await expect(page.getByRole("button", { name: "Show details", exact: true })).toBeVisible();
    await page.getByRole("link", { name: "Activities", exact: true }).tap();
    await expect(page.locator(".activity-library")).toBeVisible();
    await page.evaluate(() => {
      window.__deskRemotePlan = null;
      window.dispatchEvent(new Event("focus"));
    });
    await expect.poll(() => page.evaluate(() => window.__deskRemotePlan === undefined)).toBe(true);
    await page.getByRole("link", { name: "Training", exact: true }).tap();
    await expect(page.getByRole("button", { name: "Create plan", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "End current plan", exact: true })).toHaveCount(0);
  });

  for (const reducedMotion of ["no-preference", "reduce"]) {
    test(`mobile rapid changes remain mounted and interruptible ${reducedMotion}`, async ({ page }) => {
      await page.emulateMedia({ reducedMotion });
      await page.goto("/desk.html#training/week");
      await expect(page.locator(".mobile-date-ribbon")).toBeVisible();
      const result = await page.evaluate(async () => {
        const host = document.querySelector(".training-toolbar").nextElementSibling;
        const frames = [];
        for (const horizon of ["month", "plan", "week", "month", "week"]) {
          document.querySelector(`.horizon-switch a[href="#training/${horizon}"]`).click();
          await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
          frames.push({ sameHost: host === document.querySelector(".training-toolbar").nextElementSibling, opacity: Number(getComputedStyle(host).opacity), transform: getComputedStyle(host).transform, filter: getComputedStyle(host).filter, exits: document.querySelectorAll('[data-transition-state="exiting"]').length });
        }
        return { frames, animations: host.getAnimations().length };
      });
      expect(result.frames.every(frame => frame.sameHost && frame.opacity >= .9 && frame.transform === "none" && frame.filter === "none" && frame.exits === 0)).toBe(true);
      if (reducedMotion === "reduce") expect(result.animations).toBe(0);
      await expect(page.locator(".mobile-date-ribbon")).toBeVisible();
      await expect(page.locator(".month-board, .plan-studio")).toHaveCount(0);
    });
  }
});

for (const width of [390, 1440]) test(`native view handoffs keep their host and never wait for an exit ${width}px`, async ({ page }) => {
  await page.setViewportSize({ width, height: 900 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.addInitScript(() => { window.isTauri = true; });
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto("/desk.html#training");
  await expect(page.locator(".training-context")).toBeVisible();
  for (const destination of ["journal", "health", "training", "journal", "health"]) {
    const result = await page.evaluate(async workspace => {
      const host = document.querySelector("#workspace > .motion-change");
      location.hash = workspace;
      for (let frame = 0; frame < 3; frame++) await new Promise(requestAnimationFrame);
      return { same: host === document.querySelector("#workspace > .motion-change"), exiting: document.querySelectorAll('[data-transition-state="exiting"]').length };
    }, destination);
    expect(result).toEqual({ same: true, exiting: 0 });
    await expect(page.locator("#workspace")).toHaveClass(new RegExp(`workspace-${destination}`));
  }
  expect(errors).toEqual([]);
});

for (const runtime of ["browser", "native"]) for (const reducedMotion of ["no-preference", "reduce"]) {
  test(`appearance changes respect motion preference ${runtime} ${reducedMotion}`, async ({ page }) => {
    await page.emulateMedia({ reducedMotion });
    await page.addInitScript(native => {
      if (native) window.isTauri = true;
      const start = document.startViewTransition?.bind(document);
      window.__appearanceTransitions = 0;
      document.startViewTransition = (...args) => {
        window.__appearanceTransitions++;
        return start?.(...args);
      };
    }, runtime === "native");
    await page.goto("/desk.html?theme=light#settings/appearance");
    await expect(page.getByRole("radio", { name: "Light", exact: true })).toBeChecked();
    const initialTransitions = await page.evaluate(() => window.__appearanceTransitions);
    await page.getByRole("radio", { name: "Dark", exact: true }).check();
    await expect(page.locator("html")).toHaveAttribute("data-desk-theme", "dark");
    await expect.poll(() => page.evaluate(() => window.__appearanceTransitions)).toBe(initialTransitions + (runtime === "native" || reducedMotion === "reduce" ? 0 : 1));
    await page.getByRole("radio", { name: "Light", exact: true }).check();
    await expect(page.locator("html")).toHaveAttribute("data-desk-theme", "light");
    await page.getByRole("radio", { name: "Raspberry Relay", exact: true }).check();
    await expect(page.locator("html")).toHaveAttribute("data-desk-palette", "carnival");
    await expect.poll(() => page.evaluate(() => window.__appearanceTransitions)).toBe(initialTransitions + (runtime === "native" || reducedMotion === "reduce" ? 0 : 3));
  });
}

for (const runtime of ["browser", "native"]) for (const theme of ["light", "dark"]) {
  for (const width of [320, 390, 820, 1440]) {
    test(`calendar panel morphs from its tile and returns ${runtime} ${theme} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 900 });
      await page.emulateMedia({ reducedMotion: "no-preference" });
      if (runtime === "native") await page.addInitScript(() => { window.isTauri = true; });
      await page.goto(`/desk.html?theme=${theme}#training`);
      if (width <= 800) await page.locator('.mobile-date-ribbon button:has(.has-session)').first().click();
      await page.locator(".session-tile:visible").first().evaluate(element => element.setAttribute("data-morph-source", ""));
      const tile = page.locator("[data-morph-source]");
      await expect(tile).toBeVisible();
      await expect(tile).toHaveAttribute("data-reveal", "settled");
      await tile.scrollIntoViewIfNeeded();
      await tile.evaluate(element => {
        const source = element.getBoundingClientRect().toJSON();
        const frames = [];
        window.__morphFrames = { source, frames };
        const capture = () => {
          const modal = document.querySelector('.desk-inspector[aria-label="Session"]');
          if (modal) frames.push(modal.getBoundingClientRect().toJSON());
          if (frames.length < 32) requestAnimationFrame(capture);
        };
        element.addEventListener("click", () => requestAnimationFrame(capture), { once: true });
      });
      await tile.click();
      await expect.poll(() => page.evaluate(() => window.__morphFrames.frames.length)).toBe(32);
      const frames = await page.evaluate(() => window.__morphFrames);
      expect(frames.frames.length).toBeGreaterThan(10);
      const first = frames.frames[0];
      const last = frames.frames.at(-1);
      expect(Math.abs(first.width - frames.source.width)).toBeLessThan(Math.abs(last.width - frames.source.width) * .5);
      expect(Math.abs(first.height - frames.source.height)).toBeLessThan(Math.abs(last.height - frames.source.height) * .5);
      expect(Math.abs(first.x - frames.source.x)).toBeLessThan(Math.max(5, Math.abs(last.x - frames.source.x) * .5));
      expect(Math.abs(first.y - frames.source.y)).toBeLessThan(Math.max(5, Math.abs(last.y - frames.source.y) * .5));
      expect(frames.frames.some(frame => frame.width > first.width + 1 && frame.width < last.width - 1)).toBe(true);
      await expect(tile).toHaveCSS("visibility", "visible");
      const inspector = page.getByRole("dialog", { name: "Session", exact: true });
      await expect(inspector).toBeFocused();
      if (runtime === "native") await expect(inspector).toHaveCSS("backdrop-filter", "none");
      await expect(page.locator("#workspace")).toHaveCSS("overflow", "hidden");
      await page.screenshot({ path: testInfo.outputPath("expanded-session.png") });
      const closing = await inspector.evaluate(async element => {
        const start = element.getBoundingClientRect().toJSON();
        element.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
        const widths = [];
        for (let index = 0; index < 24; index++) {
          await new Promise(requestAnimationFrame);
          if (element.isConnected) widths.push(element.getBoundingClientRect().width);
        }
        return { start, widths };
      });
      expect(closing.widths.length).toBeGreaterThan(1);
      expect(closing.widths.every(value => Math.abs(value - closing.start.width) < 1)).toBe(true);
      await expect(inspector).toHaveCount(0);
      await expect(tile).toHaveCSS("visibility", "visible");
      await expect(tile).toBeFocused();
      await expect(page.locator("#workspace")).not.toHaveCSS("overflow", "hidden");
      await page.keyboard.press("Enter");
      await expect(inspector).toBeFocused();
      await inspector.getByRole("button", { name: "Close Session", exact: true }).click();
      await expect(inspector).toHaveCount(0);
      await expect(tile).toBeFocused();
    });
  }
}

test("calendar panel respects reduced motion and interrupted dismissal", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/desk.html#training");
  await page.locator('.mobile-date-ribbon button:has(.has-session)').first().click();
  await page.locator(".session-tile:visible").first().evaluate(element => element.setAttribute("data-morph-source", ""));
  const tile = page.locator("[data-morph-source]");
  await tile.click();
  const inspector = page.getByRole("dialog", { name: "Session", exact: true });
  await expect(inspector).toHaveCSS("transform", "none");
  await expect(tile).toHaveCSS("visibility", "visible");
  await page.keyboard.press("Escape");
  await expect(inspector).toHaveCount(0);
  await expect(tile).toBeFocused();
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await tile.evaluate(element => {
    element.click();
    requestAnimationFrame(() => document.querySelector('.desk-inspector[aria-label="Session"]')?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  });
  await expect(inspector).toHaveCount(0);
  await expect(tile).toHaveCSS("visibility", "visible");
  await expect(tile).toBeFocused();
});

test.describe("calendar month touch morph", () => {
  test.use({ hasTouch: true });
  for (const width of [390, 1440]) {
    test(`calendar panel month touch ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 844 });
      await page.emulateMedia({ reducedMotion: "no-preference" });
      await page.goto("/desk.html#training/month");
      if (width <= 800) await page.locator('.month-day:has(.session-tile) .month-number').first().tap();
      await page.locator(".session-tile:visible").first().evaluate(element => element.setAttribute("data-morph-source", ""));
      const tile = page.locator("[data-morph-source]");
      await tile.tap();
      const inspector = page.getByRole("dialog", { name: "Session", exact: true });
      await expect(inspector).toBeFocused();
      await expect(tile).toHaveCSS("visibility", "visible");
      await inspector.getByRole("button", { name: "Close Session", exact: true }).tap();
      await expect(inspector).toHaveCount(0);
      await expect(tile).toHaveCSS("visibility", "visible");
      await expect(tile).toBeFocused();
    });
  }
});

for (const theme of ["light", "dark"]) {
  test(`plan cards remain inside their scroller on hover ${theme}`, async ({ page }, testInfo) => {
    await page.emulateMedia({ reducedMotion: "no-preference" });
    for (const width of [320, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(`/desk.html?theme=${theme}#training/plan`);
      const scroller = page.getByRole("region", { name: "Plan weeks" });
      const row = scroller.locator(".roadmap-row").first();
      await expect(row).toBeVisible();
      for (const edge of ["left", "right"]) {
        const bounds = await row.boundingBox();
        await row.hover({ position: { x: edge === "left" ? 8 : bounds.width - 8, y: bounds.height / 2 } });
        await expect(row).toHaveAttribute("data-aura-tracking", "true");
        await expect.poll(() => row.evaluate(element => {
          const card = element.getBoundingClientRect();
          const container = element.closest(".plan-roadmap").getBoundingClientRect();
          return Math.min(card.left - container.left, container.right - card.right);
        })).toBeGreaterThanOrEqual(4);
      }
      await page.screenshot({ path: testInfo.outputPath(`plan-hover-${width}.png`) });
    }
  });

  for (const width of [320, 1440]) test(`glass button borders stay continuous on hover and press ${theme} ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto(`/desk.html?theme=${theme}#settings/thresholds`);
    const control = page.getByRole("button", { name: "Restore defaults", exact: true });
    await expect(control).toBeVisible();
    await page.mouse.move(0, 0);
    const size = await control.evaluate(element => ({ width: element.offsetWidth, height: element.offsetHeight }));
    const reflection = () => control.evaluate(element => {
      const style = getComputedStyle(element, "::after");
      return { image: style.backgroundImage, opacity: style.opacity, mask: style.maskComposite.split(", "), events: style.pointerEvents, padding: style.padding, inset: style.inset };
    });
    expect(await reflection()).toMatchObject({ opacity: "0", mask: ["exclude", "exclude"], events: "none", padding: "2px", inset: "-1px" });
    await control.hover({ position: { x: 8, y: size.height / 2 } });
    await expect.poll(async () => (await reflection()).opacity).toBe("0.8");
    await expect(control).toHaveAttribute("data-aura-tracking", "true");
    const first = await reflection();
    expect(first.image).toContain("linear-gradient");
    await expect.poll(() => control.evaluate(element => parseFloat(element.style.getPropertyValue("--aura-x")))).toBeLessThan(15);
    await control.hover({ position: { x: size.width - 8, y: size.height / 2 } });
    await expect.poll(() => control.evaluate(element => parseFloat(element.style.getPropertyValue("--aura-x")))).toBeGreaterThan(85);
    expect((await reflection()).image).toBe(first.image);
    await page.mouse.down();
    await expect.poll(async () => (await reflection()).opacity).toBe("1");
    await page.screenshot({ path: testInfo.outputPath("glass-rim-pressed.png") });
    await page.mouse.move(0, 0);
    await page.mouse.up();
    await expect.poll(async () => (await reflection()).opacity).toBe("0");
    expect(await control.evaluate(element => ({ width: element.offsetWidth, height: element.offsetHeight }))).toEqual(size);
    await page.keyboard.press("Tab");
    await control.focus();
    await expect(control).toHaveCSS("outline-style", "solid");
    await expect(control).toHaveCSS("--aura-x", "50%");
    await expect(control).toHaveCSS("--aura-y", "50%");
    await expect.poll(async () => (await reflection()).opacity).toBe("0.8");
    await page.emulateMedia({ contrast: "more" });
    expect(await control.evaluate(element => getComputedStyle(element, "::after").display)).toBe("none");
    await expect(control).toHaveCSS("outline-style", "solid");
    await page.emulateMedia({ contrast: "no-preference" });
    await control.evaluate(element => element.blur());
    await control.evaluate(element => { element.disabled = true; });
    await control.hover();
    expect(await control.evaluate(element => getComputedStyle(element, "::after").content)).toBe("none");
  });

  for (const width of [320, 390, 1440]) test(`reference button selection remains distinct from hover ${theme} ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto(`/desk.html?theme=${theme}#health/pace`);
    const group = page.getByRole("group", { name: "Pace workout type" });
    const speed = group.getByRole("button", { name: "Speed", exact: true });
    const long = group.getByRole("button", { name: "Long", exact: true });
    await expect(speed).toHaveAttribute("aria-pressed", "true");
    await expect(long).toHaveAttribute("aria-pressed", "false");
    await page.mouse.move(0, 0);
    const dimensions = await group.boundingBox();
    const rim = control => control.evaluate(element => getComputedStyle(element, "::after").opacity);
    expect(await rim(speed)).toBe("0");
    await expect(speed.locator(".selection-marker")).toBeVisible();
    await expect(long.locator(".selection-marker")).toHaveCount(0);
    const material = control => control.evaluate(element => {
      const style = getComputedStyle(element);
      return { face: style.backgroundColor, ink: style.color, image: style.backgroundImage, shadow: style.boxShadow, border: style.borderColor, radius: style.borderRadius };
    });
    const selectedMaterial = await material(speed);
    const inactiveMaterial = await material(long);
    expect(selectedMaterial.face).not.toBe(inactiveMaterial.face);
    expect(selectedMaterial.image).toBe("none");
    expect(selectedMaterial.shadow).not.toContain("inset");
    expect(inactiveMaterial.shadow).toContain("inset");
    expect(inactiveMaterial.image).toContain("linear-gradient");
    expect(selectedMaterial.border).toBe("rgba(0, 0, 0, 0)");
    expect(inactiveMaterial.radius).toBe("999px");
    expect(selectedMaterial.ink).toBe(theme === "dark" ? "rgb(150, 196, 255)" : "rgb(41, 104, 186)");
    if (theme === "dark") {
      expect(selectedMaterial.face).toBe("rgb(57, 62, 75)");
      expect(inactiveMaterial.face).toBe("rgb(43, 46, 54)");
    }
    await group.screenshot({ path: testInfo.outputPath("speed-selected.png") });
    for (const [control, label] of [[speed, "speed"], [long, "long"]]) {
      await control.hover();
      await expect.poll(() => rim(control)).toBe("0.8");
      await expect(speed).toHaveAttribute("aria-pressed", "true");
      await expect(speed.locator(".selection-marker")).toBeVisible();
      await group.screenshot({ path: testInfo.outputPath(`${label}-hovered.png`) });
    }
    await long.click();
    await page.mouse.move(0, 0);
    await expect(long).toHaveAttribute("aria-pressed", "true");
    await expect(speed).toHaveAttribute("aria-pressed", "false");
    await expect(long.locator(".selection-marker")).toBeVisible();
    expect(await group.boundingBox()).toEqual(dimensions);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  });

  test(`reference buttons restore selected fill after touch ${theme}`, async ({ browser }, testInfo) => {
    const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, reducedMotion: "reduce" });
    const page = await context.newPage();
    await installDeskFixture(page);
    await page.goto(`/desk.html?theme=${theme}#health/pace`);
    const group = page.getByRole("group", { name: "Pace workout type" });
    const speed = group.getByRole("button", { name: "Speed", exact: true });
    const long = group.getByRole("button", { name: "Long", exact: true });
    await expect(speed).toHaveAttribute("aria-pressed", "true");
    const selectedFace = await speed.evaluate(element => getComputedStyle(element).backgroundColor);
    await group.scrollIntoViewIfNeeded();
    const bounds = await long.boundingBox();
    const touch = await context.newCDPSession(page);
    await touch.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: bounds.x + bounds.width / 2, y: bounds.y + bounds.height / 2 }] });
    await expect.poll(() => long.evaluate(element => getComputedStyle(element, "::after").opacity)).toBe("1");
    await group.screenshot({ path: testInfo.outputPath("long-touch-held.png") });
    await touch.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
    await expect(long).toHaveAttribute("aria-pressed", "true");
    await expect(long).toHaveCSS("background-color", selectedFace);
    await expect(long).toHaveCSS("background-image", "none");
    await expect.poll(() => long.evaluate(element => getComputedStyle(element, "::after").opacity)).toBe("0");
    await group.screenshot({ path: testInfo.outputPath("long-touch-selected.png") });
    await context.close();
  });

  test(`glass buttons retain material and keyboard feedback ${theme}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto(`/desk.html?theme=${theme}#settings/thresholds`);
    const control = page.getByRole("button", { name: "Restore defaults", exact: true });
    const alpha = () => control.evaluate(element => {
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 1;
      const context = canvas.getContext("2d");
      context.fillStyle = getComputedStyle(element).backgroundColor;
      context.fillRect(0, 0, 1, 1);
      return context.getImageData(0, 0, 1, 1).data[3] / 255;
    });
    await expect(control).toBeVisible();
    await page.mouse.move(0, 0);
    await expect.poll(alpha).toBe(1);
    await control.hover();
    await expect.poll(alpha).toBe(1);
    await expect(control).toHaveCSS("backdrop-filter", /blur\(12px\)/);
    const hoveredShadow = await control.evaluate(element => getComputedStyle(element).boxShadow);
    await page.mouse.down();
    await expect(control).toHaveCSS("scale", "0.98");
    await expect.poll(alpha).toBe(1);
    await expect(control).not.toHaveCSS("box-shadow", hoveredShadow);
    await page.mouse.move(0, 0);
    await page.mouse.up();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.keyboard.press("Tab");
    await control.focus();
    await expect(control).toHaveCSS("outline-style", "solid");
    await expect(control).toHaveCSS("transform", "none");
    await control.press("Enter");
    await expect(page.getByRole("dialog", { name: "Restore training defaults?" })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(control).toBeFocused();
    await page.emulateMedia({ contrast: "more" });
    await control.hover();
    await expect.poll(alpha).toBe(1);
    await expect(control).toHaveCSS("backdrop-filter", "none");
    await page.emulateMedia({ contrast: "no-preference" });
    await control.evaluate(element => element.blur());
    await page.mouse.move(0, 0);
    await page.screenshot({ path: testInfo.outputPath("glass-buttons.png") });
  });

  for (const width of [320, 390, 768, 1440]) test(`view controls share reference material and settings use restrained tilt ${theme} ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto(`/desk.html?theme=${theme}#health/pace`);
    await page.mouse.move(0, 0);
    const modes = page.getByRole("group", { name: "Pace workout type" });
    const selectedShadow = await modes.getByRole("button", { name: "Speed", exact: true }).evaluate(element => getComputedStyle(element).boxShadow);
    const inactiveShadow = await modes.getByRole("button", { name: "Long", exact: true }).evaluate(element => getComputedStyle(element).boxShadow);
    const families = [
      ["training", ".horizon-switch a, .desk-button, .mobile-date-ribbon button"],
      ...(width <= 800 ? [["training/month", ".month-number"]] : []),
      ["health", ".horizon-switch a, .desk-button"],
      ["journal/1", ".review-tabs button, .lap-row-button, .desk-button"],
      ["health/pace", ".pace-mode button, .pace-session-picker button, .desk-button"],
      ["settings/appearance", ".palette-options label, .choice-row label, .desk-button"],
      ["settings/thresholds", ".desk-button"],
    ];
    for (const [route, selector] of families) {
      await page.goto(`/desk.html?theme=${theme}#${route}`);
      await page.mouse.move(0, 0);
      const controls = page.locator(`${selector.split(", ").map(part => `${part}:visible:not([inert] *)`).join(", ")}`);
      await expect(controls.first()).toBeVisible();
      for (const control of await controls.all()) {
        if (await control.isDisabled()) continue;
        const selected = await control.evaluate(element => element.matches('.primary, [aria-current], [aria-selected="true"], [aria-pressed="true"], :has(:checked)'));
        await expect(control).toHaveCSS("box-shadow", selected ? selectedShadow : inactiveShadow);
        await expect(control).toHaveCSS("border-radius", "999px");
        await expect(control).toHaveCSS("backdrop-filter", "none");
        if (!selected) expect(await control.evaluate(element => getComputedStyle(element).backgroundImage)).toContain("linear-gradient");
      }
      await controls.first().hover();
      await expect(controls.first()).toHaveCSS("backdrop-filter", /blur\(12px\)/);
      await page.emulateMedia({ contrast: "more" });
      await expect(controls.first()).toHaveCSS("backdrop-filter", "none");
      await expect(controls.first()).toHaveCSS("box-shadow", "none");
      await page.emulateMedia({ contrast: "no-preference" });
      await page.mouse.move(0, 0);
      await page.screenshot({ path: testInfo.outputPath(`${route.replaceAll("/", "-")}-buttons.png`) });
    }
    if (width <= 800) return;
    await page.goto(`/desk.html?theme=${theme}#training`);
    await page.getByRole("link", { name: "Month", exact: true }).click();
    await expect(page.getByRole("link", { name: "Month", exact: true })).toHaveAttribute("aria-current");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: testInfo.outputPath("unified-toolbar.png") });
    await page.goto(`/desk.html?theme=${theme}#settings`);
    await page.emulateMedia({ reducedMotion: "no-preference" });
    const row = page.locator('.settings-task-list a[href="#settings/appearance"]');
    await expect(row).toBeVisible();
    await row.hover({ position: { x: 20, y: 20 } });
    await expect(row).toHaveAttribute("data-aura-tracking", "true");
    await expect(row).not.toHaveCSS("transform", "none");
    const angles = await row.evaluate(element => {
      const style = getComputedStyle(element);
      const strength = parseFloat(style.getPropertyValue("--surface-tilt-strength"));
      return ["--aura-tilt-x", "--aura-tilt-y"].map(name => Math.abs(parseFloat(style.getPropertyValue(name)) * strength));
    });
    expect(Math.max(...angles)).toBeGreaterThan(0);
    expect(Math.max(...angles)).toBeLessThanOrEqual(1.8);
    await expect(row).toHaveCSS("--surface-tilt-strength", "3");
    await expect(row).toHaveCSS("--surface-perspective", "1200px");
    await page.mouse.down();
    await expect.poll(() => row.evaluate(element => Math.abs(new DOMMatrix(getComputedStyle(element).transform).m13))).toBeLessThan(.05);
    await page.mouse.move(0, 0);
    await page.mouse.up();
    await expect(row).toHaveCSS("transform", "none");
    await page.emulateMedia({ reducedMotion: "reduce" });
    await row.hover();
    await expect(row).toHaveCSS("transform", "none");
    await page.screenshot({ path: testInfo.outputPath("settings-motion.png") });
  });

  test(`training cards visibly tilt with pointer ${theme}`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto(`/desk.html?theme=${theme}#training`);
    const card = page.locator(".session-tile").first();
    await page.mouse.move(0, 0);
    await page.emulateMedia({ reducedMotion: "reduce" });
    const originalFill = await card.evaluate(element => getComputedStyle(element).backgroundColor);
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await card.hover({ position: { x: 10, y: 10 } });
    await expect(card).toHaveCSS("background-color", originalFill);
    await expect(card).not.toHaveCSS("background-image", "none");
    await expect(card).toHaveAttribute("data-aura-tracking", "true");
    const pivot = await card.evaluate(element => {
      const style = getComputedStyle(element);
      const [horizontal, vertical] = style.transformOrigin.split(" ").map(parseFloat);
      return { horizontal: horizontal / element.offsetWidth, vertical: vertical / element.offsetHeight };
    });
    expect(pivot.horizontal).toBeCloseTo(.5, 2);
    expect(pivot.vertical).toBeCloseTo(.5, 2);
    expect(await card.evaluate(element => parseFloat(element.style.getPropertyValue("--aura-tilt-x")))).toBeGreaterThan(0);
    expect(await card.evaluate(element => parseFloat(element.style.getPropertyValue("--aura-tilt-y")))).toBeLessThan(0);
    await expect.poll(() => card.evaluate(element => {
      const style = getComputedStyle(element);
      return Math.abs(parseFloat(style.getPropertyValue("--aura-tilt-x")) * parseFloat(style.getPropertyValue("--surface-tilt-strength")));
    })).toBeGreaterThan(6);
    await expect(card).not.toHaveCSS("transform", "none");
    const hoverDepth = await card.evaluate(element => Math.abs(new DOMMatrix(getComputedStyle(element).transform).m23));
    await page.mouse.down();
    await expect.poll(() => card.evaluate(element => Math.abs(new DOMMatrix(getComputedStyle(element).transform).m23))).toBeGreaterThan(hoverDepth * 1.1);
    await page.mouse.move(0, 0);
    await page.mouse.up();
    await page.mouse.move(0, 0);
    await expect(card).toHaveCSS("transform", "none");
  });

  test(`dynamic aura tracks pointer and click origin ${theme}`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto(`/desk.html?theme=${theme}#journal`);
    const control = page.locator(".activity-row").first();
    await expect(control).toBeVisible();
    await expect(control).toHaveCSS("--surface-tilt-strength", "3");
    await expect(control).toHaveCSS("--surface-perspective", "1200px");
    await control.hover({ position: { x: 40, y: 40 } });
    await expect(control).toHaveAttribute("data-aura-tracking", "true");
    const first = await control.evaluate(element => element.style.getPropertyValue("--aura-x"));
    await expect(control).not.toHaveCSS("transform", "none");
    const bounds = await control.boundingBox();
    await page.mouse.move(bounds.x + bounds.width - 40, bounds.y + 40);
    await expect.poll(() => control.evaluate(element => parseFloat(element.style.getPropertyValue("--aura-x")))).toBeGreaterThan(parseFloat(first));
    const tilt = await control.evaluate(element => ({ horizontal: parseFloat(element.style.getPropertyValue("--aura-tilt-y")), vertical: parseFloat(element.style.getPropertyValue("--aura-tilt-x")) }));
    expect(tilt.horizontal).toBeGreaterThan(0);
    expect(Math.abs(tilt.horizontal)).toBeLessThanOrEqual(.6);
    expect(Math.abs(tilt.vertical)).toBeLessThanOrEqual(.6);
    await expect.poll(() => control.evaluate(element => {
      const bounds = element.getBoundingClientRect();
      const container = element.closest(".activity-library").getBoundingClientRect();
      return Math.min(bounds.left - container.left, container.right - bounds.right, bounds.top - container.top);
    })).toBeGreaterThanOrEqual(4);
    const hoverTransform = await control.evaluate(element => getComputedStyle(element).transform);
    await page.mouse.down();
    await expect(control).not.toHaveCSS("transform", hoverTransform);
    await expect.poll(() => control.evaluate(element => element.getAnimations().some(animation => animation.effect?.getKeyframes().some(keyframe => keyframe.backgroundSize === "155% 155%")))).toBe(true);
    await page.mouse.move(0, 0);
    await page.mouse.up();
    await expect(control).not.toHaveAttribute("data-aura-tracking");
    await expect(control).toHaveCSS("transform", "none");
    await expect.poll(() => control.evaluate(element => element.getAnimations().length)).toBe(0);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await control.hover();
    await expect(control).toHaveCSS("transform", "none");
    await page.mouse.down();
    expect(await control.evaluate(element => element.getAnimations().length)).toBe(0);
    await page.mouse.move(0, 0);
    await page.mouse.up();
  });

  test(`shared aura touch release stays flat ${theme}`, async ({ browser }, testInfo) => {
    const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, reducedMotion: "reduce" });
    const page = await context.newPage();
    await installDeskFixture(page);
    await page.goto(`/desk.html?theme=${theme}#journal`);
    const control = page.locator(".activity-row").first();
    await expect(control).toBeVisible();
    const navigation = page.getByRole("link", { name: "Activities", exact: true });
    const bounds = await navigation.boundingBox();
    const touch = await context.newCDPSession(page);
    await touch.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: bounds.x + 8, y: bounds.y + bounds.height / 2 }] });
    await expect.poll(() => navigation.evaluate(element => getComputedStyle(element, "::after").opacity)).toBe("1");
    expect(await navigation.evaluate(element => parseFloat(element.style.getPropertyValue("--aura-x")))).toBeLessThan(25);
    await page.screenshot({ path: testInfo.outputPath("glass-rim-touch.png") });
    await touch.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
    await expect.poll(() => navigation.evaluate(element => getComputedStyle(element, "::after").opacity)).toBe("0");
    await expect(navigation).toHaveAttribute("aria-current", "page");
    await expect(navigation).toHaveCSS("--aura-strength", "0%");
    await expect(control).toHaveCSS("--aura-strength", "0%");
    await expect(control).toHaveCSS("box-shadow", "none");
    await context.close();
  });

  test(`shared flat surfaces engage with aura ${theme}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(`/desk.html?theme=${theme}#journal`);
    const toolbar = page.locator(".journal-sidebar > .desk-toolbar");
    await expect(toolbar).toBeVisible();
    await expect(toolbar).toHaveCSS("background-image", /linear-gradient/);
    await expect(toolbar).toHaveCSS("box-shadow", "none");
    const controls = [page.locator('.desk-navigation a[href="#journal"]'), toolbar.locator("button").first(), page.locator(".activity-row").first()];
    for (const control of controls) {
      await page.mouse.move(0, 0);
      await expect(control).toHaveCSS("--aura-strength", "0%");
      if (await control.evaluate(element => element.matches(".desk-button, .desk-navigation a"))) {
        const selected = await control.evaluate(element => element.matches('[aria-current], [aria-pressed="true"]'));
        if (selected) expect(await control.evaluate(element => getComputedStyle(element).boxShadow)).not.toContain("inset");
        else await expect(control).toHaveCSS("box-shadow", /inset/);
        await expect(control).toHaveCSS("backdrop-filter", "none");
        if (!selected) expect(await control.evaluate(element => getComputedStyle(element).backgroundImage)).toContain("linear-gradient");
      } else {
        await expect(control).toHaveCSS("box-shadow", "none");
      }
      await control.hover();
      await expect(control).not.toHaveCSS("background-image", "none");
      await expect(control).not.toHaveCSS("box-shadow", "none");
      if (await control.evaluate(element => element.matches(".desk-button"))) await expect(control).toHaveCSS("backdrop-filter", /blur\(12px\)/);
      await expect(control).toHaveCSS("transform", "none");
      await page.mouse.down();
      await expect(control).toHaveCSS("box-shadow", /inset/);
      await page.mouse.move(0, 0);
      await page.mouse.up();
      await page.keyboard.press("Tab");
      await control.focus();
      await expect(control).toHaveCSS("outline-style", "solid");
      await expect(control).not.toHaveCSS("background-image", "none");
      await control.evaluate(element => element.blur());
    }
    await expect(toolbar).toHaveCSS("background-image", /linear-gradient/);
    await page.locator(".activity-row").first().hover();
    await page.screenshot({ path: testInfo.outputPath("shared-aura.png") });
    await page.emulateMedia({ contrast: "more" });
    await expect(page.locator(".activity-row").first()).toHaveCSS("background-image", "none");
    await expect(toolbar.locator("button").first()).toHaveCSS("backdrop-filter", "none");
    await expect(toolbar.locator("button").first()).toHaveCSS("box-shadow", "none");
  });
}

test("shared aura color fades on hover and departure", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/desk.html#journal");
  const control = page.locator(".activity-row").first();
  await page.mouse.move(0, 0);
  await expect(control).toHaveCSS("--aura-strength", "0%");
  await control.hover();
  await expect.poll(() => control.evaluate(element => element.getAnimations().some(animation => animation instanceof CSSTransition && animation.transitionProperty === "--aura-strength"))).toBe(true);
  await expect(control).toHaveCSS("--aura-strength", "100%");
  await page.mouse.move(0, 0);
  await expect.poll(() => control.evaluate(element => element.getAnimations().some(animation => animation instanceof CSSTransition && animation.transitionProperty === "--aura-strength"))).toBe(true);
  await expect(control).toHaveCSS("--aura-strength", "0%");
});

for (const theme of ["light", "dark"]) {
  test(`multicolor palettes stay distinct and persist ${theme}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: theme === "light" ? 320 : 1440, height: 900 });
    await page.goto(`/desk.html?theme=${theme}#settings/appearance`);
    const options = page.locator('.palette-options input[type="radio"]');
    await expect(options).toHaveCount(20);
    await expect(page.getByRole("radio", { name: "Dashing Blueberry", exact: true })).toHaveCount(0);
    await expect(page.getByRole("radio", { name: "The Coastal Chaos", exact: true })).toHaveCount(0);
    const signatures = new Set();
    for (const [palette, name] of [["carnival", "Raspberry Relay"], ["electric", "Electric Stride"], ["sorbet", "Sorbet Sprint"], ["regatta", "Regatta Ready"], ["meadow", "Meadow Medley"]]) {
      const radio = page.getByRole("radio", { name, exact: true });
      await expect(radio).toBeEnabled();
      await radio.check();
      await expect(radio).toBeChecked();
      await expect(page.locator("html")).toHaveAttribute("data-desk-palette", palette);
      await expect.poll(() => page.evaluate(() => localStorage.getItem("runalytics.palette"))).toBe(palette);
      const colors = await page.evaluate(() => {
        const style = getComputedStyle(document.documentElement);
        const tokens = ["--desk-action", "--desk-blue", "--session-tempo"];
        return tokens.map(token => style.getPropertyValue(token).trim());
      });
      expect(new Set(colors).size).toBe(3);
      signatures.add([...colors].sort().join("/"));
    }
    expect(signatures.size).toBe(5);
    await expect.poll(() => page.evaluate(async () => (await window.__TAURI_INTERNALS__.invoke("get_settings")).palette)).toBe("meadow");
    await page.getByRole("button", { name: "All settings", exact: true }).click();
    await page.locator('a[href="#settings/appearance"]').click();
    await expect(page.getByRole("radio", { name: "Meadow Medley", exact: true })).toBeChecked();
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
    await expect(page.getByRole("radio", { name: "Meadow Medley", exact: true })).toBeEnabled();
    const accessibility = await new AxeBuilder({ page }).include(".settings-editor").analyze();
    expect(accessibility.violations).toEqual([]);
    await page.screenshot({ path: testInfo.outputPath(`multicolor-${theme}.png`), fullPage: true });
  });
}

for (const [palette, name] of [["garnet", "Blue Steel Sprint"], ["coastline", "Sunrise Shenanigans"]]) {
  test(`retired palette ${palette} retains a selected choice`, async ({ page }) => {
    await page.goto(`/desk.html?palette=${palette}#settings/appearance`);
    await expect(page.getByRole("radio", { name, exact: true })).toBeChecked();
    await expect(page.locator('.palette-options input:checked')).toHaveCount(1);
  });
}

for (const theme of ["light", "dark", "system"]) {
  test(`macOS title bar follows the palette ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(() => {
      document.addEventListener("DOMContentLoaded", () => {
        document.documentElement.dataset.nativeWindow = "macos";
        window.__TAURI_INTERNALS__.metadata = { currentWindow: { label: "main" }, currentWebview: { label: "main" } };
        const invoke = window.__TAURI_INTERNALS__.invoke;
        window.__nativeThemes = [];
        window.__TAURI_INTERNALS__.invoke = (command, args) => {
          if (command === "plugin:window|set_theme") {
            window.__nativeThemes.push(args.value);
            return Promise.resolve();
          }
          return invoke(command, args);
        };
      });
    });
    await page.setViewportSize({ width: 1180, height: 780 });
    await page.goto(`/desk.html?theme=${theme}#health/pace`);
    await expect(page.locator(".adjustment-status")).toHaveText("No pending adjustments");
    await expect.poll(() => page.evaluate(() => window.__nativeThemes.at(-1))).toBe(theme === "system" ? null : theme);
    const title = page.locator(".native-window-title");
    await expect(title).toBeVisible();
    await expect(title).toHaveCSS("color", theme === "dark" ? "rgb(255, 255, 255)" : "rgb(0, 0, 0)");
    await expect(title).toHaveCSS("left", "104px");
    const inspect = () => page.evaluate(() => {
      const root = getComputedStyle(document.documentElement);
      const shell = getComputedStyle(document.getElementById("desk-root"));
      const body = getComputedStyle(document.body);
      return {
        inset: parseFloat(shell.paddingTop) - parseFloat(shell.paddingBottom),
        background: body.backgroundImage,
        accent: root.getPropertyValue("--desk-action").trim(),
        overflow: document.documentElement.scrollWidth > innerWidth,
        workspaceTop: document.getElementById("workspace").getBoundingClientRect().top,
      };
    });
    const initial = await inspect();
    expect(initial.inset).toBe(32);
    expect(initial.workspaceTop).toBeGreaterThanOrEqual(32);
    expect(initial.overflow).toBe(false);
    const pageBackground = await page.evaluate(() => {
      delete document.documentElement.dataset.nativeWindow;
      const background = getComputedStyle(document.body).backgroundImage;
      document.documentElement.dataset.nativeWindow = "macos";
      return background;
    });
    expect(initial.background).toBe(pageBackground);
    await page.evaluate(() => { document.documentElement.dataset.deskPalette = "sakura"; });
    const changed = await inspect();
    expect(changed.accent).not.toBe(initial.accent);
    expect(changed.background).not.toBe(initial.background);
    expect(changed.inset).toBe(32);
    expect(changed.overflow).toBe(false);
    if (theme === "system") {
      const commands = await page.evaluate(() => window.__nativeThemes.length);
      await page.emulateMedia({ colorScheme: "dark" });
      await expect(page.locator("html")).toHaveAttribute("data-desk-theme", "dark");
      await expect(title).toHaveCSS("color", "rgb(255, 255, 255)");
      await page.emulateMedia({ colorScheme: "light" });
      await expect(page.locator("html")).toHaveAttribute("data-desk-theme", "light");
      expect(await page.evaluate(() => window.__nativeThemes.length)).toBe(commands);
      await page.evaluate(() => { location.hash = "settings/appearance"; });
      for (const selection of ["Light", "Dark", "System", "Dark", "Light", "System"]) {
        const radio = page.getByRole("radio", { name: selection, exact: true });
        await expect(radio).toBeEnabled();
        await radio.check();
        await expect(radio).toBeChecked();
        await expect.poll(() => page.evaluate(() => window.__nativeThemes.at(-1))).toBe(selection === "System" ? null : selection.toLowerCase());
      }
      await expect(page.locator("html")).toHaveAttribute("data-desk-theme", "light");
    }
    await page.screenshot({ path: testInfo.outputPath(`macos-header-${theme}.png`) });
  });
}

for (const theme of ["light", "dark"]) {
  for (const width of [320, 390, 768, 1440, 2560]) {
    test(`Prism overlay controls and corner alignment ${theme} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(`/desk.html?theme=${theme}#health/pace`);
      const status = page.locator(".adjustment-status");
      await expect(status).toHaveText("No pending adjustments");
      await expect(status).toHaveCSS("font-size", "13px");
      const opener = page.getByRole("button", { name: "About pace insights", exact: true });
      await opener.click();
      const dialog = page.getByRole("dialog", { name: "About pace insights", exact: true });
      const close = dialog.getByRole("button", { name: "Close About pace insights", exact: true });
      await expect(dialog.getByRole("heading", { name: "About pace insights", exact: true })).toBeFocused();
      await expect(close.locator(".control-tooltip")).toHaveCSS("visibility", "hidden");
      await page.keyboard.press("Tab");
      await expect(close).toBeFocused();
      await page.keyboard.press("Escape");
      await expect(dialog).toHaveCount(0);
      await expect(opener).toBeFocused();
      await page.goto(`/desk.html?theme=${theme}#health/journal`);
      await page.getByRole("button", { name: "Log pain", exact: true }).click();
      await expect(page.getByRole("dialog", { name: "Log pain" }).getByLabel("Body region")).toBeFocused();
      await page.keyboard.press("Escape");
      await page.goto(`/desk.html?theme=${theme}#health/pace`);
      await expect(opener).toBeVisible();
      const showNotice = () => page.evaluate(() => window.__deskEmit("plan://stage", { op: "alignment", target: "coros", state: "working" }));
      await showNotice();
      const notice = page.locator(".desk-stages");
      await expect(notice).toBeVisible();
      const checkCorners = async () => {
        const workspace = await page.locator("#workspace").boundingBox();
        const bounds = await notice.boundingBox();
        expect(bounds.x).toBeGreaterThanOrEqual(0);
        expect(bounds.x + bounds.width).toBeLessThanOrEqual(width);
        if (width > 800) {
          expect(workspace.x + workspace.width - bounds.x - bounds.width).toBeCloseTo(24, 0);
          expect(workspace.y + workspace.height - bounds.y - bounds.height).toBeCloseTo(24, 0);
        }
      };
      await checkCorners();
      await page.screenshot({ path: testInfo.outputPath("notification.png") });
      await opener.click();
      await expect(dialog).toBeVisible();
      await expect(notice).toBeVisible();
      await checkCorners();
      await page.screenshot({ path: testInfo.outputPath("dialog.png") });
      await page.getByRole("button", { name: "Dismiss notification", exact: true }).click();
      await expect(notice).toHaveCount(0);
      await expect(dialog.getByRole("heading", { name: "About pace insights", exact: true })).toBeFocused();
      await close.click();
      await expect(opener).toBeFocused();
      await page.goto(`/desk.html?theme=${theme}#training`);
      if (width <= 800) await page.locator('.mobile-date-ribbon button:has(.has-session)').first().click();
      const session = page.locator(".session-tile:visible").first();
      await session.click();
      const inspector = page.getByRole("dialog", { name: "Session", exact: true });
      await expect(inspector).toBeFocused();
      const dismiss = inspector.getByRole("button", { name: "Close Session", exact: true });
      await expect(dismiss).toHaveClass(/icon-button/);
      await expect(inspector.getByRole("button", { name: "Back", exact: true })).toHaveCount(0);
      await page.screenshot({ path: testInfo.outputPath("inspector.png") });
      await dismiss.click();
      await expect(session).toBeFocused();
    });
  }
}

for (const theme of ["light", "dark"]) {
  for (const width of [320, 390, 768, 1440]) {
    test(`Prism Health tabs and unified Pace layout ${theme} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 844 });
      await page.goto(`/desk.html?theme=${theme}#health`);
      const tabs = page.getByRole("navigation", { name: "Health view" });
      if (width <= 390) {
        await expect.poll(() => tabs.evaluate(element => element.scrollWidth - element.clientWidth)).toBeGreaterThan(0);
        await tabs.evaluate(element => { element.scrollLeft = element.scrollWidth; });
        await expect.poll(() => tabs.evaluate(element => element.scrollLeft)).toBeGreaterThan(0);
        await expect(tabs.getByRole("link", { name: "Milestones", exact: true })).toBeInViewport();
      }
      await tabs.getByRole("link", { name: "Pace insights", exact: true }).focus();
      await page.keyboard.press("Enter");
      const pace = page.getByRole("region", { name: "Pace insights", exact: true });
      await expect(pace).toBeVisible();
      await expect(pace).toHaveCSS("row-gap", "20px");
      await expect(pace.locator(".pace-mode")).toHaveCSS("margin-bottom", "0px");
      await expect(pace.locator(".pace-session-picker")).toHaveCSS("margin-top", "0px");
      await expect(pace.locator(".pace-session-picker")).toHaveCSS("overflow-x", "auto");
      const chart = await pace.locator(".pace-insights-figure svg").boundingBox();
      expect(chart.height).toBeLessThanOrEqual(240);
      const detail = pace.getByRole("region", { name: "Selected pace session" });
      await expect(detail.getByRole("heading", { level: 3 })).toBeVisible();
      await expect(detail.getByRole("heading", { level: 3 })).toHaveCSS("font-size", "18px");
      await expect(pace.locator(".pace-history-heading h3")).toHaveCSS("font-size", "18px");
      await assertLayout(page, `Pace ${theme} ${width}`);
      await page.screenshot({ path: testInfo.outputPath("pace-overview.png") });
      await detail.scrollIntoViewIfNeeded();
      await page.screenshot({ path: testInfo.outputPath("pace-session.png") });
    });
  }
}

for (const theme of ["light", "dark"]) {
  test(`activity inspector Back hover remains unclipped ${theme}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto(`/desk.html?theme=${theme}#journal/1`);
    const inspector = page.locator(".journal-workspace .desk-inspector");
    const back = inspector.getByRole("button", { name: "Back", exact: true });
    await expect(back).toBeVisible();
    await expect(page.locator(".desk-loading")).toHaveCount(0);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await inspector.evaluate(element => { element.scrollTop = 0; });
    await back.hover();
    await expect(back.locator(".control-tooltip")).toBeVisible();
    await expect.poll(() => back.evaluate(element => {
      const button = element.getBoundingClientRect();
      const surface = element.closest(".desk-inspector").getBoundingClientRect();
      const tooltip = element.querySelector(".control-tooltip").getBoundingClientRect();
      return Math.min(button.top - surface.top, button.left - surface.left, tooltip.left - surface.left);
    })).toBeGreaterThanOrEqual(4);
    await page.keyboard.press("Tab");
    await back.focus();
    await expect(back).toHaveCSS("outline-style", "solid");
    await inspector.screenshot({ path: testInfo.outputPath("activity-back-hover.png") });
  });

  test(`Prism scroll controls preserve hover clearance ${theme}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    for (const route of ["training", "health", "health/pace", "journal/1"]) {
      await page.goto(`/desk.html?theme=${theme}#${route}`);
      const selector = route === "journal/1" ? ".review-tabs" : route === "health/pace" ? ".pace-session-picker" : ".horizon-switch";
      const strip = page.locator(selector).first();
      await expect(strip).toBeVisible();
      await expect(page.locator(".desk-loading")).toHaveCount(0);
      const controls = strip.locator(":scope > a, :scope > button");
      for (const control of [controls.first(), controls.last()]) {
        await control.hover();
        await control.focus();
        await expect.poll(() => control.evaluate(element => {
          const bounds = element.getBoundingClientRect();
          const viewport = element.parentElement.getBoundingClientRect();
          return Math.min(bounds.top - viewport.top, viewport.bottom - bounds.bottom, bounds.left - viewport.left, viewport.right - bounds.right);
        })).toBeGreaterThanOrEqual(3);
        await expect(control).toHaveCSS("outline-style", "solid");
      }
      await strip.screenshot({ path: testInfo.outputPath(`${route.replaceAll("/", "-")}-hover.png`) });
    }
  });
}

for (const theme of ["light", "dark"]) {
  test(`Prism horizontal transparent scroll fades ${theme}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(`/desk.html?theme=${theme}#journal/1`);
    const tabs = page.getByRole("tablist", { name: "Activity detail" });
    await expect(tabs).toBeVisible();
    await expect(tabs).toHaveAttribute("data-scroll-right", "1");
    await expect(tabs).toHaveCSS("background-image", "none");
    await expect(tabs).not.toHaveCSS("mask-image", "none");
    await expect.poll(() => tabs.evaluate(element => getComputedStyle(element).getPropertyValue("--scroll-left-opacity").trim())).toBe("0");
    await tabs.evaluate(element => { element.scrollLeft = (element.scrollWidth - element.clientWidth) / 2; });
    await expect(tabs).toHaveAttribute("data-scroll-left", "1");
    await expect(tabs).toHaveAttribute("data-scroll-right", "1");
    await tabs.screenshot({ path: testInfo.outputPath("tabs-transparent-fades.png") });
    const context = tabs.getByRole("tab", { name: "Training context", exact: true });
    await context.focus();
    await page.keyboard.press("Enter");
    await expect(context).toHaveAttribute("aria-selected", "true");
    await tabs.evaluate(element => { element.scrollLeft = element.scrollWidth; });
    await expect(tabs).toHaveAttribute("data-scroll-right", "");
    await expect.poll(() => tabs.evaluate(element => getComputedStyle(element).getPropertyValue("--scroll-right-opacity").trim())).toBe("0");
    await page.emulateMedia({ contrast: "more", reducedMotion: "reduce" });
    await expect(tabs).toHaveCSS("mask-image", "none");
    await page.emulateMedia({ contrast: "no-preference", reducedMotion: "reduce" });
    await page.goto(`/desk.html?theme=${theme}#training/plan`);
    await page.getByRole("button", { name: "Show details", exact: true }).click();
    const table = page.getByRole("region", { name: "Plan basis", exact: true });
    await table.locator("table").evaluate(element => { element.style.minWidth = "800px"; });
    await page.setViewportSize({ width: 375, height: 844 });
    await expect(table).toHaveAttribute("data-scroll-right", "1");
    await expect(table).toHaveCSS("background-image", "none");
    await expect(table).not.toHaveCSS("mask-image", "none");
    await table.evaluate(element => { element.scrollLeft = element.scrollWidth; });
    await expect(table).toHaveAttribute("data-scroll-left", "1");
    await expect(table).toHaveAttribute("data-scroll-right", "");
  });
}

for (const theme of ["light", "dark"]) {
  for (const viewport of [{ width: 320, height: 640 }, { width: 390, height: 844 }, { width: 768, height: 844 }, { width: 740, height: 390 }]) {
    test(`Prism contained mobile shell ${theme} ${viewport.width}x${viewport.height}`, async ({ page }, testInfo) => {
      await page.setViewportSize(viewport);
      await page.emulateMedia({ reducedMotion: "reduce" });
      await page.goto(`/desk.html?theme=${theme}#journal`);
      const workspace = page.locator("#workspace");
      const header = page.locator(".desk-appbar");
      const navigation = page.getByRole("navigation", { name: "Workspaces" });
      await expect(page.locator(".activity-row")).toHaveCount(9);
      const frame = await workspace.boundingBox();
      const island = await navigation.boundingBox();
      expect(frame.y + frame.height - island.y - island.height).toBeGreaterThanOrEqual(4);
      expect(island.x - frame.x).toBeGreaterThanOrEqual(8);
      expect(frame.x + frame.width - island.x - island.width).toBeGreaterThanOrEqual(8);
      const brand = await header.locator(".desk-brand").boundingBox();
      await page.getByRole("button", { name: "Filters", exact: true }).click();
      const filters = page.locator(".journal-tools");
      await expect(filters).toHaveCSS("display", "flex");
      const reset = page.getByRole("button", { name: "Reset filters", exact: true });
      await reset.scrollIntoViewIfNeeded();
      const fields = await filters.locator("input, select").evaluateAll(elements => elements.map(element => element.getBoundingClientRect().toJSON()));
      const resetBounds = await reset.boundingBox();
      for (const field of fields) {
        const horizontal = Math.max(field.left - resetBounds.x - resetBounds.width, resetBounds.x - field.right);
        const vertical = Math.max(field.top - resetBounds.y - resetBounds.height, resetBounds.y - field.bottom);
        expect(Math.max(horizontal, vertical)).toBeGreaterThanOrEqual(8);
      }
      await workspace.evaluate(element => { element.scrollTop = element.scrollHeight; });
      await expect.poll(() => workspace.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
      expect(await workspace.boundingBox()).toEqual(frame);
      expect(await navigation.boundingBox()).toEqual(island);
      expect(await header.locator(".desk-brand").boundingBox()).toEqual(brand);
      await expect(header.locator(".desk-brand")).toBeVisible();
      expect(await page.evaluate(() => ({ x: scrollX, y: scrollY, overflow: document.documentElement.scrollWidth > innerWidth }))).toEqual({ x: 0, y: 0, overflow: false });
      await page.screenshot({ path: testInfo.outputPath("scrolled-shell.png") });
      await workspace.evaluate(element => { element.scrollTop = 0; });
      await page.screenshot({ path: testInfo.outputPath("filter-spacing.png") });
      await page.goto(`/desk.html?theme=${theme}#health/journal`);
      const logPain = page.getByRole("button", { name: "Log pain", exact: true });
      await logPain.click();
      const dialog = page.getByRole("dialog", { name: "Log pain", exact: true });
      await expect(dialog.locator(".form-actions")).toHaveCSS("gap", "12px");
      const cancel = await dialog.getByRole("button", { name: "Cancel", exact: true }).boundingBox();
      const save = await dialog.getByRole("button", { name: "Save pain entry", exact: true }).boundingBox();
      expect(Math.max(save.x - cancel.x - cancel.width, save.y - cancel.y - cancel.height)).toBeGreaterThanOrEqual(12);
      await page.keyboard.press("Escape");
      await expect(logPain).toBeFocused();
    });
  }
}

for (const theme of ["light", "dark"]) {
  for (const width of [320, 390, 768, 820, 1440]) {
    test(`Prism text clipping audit ${theme} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 844 });
      await page.emulateMedia({ reducedMotion: "reduce" });
      for (const route of ["training", "training/month", "training/plan", "journal", "journal/1", "health", "health/pace", "health/journal", "health/milestones", "settings", "settings/appearance", "settings/calendar", "help"]) {
        await page.goto(`/desk.html?theme=${theme}#${route}`);
        await expect(page.locator("#workspace h1")).toBeVisible();
        await expect(page.locator(".desk-loading")).toHaveCount(0);
        if (route === "journal/1") await expect(page.getByRole("region", { name: "Recorded GPS route" })).toBeVisible();
        await page.evaluate(() => document.fonts.ready);
        const clipped = () => page.locator("#desk-root").evaluate(root => {
          const failures = [];
          const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
          while (walker.nextNode()) {
            const node = walker.currentNode;
            const element = node.parentElement;
            if (!node.textContent.trim() || !element || element.closest(".sr-only, svg, .control-tooltip, .maplibregl-map")) continue;
            if (!element.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })) continue;
            if (getComputedStyle(element).clipPath !== "none" || getComputedStyle(element).clip !== "auto") continue;
            const range = document.createRange();
            range.selectNodeContents(node);
            for (const rect of range.getClientRects()) {
              if (rect.width < 1 || rect.height < 1) continue;
              let ancestor = element;
              while (ancestor && ancestor !== root) {
                const style = getComputedStyle(ancestor);
                const bounds = ancestor.getBoundingClientRect();
                if (style.clipPath !== "none" || style.clip !== "auto") break;
                if ([style.overflowX, style.overflowY].some(value => value === "auto" || value === "scroll")) break;
                if (["hidden", "clip"].includes(style.overflowX) && (rect.left < bounds.left - 1 || rect.right > bounds.right + 1)) {
                  failures.push({ text: node.textContent.trim(), container: ancestor.className, axis: "horizontal" });
                  break;
                }
                if (["hidden", "clip"].includes(style.overflowY) && (rect.top < bounds.top - 1 || rect.bottom > bounds.bottom + 1)) {
                  failures.push({ text: node.textContent.trim(), container: ancestor.className, axis: "vertical" });
                  break;
                }
                ancestor = ancestor.parentElement;
              }
            }
          }
          return failures;
        });
        await expect.poll(clipped, { message: `${route} ${theme} ${width}` }).toEqual([]);
        await assertLayout(page, `text audit ${route} ${theme} ${width}`);
        if (route === "journal") {
          await expect(page.locator(".activity-library")).toHaveCSS("border-radius", "0px");
          await expect(page.locator(".activity-purpose").first()).toHaveCSS("-webkit-line-clamp", "none");
          await page.screenshot({ path: testInfo.outputPath("journal-text.png") });
        }
      }
    });
  }
}

for (const reducedMotion of ["reduce", "no-preference"]) {
  test(`Prism compact navigation island responds to input ${reducedMotion}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.emulateMedia({ reducedMotion });
    await page.goto("/desk.html#training");
    const navigation = page.getByRole("navigation", { name: "Workspaces" });
    const before = await navigation.boundingBox();
    expect(before.width).toBeLessThan(244);
    const training = navigation.getByRole("link", { name: "Training", exact: true });
    const health = navigation.getByRole("link", { name: "Health", exact: true });
    const activities = navigation.getByRole("link", { name: "Activities", exact: true });
    await expect(navigation.locator('[tabindex="0"]')).toHaveCount(0);
    for (const link of [training, activities, health]) {
      await expect(link).toHaveAttribute("data-reveal", "settled");
      const bounds = await link.boundingBox();
      expect(bounds.width).toBeGreaterThanOrEqual(44);
      expect(bounds.width).toBe(64);
      expect(bounds.height).toBeGreaterThanOrEqual(44);
    }
    await health.focus();
    await expect(health).toHaveCSS("outline-style", "solid");
    await navigation.evaluate(element => {
      window.__navigationPositions = [];
      window.__navigationObserver = new MutationObserver(() => {
        const marker = element.querySelector(".selection-marker");
        if (marker) window.__navigationPositions.push(marker.getBoundingClientRect().x);
      });
      window.__navigationObserver.observe(element, { subtree: true, attributes: true, childList: true });
    });
    await page.keyboard.press("Enter");
    await expect(health).toHaveAttribute("aria-current", "page");
    await expect.poll(() => health.locator(".selection-marker").evaluate(element => {
      const marker = element.getBoundingClientRect();
      const link = element.parentElement.getBoundingClientRect();
      const border = parseFloat(getComputedStyle(element.parentElement).borderLeftWidth);
      return Math.abs(marker.x - link.x - border) + Math.abs(marker.width - link.width + 2 * border);
    })).toBeLessThan(1);
    if (reducedMotion === "no-preference") {
      const origin = await training.boundingBox();
      const target = await health.boundingBox();
      expect(await page.evaluate(({ start, end }) => window.__navigationPositions.some(position => position > start + 8 && position < end - 8), { start: origin.x, end: target.x })).toBe(true);
    }
    await page.evaluate(() => window.__navigationObserver.disconnect());
    const center = await activities.boundingBox();
    await page.mouse.move(center.x + center.width / 2, center.y + center.height / 2);
    await page.mouse.down();
    if (reducedMotion === "no-preference") await expect.poll(() => activities.evaluate(element => parseFloat(getComputedStyle(element).scale))).toBeLessThan(1);
    await page.mouse.up();
    await expect(activities).toHaveAttribute("aria-current", "page");
    await health.click();
    await training.click();
    await expect(training).toHaveAttribute("aria-current", "page");
    await expect.poll(() => training.locator(".selection-marker").evaluate(element => {
      const marker = element.getBoundingClientRect();
      const link = element.parentElement.getBoundingClientRect();
      const border = parseFloat(getComputedStyle(element.parentElement).borderLeftWidth);
      return Math.abs(marker.x - link.x - border);
    })).toBeLessThan(1);
    await page.mouse.move(10, 10);
    await expect.poll(async () => (await navigation.boundingBox()).width).toBe(before.width);
    if (reducedMotion === "reduce") await expect(training.locator(".navigation-icon")).toHaveCSS("transform", "none");
    const session = await page.context().newCDPSession(page);
    await session.send("Emulation.setTouchEmulationEnabled", { enabled: true });
    const target = await health.boundingBox();
    const touchPoints = [{ x: target.x + target.width / 2, y: target.y + target.height / 2 }];
    await session.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints });
    await expect(training).toHaveAttribute("aria-current", "page");
    await session.send("Input.dispatchTouchEvent", { type: "touchCancel", touchPoints: [] });
    await expect(training).toHaveAttribute("aria-current", "page");
    await session.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints });
    await session.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
    await expect(health).toHaveAttribute("aria-current", "page");
    await session.detach();
    await expect.poll(() => health.locator(".navigation-icon").evaluate(element => getComputedStyle(element).transform)).toBe(reducedMotion === "reduce" ? "none" : "matrix(1.08, 0, 0, 1.08, 0, -2)");
    await page.screenshot({ path: testInfo.outputPath("compact-navigation.png") });
  });
}

for (const theme of ["light", "dark"]) {
  for (const width of [320, 390, 768]) {
    test(`mobile controls share raised depth ${theme} ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 844 });
      await page.emulateMedia({ reducedMotion: "reduce" });
      await page.goto(`/desk.html?theme=${theme}#training`);
      const action = page.getByRole("button", { name: "Sync activities", exact: true });
      const horizon = page.locator(".horizon-switch a").first();
      await expect(action).toBeVisible();
      await expect(horizon).toBeVisible();
      const shadow = await action.evaluate(element => getComputedStyle(element).boxShadow);
      expect(shadow).not.toBe("none");
      await expect(horizon).toHaveCSS("box-shadow", shadow);
      await action.evaluate(element => { element.disabled = true; });
      await expect(action).toHaveCSS("box-shadow", "none");
      await action.evaluate(element => { element.disabled = false; });
      await page.emulateMedia({ contrast: "more" });
      await expect(action).toHaveCSS("box-shadow", "none");
      await expect(horizon).toHaveCSS("box-shadow", "none");
    });

    test(`Prism flat mobile surfaces ${theme} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 844 });
      await page.emulateMedia({ reducedMotion: "reduce" });
      await page.goto(`/desk.html?theme=${theme}#journal`);
      const rows = page.locator(".activity-row");
      await expect(rows).toHaveCount(9);
      await expect(page.locator(".journal-sidebar > .desk-toolbar")).not.toHaveCSS("background-image", "none");
      await expect(rows.last()).toHaveCSS("border-bottom-width", "0px");
      await expect(rows.first()).toHaveCSS("border-bottom-width", "0px");
      expect(await rows.first().evaluate(element => getComputedStyle(element).borderRadius)).toBe(await rows.first().evaluate(element => getComputedStyle(element).getPropertyValue("--radius-card").trim()));
      await expect(rows.first()).toHaveCSS("margin-bottom", "8px");
      await expect(rows.first()).not.toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
      const navigation = page.getByRole("navigation", { name: "Workspaces" });
      await expect(navigation).toHaveCSS("box-shadow", "none");
      const active = navigation.locator("a[aria-current]");
      await active.focus();
      await expect(active).toBeFocused();
      await expect(active).toHaveCSS("outline-style", "solid");
      const shell = await page.locator("#workspace").evaluate(element => {
        const header = document.querySelector(".desk-appbar");
        const bounds = element.getBoundingClientRect();
        return {
          inset: bounds.left,
          radius: getComputedStyle(element).borderTopLeftRadius,
          headerRadius: getComputedStyle(header, "::before").borderTopLeftRadius,
          headerLeft: header.getBoundingClientRect().left + parseFloat(getComputedStyle(header, "::before").left),
          headerRight: header.getBoundingClientRect().right - parseFloat(getComputedStyle(header, "::before").right),
          headerShadow: getComputedStyle(header, "::before").boxShadow,
          overflow: document.documentElement.scrollWidth > innerWidth,
        };
      });
      expect(shell).toEqual({ inset: 0, radius: "0px", headerRadius: "0px", headerLeft: 0, headerRight: width, headerShadow: "none", overflow: false });
      await page.screenshot({ path: testInfo.outputPath("flat-mobile-journal.png") });
      await navigation.getByRole("link", { name: "Training", exact: true }).click();
      await expect(page.locator(".day-itinerary")).toBeVisible();
      await expect(page.locator(".training-toolbar")).not.toHaveCSS("background-image", "none");
      expect(await page.locator(".training-toolbar").evaluate(element => {
        const parent = element.getBoundingClientRect();
        const inset = parseFloat(getComputedStyle(element).paddingLeft);
        return inset >= 8 && [...element.querySelectorAll("a, button")].every(control => {
          const bounds = control.getBoundingClientRect();
          return bounds.left >= parent.left + inset - 1 && bounds.right <= parent.right - inset + 1
            && bounds.top >= parent.top + inset - 1 && bounds.bottom <= parent.bottom - inset + 1;
        });
      })).toBe(true);
      await assertLayout(page, `flat mobile ${theme} ${width}`);
      await page.screenshot({ path: testInfo.outputPath("flat-mobile-training.png") });
      await navigation.getByRole("link", { name: "Health", exact: true }).click();
      await expect(page.locator(".health-toolbar")).not.toHaveCSS("background-image", "none");
      await expect(page.locator(".health-toolbar")).toHaveCSS("overflow", "clip");
      const healthTabs = page.locator(".health-toolbar .horizon-switch");
      await expect(healthTabs).toHaveCSS("overflow-x", "auto");
      const lastHealthTab = healthTabs.locator("a").last();
      await lastHealthTab.focus();
      await expect(lastHealthTab).toBeFocused();
      expect(await lastHealthTab.evaluate(element => {
        const bounds = element.getBoundingClientRect();
        const parent = element.closest(".health-toolbar").getBoundingClientRect();
        return bounds.left >= parent.left && bounds.right <= parent.right;
      })).toBe(true);
      await page.emulateMedia({ contrast: "more" });
      await expect(page.locator(".health-toolbar")).toHaveCSS("background-image", "none");
    });
  }
}

for (const theme of ["light", "dark"]) {
  for (const width of [320, 390, 768, 1440]) {
    test(`Prism compact health workflow ${theme} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 844 });
      await page.goto(`/desk.html?theme=${theme}#health`);
      const navigation = page.getByRole("navigation", { name: "Health view" });
      const details = page.getByRole("button", { name: "Show health details", exact: true });
      await expect(details).toHaveAttribute("aria-expanded", "false");
      await expect(page.locator("#health-details")).toBeHidden();
      await assertLayout(page, `health ${theme} ${width}`);
      const disclaimer = page.locator(".health-disclaimer");
      await expect(disclaimer).toHaveCSS("border-left-width", "1px");
      await expect(disclaimer).toHaveCSS("border-radius", "8px");
      await expect(disclaimer).toHaveCSS("font-size", "14px");
      await expect(disclaimer.locator("svg")).toHaveAttribute("aria-hidden", "true");
      expect(await disclaimer.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
      await disclaimer.scrollIntoViewIfNeeded();
      expect((await new AxeBuilder({ page }).include(".health-disclaimer").analyze()).violations).toEqual([]);
      await disclaimer.screenshot({ path: testInfo.outputPath("health-disclaimer.png") });
      await page.locator("#workspace").evaluate(element => { element.scrollTop = 0; });
      if (width <= 800) {
        await expect(page.locator(".health-heading")).toBeHidden();
        await expect(details).toBeInViewport();
        const geometry = await page.evaluate(() => ({
          tabs: [...document.querySelectorAll(".health-toolbar a")].map(element => ({ top: element.offsetTop, height: element.getBoundingClientRect().height, width: element.getBoundingClientRect().width })),
          metrics: [...document.querySelectorAll(".health-verdict .desk-metric")].map(element => ({ top: element.offsetTop, height: element.getBoundingClientRect().height })),
          detailsBottom: document.querySelector(".health-detail-toggle").getBoundingClientRect().bottom,
          navigationTop: document.querySelector(".desk-navigation").getBoundingClientRect().top,
        }));
        expect(new Set(geometry.tabs.map(tab => tab.top)).size).toBe(1);
        expect(geometry.tabs.every(tab => tab.width >= 44 && tab.height >= 44)).toBe(true);
        expect(new Set(geometry.metrics.map(metric => metric.top)).size).toBe(1);
        expect(geometry.metrics.every(metric => metric.height < 90)).toBe(true);
        expect(geometry.detailsBottom).toBeLessThan(geometry.navigationTop);
      } else await expect(page.locator(".health-heading")).toBeVisible();
      const period = page.getByRole("combobox", { name: "Health period", exact: true });
      await period.selectOption("1mo");
      await expect(period).toHaveValue("1mo");
      await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call => call.command === "dashboard").at(-1)?.args?.range)).toBe("1mo");
      await page.getByRole("button", { name: "Data for readiness", exact: true }).click();
      await expect(page.getByRole("dialog", { name: "Data for readiness", exact: true })).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(page.getByRole("button", { name: "Data for readiness", exact: true })).toBeFocused();
      await details.focus();
      await page.keyboard.press("Enter");
      await expect(page.locator("#health-details")).toBeVisible();
      await expect(page.getByRole("heading", { name: "Daily health history", exact: true })).toBeVisible();
      await page.getByRole("button", { name: "Hide health details", exact: true }).click();
      await expect(page.locator("#health-details")).toBeHidden();
      await page.screenshot({ path: testInfo.outputPath("health-overview.png") });
      for (const label of ["Pace insights", "Milestones", "Pain journal"]) {
        await navigation.getByRole("link", { name: label, exact: true }).click();
        await expect(navigation.getByRole("link", { name: label, exact: true })).toHaveAttribute("aria-current", "page");
        await assertLayout(page, `${label} ${theme} ${width}`);
      }
      await page.getByRole("button", { name: "Log pain", exact: true }).click();
      const dialog = page.getByRole("dialog", { name: "Log pain", exact: true });
      await expect(dialog).toBeVisible();
      await dialog.getByRole("combobox", { name: "Body region", exact: true }).selectOption({ index: 1 });
      await dialog.getByRole("textbox", { name: "Notes", exact: true }).fill("Mobile check");
      await page.evaluate(() => { window.__deskFail = "save_pain_entry"; });
      await dialog.getByRole("button", { name: "Save pain entry", exact: true }).click();
      await expect(dialog.getByRole("alert")).toBeVisible();
      await expect(dialog.getByRole("textbox", { name: "Notes", exact: true })).toHaveValue("Mobile check");
      await page.keyboard.press("Escape");
      await expect(page.getByRole("button", { name: "Log pain", exact: true })).toBeFocused();
    });
  }
}

for (const width of [320, 390]) {
  test(`Prism compact health keeps warning and fallback states at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 740 });
    await page.addInitScript(() => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        const result = await invoke(command, args);
        if (command !== "dashboard") return result;
        return { ...result, scoreHistory: [], trainingStatus: null, recoveryBand: null, activePain: { bodyPart: "knee_left", intensity: 6, startedOn: "2026-11-01" } };
      };
    });
    await page.goto("/desk.html#health");
    await expect(page.getByRole("heading", { name: "Building your baseline", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Building your readiness history", exact: true })).toBeVisible();
    await expect(page.locator(".proposal-prompt")).toContainText("6/10 pain");
    await assertLayout(page, `health warning ${width}`);
    const period = page.getByRole("combobox", { name: "Health period", exact: true });
    await page.evaluate(() => { window.__deskFail = "dashboard"; });
    await period.selectOption("1d");
    await expect(page.getByRole("alert")).toBeVisible();
    await page.evaluate(() => { window.__deskFail = ""; });
    await page.getByRole("button", { name: "Try again", exact: true }).click();
    await expect(page.getByRole("alert")).toHaveCount(0);
    await expect(period).toHaveValue("1d");
    await page.getByRole("button", { name: "Review episode", exact: true }).click();
    await expect(page).toHaveURL(/#health\/journal$/);
    await expect(page.getByRole("button", { name: "Log pain", exact: true })).toBeVisible();
  });
}

for (const theme of ["light", "dark"]) {
  for (const width of [320, 390, 768, 1440]) {
    test(`Prism compact plan workflow ${theme} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 844 });
      await page.goto(`/desk.html?theme=${theme}#training/plan`);
      const current = page.locator(".roadmap-row.is-current");
      await expect(current).toBeInViewport({ ratio: .9 });
      await expect(current).toHaveAttribute("aria-current", "date");
      await expect(page.locator(".training-context h2")).toBeInViewport();
      await assertLayout(page, `plan ${theme} ${width}`);
      const details = page.locator(".plan-basis");
      await expect(page.locator("details, summary")).toHaveCount(0);
      if (width <= 800) {
        await expect(details.getByRole("button", { name: "Show details", exact: true })).toHaveAttribute("aria-expanded", "false");
        await expect(page.locator("#plan-details")).toHaveAttribute("inert", "");
        const geometry = await page.evaluate(() => ({
          metrics: document.querySelector(".plan-overview .metrics-line").getBoundingClientRect().height,
          row: document.querySelector(".roadmap-row.is-current").getBoundingClientRect().height,
          targets: [...document.querySelectorAll(".training-toolbar button, .training-toolbar a")].map(element => element.getBoundingClientRect().height),
        }));
        expect(geometry.metrics).toBeLessThan(130);
        expect(geometry.row).toBeLessThan(110);
        expect(geometry.targets.every(height => height >= 44)).toBe(true);
        await details.getByRole("button", { name: "Show details", exact: true }).focus();
        await page.keyboard.press("Enter");
        await expect(details.getByRole("button", { name: "Hide details", exact: true })).toHaveAttribute("aria-expanded", "true");
      }
      await expect(details.getByRole("heading", { name: "Plan basis", exact: true })).toBeVisible();
      await page.screenshot({ path: testInfo.outputPath("plan-details-open.png") });
      await details.getByRole("button", { name: "End current plan", exact: true }).click();
      const end = page.getByRole("dialog", { name: "End current plan?", exact: true });
      await expect(end).toBeVisible();
      await end.getByRole("button", { name: "Keep plan", exact: true }).click();
      await expect(end).toHaveCount(0);
      if (width <= 800) await details.getByRole("button", { name: "Hide details", exact: true }).click();
      await page.getByRole("button", { name: "Schedule race or test", exact: true }).click();
      await expect(page.getByRole("dialog", { name: "Schedule race or test", exact: true })).toBeVisible();
      await page.keyboard.press("Escape");
      await current.scrollIntoViewIfNeeded();
      await page.screenshot({ path: testInfo.outputPath("plan.png") });
      await current.click();
      await expect(page).toHaveURL(/#training\/week$/);
      if (width <= 800) await expect(page.locator(".day-itinerary h3")).toContainText("Today");
    });
  }
}

for (const width of [320, 390]) {
  test(`Prism mobile training controls ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 844 });
    await page.goto("/desk.html#training/week");
    const toolbar = page.locator(".training-toolbar");
    const horizon = toolbar.getByRole("navigation", { name: "Training horizon" });
    await expect(horizon.getByRole("link", { name: "Week" })).toHaveAttribute("aria-current", "page");
    const geometry = await toolbar.evaluate(element => {
      const links = [...element.querySelectorAll(".horizon-switch a")].map(item => item.getBoundingClientRect());
      const buttons = [...element.querySelectorAll(".date-controls button")].map(item => item.getBoundingClientRect());
      return { links: links.map(rect => ({ x: rect.x, y: rect.y, width: rect.width, height: rect.height })), buttons: buttons.map(rect => ({ x: rect.x, y: rect.y, width: rect.width, height: rect.height })) };
    });
    expect(geometry.links.every(rect => rect.height >= 44 && rect.width >= 44)).toBe(true);
    expect(geometry.buttons.every(rect => rect.height >= 44 && rect.width >= 44)).toBe(true);
    expect(geometry.buttons[0].y).toBeGreaterThan(geometry.links[0].y);
    expect(Math.abs(geometry.links[0].width - geometry.links[2].width)).toBeLessThan(1);
    await toolbar.getByRole("button", { name: "Next week" }).click();
    await toolbar.getByRole("button", { name: "Today" }).click();
    await horizon.getByRole("link", { name: "Month" }).click();
    await expect(toolbar.getByRole("button", { name: "Previous month" })).toBeVisible();
    await horizon.getByRole("link", { name: "Plan" }).click();
    await expect(horizon.getByRole("link", { name: "Plan" })).toHaveAttribute("aria-current", "page");
    await page.screenshot({ path: testInfo.outputPath("training-controls.png") });
    await assertLayout(page, `mobile training controls ${width}`);
  });
}

for (const theme of ["light", "dark"]) {
  for (const width of [320, 375, 390, 768]) {
    test(`Prism mobile today-first training ${theme} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 844 });
      await page.addInitScript(() => {
        const invoke = window.__TAURI_INTERNALS__.invoke;
        window.__TAURI_INTERNALS__.invoke = async (command, args) => {
          const result = await invoke(command, args);
          if (command !== "calendar_events") return result;
          const now = new Date();
          const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
          return [...result.filter(event => event.date !== today), { ...result[0], id: "mobile-today", workoutId: "mobile-today", date: today, title: "Easy run 7 km", kind: "easy", distanceM: 7000, done: false, activityId: null }];
        };
      });
      await page.goto(`/desk.html?theme=${theme}&palette=aurora#training`);
      const itinerary = page.locator(".day-itinerary");
      await expect(itinerary.getByRole("heading")).toContainText("Today");
      const session = itinerary.getByRole("button", { name: /Easy run 7 km/ });
      await expect(session).toBeInViewport();
      await page.evaluate(() => document.fonts.ready);
      const geometry = await page.evaluate(() => {
        const bounds = selector => document.querySelector(selector).getBoundingClientRect();
        const horizon = bounds(".horizon-switch");
        const dates = bounds(".date-controls");
        const session = bounds(".day-itinerary .session-tile");
        const navigation = bounds(".desk-navigation");
        const actions = bounds(".training-secondary-actions");
        return { alignment: horizon.y - dates.y, sessionBottom: session.bottom, navigationTop: navigation.y, bottomGap: innerHeight - navigation.bottom, actionsTop: actions.y, tintedGlass: getComputedStyle(document.querySelector(".desk-navigation")).getPropertyValue("--glass-navigation") !== getComputedStyle(document.documentElement).getPropertyValue("--glass-navigation"), controls: [...document.querySelectorAll(".training-toolbar a, .training-toolbar button")].map(element => element.getBoundingClientRect().height) };
      });
      if (width <= 480) expect(geometry.alignment).toBeLessThan(0);
      else expect(Math.abs(geometry.alignment)).toBeLessThan(1);
      expect(geometry.sessionBottom).toBeLessThan(geometry.navigationTop);
      expect(geometry.actionsTop).toBeGreaterThan(geometry.sessionBottom);
      expect(geometry.bottomGap).toBe(4);
      expect(geometry.tintedGlass).toBe(true);
      expect(geometry.controls.every(height => height >= 44)).toBe(true);
      await assertLayout(page, `today-first ${theme} ${width}`);
      await page.screenshot({ path: testInfo.outputPath("today-first.png") });
      await page.getByRole("button", { name: "Next week", exact: true }).click();
      await page.getByRole("button", { name: "Today", exact: true }).click();
      await expect(itinerary.getByRole("heading")).toContainText("Today");
      await page.getByRole("button", { name: "Reset week", exact: true }).click();
      await expect(page.getByRole("dialog", { name: "Reset this week", exact: true })).toBeVisible();
      await page.keyboard.press("Escape");
      await page.getByRole("button", { name: "Schedule race or test", exact: true }).click();
      await expect(page.getByRole("dialog", { name: "Schedule race or test", exact: true })).toBeVisible();
    });
  }
}

for (const reducedMotion of ["reduce", "no-preference"]) {
  test(`Prism mobile stationary topbar ${reducedMotion}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: 390, height: 640 });
    await page.emulateMedia({ reducedMotion });
    await page.goto("/desk.html#training/month");
    await expect(page.locator(".month-board")).toBeVisible();
    await expect(page.locator(".training-context")).toHaveAttribute("data-reveal", "settled");
    const navigation = page.getByRole("navigation", { name: "Workspaces" });
    const before = await navigation.boundingBox();
    const workspace = page.locator("#workspace");
    const frame = await workspace.boundingBox();
    const header = await page.locator(".desk-appbar").boundingBox();
    await page.mouse.move(120, 400);
    await page.mouse.wheel(0, 180);
    await expect.poll(() => workspace.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
    await expect(page.getByRole("button", { name: "Settings", exact: true })).toBeVisible();
    expect(await workspace.boundingBox()).toEqual(frame);
    expect(await page.locator(".desk-appbar").boundingBox()).toEqual(header);
    await expect(navigation).toBeVisible();
    expect(await navigation.boundingBox()).toEqual(before);
    await page.screenshot({ path: testInfo.outputPath("topbar-stationary.png") });
    await page.mouse.wheel(0, -8);
    await expect(page.locator(".desk-appbar")).not.toHaveClass(/is-scrolled-away/);
    await expect(page.getByRole("button", { name: "Settings", exact: true })).toBeVisible();
    await page.locator("#workspace").evaluate(element => { element.scrollTop += 100; });
    await expect(page.locator(".desk-appbar")).not.toHaveClass(/is-scrolled-away/);
    expect(await workspace.boundingBox()).toEqual(frame);
    await navigation.getByRole("link", { name: "Health", exact: true }).click();
    await expect(page.locator(".desk-appbar")).not.toHaveClass(/is-scrolled-away/);
    await expect(page.getByRole("button", { name: "Settings", exact: true })).toBeVisible();
  });
}

test("Prism mobile shell follows safe-area geometry", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/desk.html#training/month");
  await expect(page.locator(".month-board")).toBeVisible();
  const geometry = await page.evaluate(() => {
    const workspace = document.querySelector(".desk-workspace");
    const header = document.querySelector(".desk-appbar");
    const nav = document.querySelector(".desk-navigation");
    const style = document.createElement("style");
    style.textContent = ".desk-navigation { bottom: max(var(--space-1), calc(34px - var(--space-5))) !important; }";
    document.head.append(style);
    const result = {
      radius: getComputedStyle(workspace).borderTopLeftRadius,
      headerRadius: getComputedStyle(header).borderTopLeftRadius,
      top: workspace.getBoundingClientRect().top,
      headerTop: header.getBoundingClientRect().top,
      shellBottom: innerHeight - workspace.getBoundingClientRect().bottom,
      blur: getComputedStyle(header, "::before").backdropFilter,
      blurTop: header.getBoundingClientRect().top + parseFloat(getComputedStyle(header, "::before").top),
      bottom: innerHeight - nav.getBoundingClientRect().bottom,
    };
    style.remove();
    return result;
  });
  expect(geometry.radius).toBe("0px");
  expect(geometry.headerRadius).toBe(geometry.radius);
  expect(geometry.top).toBe(0);
  expect(geometry.headerTop).toBe(geometry.top);
  expect(geometry.shellBottom).toBe(0);
  expect(geometry.blur).toContain("blur(");
  expect(geometry.blurTop).toBe(0);
  expect(geometry.bottom).toBeGreaterThanOrEqual(4);
  expect(geometry.bottom).toBeLessThan(34);
});

test("Prism mobile opaque topbar stays within the shell while scrolling", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 640 });
  await page.emulateMedia({ contrast: "more" });
  await page.goto("/desk.html#training/month");
  await expect(page.locator(".month-board")).toBeVisible();
  await page.locator("#workspace").evaluate(element => { element.scrollTop = 180; });
  await expect(page.locator(".desk-appbar")).not.toHaveClass(/is-scrolled-away/);
  await expect.poll(() => page.locator(".desk-appbar").evaluate(element => getComputedStyle(element, "::before").visibility)).toBe("visible");
  await expect(page.locator(".desk-appbar")).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  await expect(page.getByRole("navigation", { name: "Workspaces" })).toBeVisible();
  await expect(page.locator(".desk-navigation")).toHaveCSS("background-image", "none");
});

for (const width of [320, 375, 390, 820, 1440]) {
  test(`Prism brand stays aligned and follows every accent at ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/desk.html#training");
    const brand = page.getByRole("link", { name: "Runalytics training" });
    const icon = brand.locator(".desk-brand-icon");
    await expect(icon).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    await expect(brand.locator(".desk-brand-name")).toHaveCSS("font-family", /^Outfit, "?Avenir Next"?, sans-serif$/);
    await expect(brand.locator(".desk-brand-name")).toHaveCSS("font-size", "17px");
    await expect(brand.locator(".desk-brand-name")).toHaveCSS("font-weight", "700");
    await expect(brand.locator(".desk-brand-name")).toHaveCSS("font-synthesis", "none");
    await expect(brand.locator(".desk-brand-name")).toHaveCSS("letter-spacing", /^(normal|0px)$/);
    await expect(icon).toHaveAttribute("aria-hidden", "true");
    const geometry = await brand.evaluate(element => {
      const bounds = element.getBoundingClientRect();
      const mark = element.querySelector(".desk-brand-icon").getBoundingClientRect();
      const name = element.querySelector(".desk-brand-name");
      const text = name.getBoundingClientRect();
      const textVisible = getComputedStyle(name).display !== "none";
      return { width: mark.width, height: mark.height, textVisible, leftOffset: mark.left - bounds.left, centerOffset: (mark.left + (textVisible ? text.right : mark.right)) / 2 - (bounds.left + bounds.width / 2), verticalOffset: textVisible ? (mark.top + mark.height / 2) - (text.top + text.height / 2) : 0 };
    });
    expect(geometry.width).toBe(width > 800 ? 44 : 36);
    expect(geometry.height).toBe(geometry.width);
    expect(geometry.textVisible).toBe(width === 1440);
    expect(Math.abs(width <= 800 ? geometry.leftOffset : geometry.centerOffset)).toBeLessThan(1);
    expect(Math.abs(geometry.verticalOffset)).toBeLessThan(1);
    await assertLayout(page, `brand ${width}`);
    for (const theme of ["light", "dark"]) {
      for (const palette of ["reference", "amethyst", "glacier", "evergreen", "sakura", "ember"]) {
        const colors = await icon.evaluate((element, appearance) => {
          document.documentElement.dataset.deskTheme = appearance.theme;
          document.documentElement.dataset.deskPalette = appearance.palette;
          const context = document.createElement("canvas").getContext("2d");
          const normalized = color => { context.fillStyle = color; return context.fillStyle; };
          const style = getComputedStyle(element);
          return { accent: normalized(style.getPropertyValue("--desk-action").trim()), foreground: normalized(style.getPropertyValue("--desk-on-action").trim()), paths: [...element.querySelectorAll("path")].map(path => normalized(getComputedStyle(path).fill)), tile: normalized(getComputedStyle(element.querySelector("rect")).fill) };
        }, { theme, palette });
        expect(colors.paths, `${theme}/${palette}`).toEqual([colors.accent, colors.accent]);
        expect(colors.tile, `${theme}/${palette}`).toBe(colors.foreground);
      }
      await page.screenshot({ path: testInfo.outputPath(`brand-${theme}.png`) });
    }
  });
}

test("Prism brand keeps enlarged text clear of mobile utilities", async ({ page }) => {
  for (const width of [375, 390]) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/desk.html#training");
    await page.addStyleTag({ content: ":root { font-size: 200%; }" });
    await expect(page.locator(".desk-brand-name")).toBeHidden();
    await expect(page.locator(".desk-brand-icon")).toBeVisible();
    const icon = await page.locator(".desk-brand-icon").boundingBox();
    const utilities = await page.locator(".desk-utilities").boundingBox();
    expect(icon.x + icon.width).toBeLessThan(utilities.x);
    await assertLayout(page, `brand enlarged text ${width}`);
  }
});

for (const reducedMotion of ["no-preference", "reduce"]) {
  test(`Prism brand entrance runs once with ${reducedMotion} motion`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion });
    await page.addInitScript(() => {
      window.__brandEntrances = [];
      const animate = Element.prototype.animate;
      Element.prototype.animate = function (keyframes, options) {
        if (this.matches(".desk-brand-icon, .desk-brand-name, .brand-splash-wordmark, .brand-splash-wordmark > span")) window.__brandEntrances.push({ target: this.className, splash: Boolean(this.closest(".brand-splash")), duration: options?.duration });
        return animate.call(this, keyframes, options);
      };
    });
    await page.goto("/desk.html#journal");
    const brand = page.getByRole("link", { name: "Runalytics training" });
    await expect(brand.locator(".desk-brand-icon")).toHaveAttribute("data-reveal", "settled");
    await expect(brand.locator(".desk-brand-name")).toHaveAttribute("data-reveal", "settled");
    await expect(page.locator(".brand-splash")).toHaveCount(0);
    const entrances = await page.evaluate(() => window.__brandEntrances);
    if (reducedMotion === "reduce") expect(entrances).toEqual([]);
    else {
      expect(entrances.some(entry => entry.splash && entry.target === "desk-brand-icon")).toBe(true);
      expect(entrances.some(entry => entry.splash && entry.target === "brand-splash-wordmark")).toBe(true);
      expect(entrances.every(entry => entry.duration === 560)).toBe(true);
    }
    await page.keyboard.press("Tab");
    await page.keyboard.press("Tab");
    await expect(brand).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.locator(".workspace-heading h1")).toHaveText("Training");
    await page.getByRole("link", { name: "Activities", exact: true }).click();
    await expect(page.locator(".workspace-heading h1")).toHaveText("Activities");
    expect(await page.evaluate(() => window.__brandEntrances)).toEqual(entrances);
    expect(await brand.evaluate(element => element.getAnimations({ subtree: true }).filter(animation => animation.playState === "running").length)).toBe(0);
  });
}

for (const [first, current, label] of [[60, 70, "improving"], [80, 70, "declining"], [70, 70, "steady"], [null, 70, "history"]]) {
  test(`Prism readiness summary ${label}`, async ({ page }) => {
    await page.goto("/desk.html#training");
    await page.evaluate(({ first, current }) => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        const result = await invoke(command, args);
        if (command === "dashboard") {
          result.scores.globalScore = current;
          result.scoreHistory = first == null ? [] : [{ date: "2026-10-01", readiness: first }, { date: "2026-10-02", readiness: current }];
        }
        return result;
      };
    }, { first, current });
    await page.getByRole("link", { name: "Health", exact: true }).click();
    await expect(page.getByRole("region", { name: "Readiness trend" }).getByRole("heading")).toContainText(label);
    await expect(page.locator(".trend-chart:visible")).toHaveCount(1);
    await expect(page.locator("#health-details")).toBeHidden();
    await page.getByRole("button", { name: "Show health details" }).click();
    await expect(page.getByRole("heading", { name: "Recovery and training" })).toBeVisible();
    await page.getByRole("button", { name: "Hide health details" }).click();
    await expect(page.locator("#health-details")).toBeHidden();
  });
}

for (const width of [320, 820, 1440]) {
  test(`Prism centered milestones ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/desk.html#health/milestones");
    await expect(page.locator(".milestone-list article").first()).toBeVisible();
    const layout = await page.locator(".milestones").evaluate(element => {
      const bounds = element.getBoundingClientRect();
      const grid = element.querySelector(".consistency-grid").getBoundingClientRect();
      const cell = element.querySelector(".consistency-week > div").getBoundingClientRect();
      return { center: bounds.x + bounds.width / 2, gridCenter: grid.x + grid.width / 2, cellWidth: cell.width, cellHeight: cell.height, overflow: element.scrollWidth - element.clientWidth };
    });
    expect(Math.abs(layout.center - layout.gridCenter)).toBeLessThan(1);
    expect(Math.abs(layout.cellWidth - layout.cellHeight)).toBeLessThan(1);
    expect(layout.overflow).toBeLessThanOrEqual(1);
    await expect(page.locator(".consistency-week")).toHaveCount(12);
    await expect(page.locator(".consistency-week > div")).toHaveCount(84);
    const total = await page.locator(".milestone-list article").count();
    const earned = await page.locator(".milestone-list article.earned").count();
    const heading = page.locator(".milestones .section-heading h3");
    await expect(heading).toHaveText(`${earned} / ${total} milestones reached`);
    await page.getByRole("checkbox", { name: "Earned only" }).check();
    await expect(page.locator(".milestone-list article:not(.earned)")).toHaveCount(0);
    await expect(heading).toHaveText(`${earned} / ${total} milestones reached`);
  });
}

test("milestone catalog excludes extreme goals", async ({ page }) => {
  await page.goto("/desk.html#milestones");
  const achievements = await page.evaluate(async () => {
    const { gamificationStats } = await import("/src/lib/flow.ts");
    return gamificationStats([], null, false).achievements.map(({ id }) => id);
  });
  for (const id of ["km-384400", "streak-3650", "sessions-10000", "hours-8760", "long-50", "week-100", "month-400", "pace-240", "form-90", "redline", "triple-day"]) expect(achievements).not.toContain(id);
  for (const id of ["first-run", "km-1000", "streak-30", "long-42.2", "week-50", "pace-300"]) expect(achievements).toContain(id);
});

for (const reducedMotion of ["reduce", "no-preference"]) {
  test(`milestone consistency grid motion ${reducedMotion}`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion });
    await page.goto("/desk.html#milestones");
    const weeks = page.locator(".consistency-week");
    await expect(weeks).toHaveCount(12);
    await weeks.first().scrollIntoViewIfNeeded();
    await expect.poll(() => weeks.evaluateAll(elements => elements.every(element => element.dataset.reveal === "settled"))).toBe(true);
    const delays = await weeks.evaluateAll(elements => elements.map(element => element.style.getPropertyValue("--reveal-delay")));
    if (reducedMotion === "reduce") expect(delays.every(delay => !delay)).toBe(true);
    else {
      expect(delays.every(Boolean)).toBe(true);
      for (let index = 1; index < delays.length; index++) expect(parseFloat(delays[index])).toBeGreaterThan(parseFloat(delays[index - 1]));
    }
  });
}

for (const reducedMotion of ["reduce", "no-preference"]) {
  test(`Prism activity switch settles correctly ${reducedMotion}`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion });
    await page.goto("/desk.html#journal/1");
    await expect(page.locator(".activity-review h2")).toBeVisible();
    await expect(page.locator(".activity-transition .route-figure")).toHaveAttribute("data-reveal", "settled");
    await page.evaluate(() => {
      window.__activityDepartures = [];
      document.addEventListener("animationstart", event => {
        if (event.animationName === "content-depart" && event.target.closest(".activity-transition")) window.__activityDepartures.push(getComputedStyle(event.target).animationDelay);
      });
    });
    const next = page.locator(".activity-row:not([aria-pressed=true])").first();
    const name = await next.locator("strong").first().textContent();
    await next.click();
    await expect(page.locator(".activity-review h2")).toHaveText(name);
    await expect(page.locator(".activity-transition")).toHaveCount(1);
    await expect(page.locator(".activity-transition[inert]")).toHaveCount(0);
    expect(await page.locator(".desk-inspector").evaluate(element => element.scrollTop)).toBe(0);
    if (reducedMotion === "no-preference") expect(await page.evaluate(() => window.__activityDepartures.some(delay => parseFloat(delay) > 0))).toBe(true);
    else expect(await page.evaluate(() => window.__activityDepartures)).toEqual([]);
  });
}

for (const width of [820, 1440]) {
  test(`Prism independent activity panes ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/desk.html#journal/1/execution");
    const list = page.getByRole("region", { name: "Activity history" });
    const detail = page.getByRole("complementary", { name: "Activity review" });
    await expect(page.locator(".lap-execution-table")).toBeVisible();
    const geometry = await page.evaluate(() => {
      const list = document.querySelector(".activity-library").getBoundingClientRect();
      const detail = document.querySelector(".desk-inspector").getBoundingClientRect();
      const column = document.querySelector(".lap-execution-table thead th").getBoundingClientRect();
      return { listBottom: list.bottom, detailBottom: detail.bottom, listRight: list.right, detailLeft: detail.left, columnWidth: column.width };
    });
    expect(geometry.listRight).toBeLessThan(geometry.detailLeft);
    expect(geometry.listBottom).toBeGreaterThan(800);
    expect(geometry.listBottom).toBeLessThan(900);
    expect(Math.abs(geometry.detailBottom - geometry.listBottom)).toBeLessThan(3);
    expect(geometry.columnWidth).toBeLessThanOrEqual(50);
    await expect(list).toHaveCSS("padding", "0px");
    const shadow = await list.evaluate(element => {
      const row = element.querySelector(".activity-row");
      const edge = getComputedStyle(element, "::after");
      return { width: parseFloat(edge.width), rowWidth: row.getBoundingClientRect().width, bottom: edge.bottom, radius: edge.borderBottomLeftRadius, clipRadius: getComputedStyle(element).borderBottomLeftRadius };
    });
    expect(shadow.width).toBeCloseTo(shadow.rowWidth, 1);
    expect(shadow.bottom).toBe("0px");
    expect(shadow.radius).toBe(shadow.clipRadius);
    await list.evaluate(element => { element.scrollTop = 100; });
    await expect(list).toHaveAttribute("data-scroll-top", "1");
    await expect(list).toHaveAttribute("data-scroll-bottom", "1");
    await list.screenshot({ path: testInfo.outputPath("activity-scroll-edges.png") });
    await list.evaluate(element => { element.scrollTop = element.scrollHeight; });
    await expect(list).toHaveAttribute("data-scroll-bottom", "");
    expect(await detail.evaluate(element => element.scrollTop)).toBe(0);
    const listPosition = await list.evaluate(element => element.scrollTop);
    await detail.evaluate(element => { element.scrollTop = element.scrollHeight; });
    expect(await detail.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
    expect(await list.evaluate(element => element.scrollTop)).toBe(listPosition);
    expect(await page.locator("#workspace").evaluate(element => element.scrollTop)).toBe(0);
    await page.getByRole("button", { name: "Filters", exact: true }).click();
    await expect(page.locator(".journal-sidebar").getByRole("searchbox")).toBeVisible();
    await expect(detail.getByRole("searchbox")).toHaveCount(0);
    await page.getByRole("button", { name: "Hide filters", exact: true }).click();
    await expect(page.getByRole("searchbox")).toBeHidden();
    await expect(list).toBeVisible();
  });
}

for (const theme of ["light", "dark"]) {
  for (const width of [820, 1180, 1440]) {
    test(`Prism today marker and compact navigation ${theme} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(`/desk.html?theme=${theme}#training/week`);
      const today = page.locator(".day-lane.is-today");
      await expect(today).toBeVisible();
      const days = page.locator(".week-board > .day-lane");
      await expect(days).toHaveCount(7);
      const heights = await days.evaluateAll(elements => elements.map(element => element.getBoundingClientRect().height));
      expect(Math.max(...heights) - Math.min(...heights)).toBeLessThan(1);
      await expect(today).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
      const circle = today.locator("header strong");
      await expect(circle).toHaveCSS("border-radius", "50%");
      await expect(circle).not.toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
      const navigation = page.getByRole("navigation", { name: "Workspaces" });
      for (const label of ["Training", "Activities", "Health"]) {
        const link = navigation.getByRole("link", { name: label, exact: true });
        await expect(link).toBeVisible();
        const dimensions = await link.evaluate(element => ({ width: element.offsetWidth, height: element.offsetHeight }));
        if (width <= 1180) {
          expect(dimensions.width).toBe(dimensions.height);
          expect(dimensions.width).toBeGreaterThanOrEqual(44);
          await expect(link.locator("span:not(.selection-marker)")).toBeHidden();
        } else {
          await expect(link.locator("span:not(.selection-marker)")).toBeVisible();
        }
      }
      await page.screenshot({ path: testInfo.outputPath("week-navigation.png") });
      await navigation.getByRole("link", { name: "Activities", exact: true }).focus();
      await page.keyboard.press("Enter");
      await expect(page.getByRole("heading", { name: "Activities", exact: true })).toBeVisible();
    });
  }

  for (const viewport of [{ width: 320, height: 740 }, { width: 390, height: 844 }, { width: 820, height: 900 }, { width: 913, height: 480 }, { width: 1440, height: 900 }, { width: 820, height: 480, enlarged: true }]) {
    test(`Prism full-height activity filters ${theme} ${viewport.width}x${viewport.height}${viewport.enlarged ? " enlarged" : ""}`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto(`/desk.html?theme=${theme}#journal`);
      if (viewport.enlarged) await page.addStyleTag({ content: ":root { font-size: 200%; }" });
      const filters = page.locator("#activity-filters");
      await expect(filters).toBeHidden();
      await page.getByRole("button", { name: "Filters", exact: true }).click();
      await expect(filters).toBeVisible();
      await expect(filters).toHaveCSS("overflow-y", "visible");
      const geometry = await filters.evaluate(element => ({ height: element.clientHeight, scrollHeight: element.scrollHeight, pageOverflow: document.documentElement.scrollWidth - innerWidth }));
      expect(geometry.scrollHeight).toBeLessThanOrEqual(geometry.height + 1);
      expect(geometry.pageOverflow).toBeLessThanOrEqual(1);
      await page.screenshot({ path: testInfo.outputPath("expanded-filters.png") });
      await filters.getByRole("combobox", { name: "Sport", exact: true }).selectOption("Run");
      await expect(page.locator(".activity-row")).toHaveCount(6);
      const reset = filters.getByRole("button", { name: "Reset filters", exact: true });
      await reset.scrollIntoViewIfNeeded();
      await expect(reset).toBeInViewport();
      await reset.click();
      await expect(page.locator(".activity-row")).toHaveCount(9);
      await page.getByRole("button", { name: "Hide filters", exact: true }).click();
      await expect(filters).toBeHidden();
    });
  }
}

test("Prism preview uses native date shapes and populated initial progress", async ({ page }) => {
  await page.goto("/desk.html#training");
  await expect(page.locator(".session-tile:visible").first()).toBeVisible();
  await expect(page.locator(".training-context")).not.toContainText("Invalid Date");
  const fixture = await page.evaluate(async () => {
    const plan = await window.__TAURI_INTERNALS__.invoke("current_plan");
    const progress = await window.__TAURI_INTERNALS__.invoke("plan_week_progress");
    return { target: plan[0].targetDate, planned: progress.reduce((total, week) => total + week.plannedM, 0), workouts: plan[1].reduce((total, workout) => total + workout.distanceM, 0) };
  });
  expect(fixture.target).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  expect(fixture.planned).toBeGreaterThan(0);
  expect(fixture.planned).toBe(fixture.workouts);
  await expect(page.locator(".schedule-summary")).not.toContainText("/ 0.0 km");
});

const routes = ["training", "training/month", "training/plan", "journal", "health", "health/pace", "health/journal", "health/milestones", "settings", "settings/calendar", "settings/devices", "settings/transfer", "help"];
const widths = [320, 360, 375, 390, 414, 430, 600, 768, 820, 1024, 1280, 1366, 1440, 1600, 1920, 2560];

async function assertLayout(page, context) {
  await expect(page.locator(".workspace-heading h1")).toBeVisible();
  await expect(page.locator(".desk-loading")).toHaveCount(0);
  await page.evaluate(() => document.fonts.ready);
  const geometry = await page.evaluate(() => ({
    pageOverflow: document.documentElement.scrollWidth - innerWidth,
    overflowingText: [...document.querySelectorAll("button, a, h1, h2, h3, p, summary, label")].filter(element => {
      if (!element.checkVisibility() || element.clientWidth === 0 || element.matches(".icon-button") || element.closest(".sr-only, .data-table-scroll, .review-tabs, .horizon-switch, .pace-session-picker, .lap-chart-scroll")) return false;
      return element.scrollWidth > element.clientWidth + 2;
    }).map(element => `${element.className || element.tagName}: ${element.textContent.trim().slice(0, 80)}`),
  }));
  expect(geometry.pageOverflow, context).toBeLessThanOrEqual(1);
  expect(geometry.overflowingText, context).toEqual([]);
}

for (const theme of ["light", "dark"]) {
  for (const width of widths) {
    test(`Prism route matrix ${theme} ${width}px`, async ({ page }, testInfo) => {
      test.setTimeout(90000);
      await page.setViewportSize({ width, height: 960 });
      for (const route of routes) {
        await page.goto(`/desk.html?theme=${theme}#${route}`);
        await assertLayout(page, `${route} ${theme} ${width}`);
        if ([320, 820, 1440, 2560].includes(width)) await page.screenshot({ path: testInfo.outputPath(`${route.replaceAll("/", "-")}.png`), fullPage: true });
      }
    });
  }
}

for (const viewport of [{ width: 537, height: 740 }, { width: 913, height: 480 }, { width: 667, height: 375 }]) {
  test(`Prism intermediate and short viewport ${viewport.width}x${viewport.height}`, async ({ page }) => {
    test.setTimeout(60000);
    await page.setViewportSize(viewport);
    for (const route of routes) {
      await page.goto(`/desk.html#${route}`);
      await assertLayout(page, route);
    }
  });
}

for (const theme of ["light", "dark"]) {
  test(`Prism detail views and settings at rail breakpoint ${theme}`, async ({ page }, testInfo) => {
    test.setTimeout(90000);
    await page.setViewportSize({ width: 820, height: 960 });
    for (const route of ["journal/1", "journal/1/execution", "journal/1/splits", "journal/1/context", ...["watch", "appearance", "thresholds", "training", "calendar", "assistant", "maps", "about"].map(section => `settings/${section}`)]) {
      await page.goto(`/desk.html?theme=${theme}#${route}`);
      await assertLayout(page, `${route} ${theme}`);
      if (route.startsWith("settings")) {
        await expect(page.getByRole("button", { name: "Settings", exact: true })).toHaveAttribute("aria-pressed", "true");
        for (const detail of await page.locator(".settings-editor details:not([open])").all()) await detail.locator(":scope > summary").click();
      }
      expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations, route).toEqual([]);
      await page.screenshot({ path: testInfo.outputPath(`${route.replaceAll("/", "-")}.png`), fullPage: true });
    }
  });

  test(`Prism settled rendering budget ${theme}`, async ({ page }, testInfo) => {
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.setViewportSize({ width: 1440, height: 960 });
    await page.goto(`/desk.html?theme=${theme}#training`);
    await expect(page.locator(".session-tile:visible").first()).toBeVisible();
    await page.waitForFunction(() => {
      const running = document.getAnimations().some(animation => animation.playState === "running");
      window.__prismQuiet = running ? 0 : (window.__prismQuiet ?? 0) + 1;
      return window.__prismQuiet >= 5;
    }, null, { polling: 100 });
    const rendering = await page.evaluate(() => {
      const filtered = [...document.querySelectorAll("body *")].flatMap(element => {
        if (!element.checkVisibility()) return [];
        return [null, "::before", "::after"].filter(pseudo => {
          const style = getComputedStyle(element, pseudo);
          return style.backdropFilter !== "none" && (!pseudo || !["none", "normal"].includes(style.content));
        }).map(pseudo => `${element.className}${pseudo ?? ""}`);
      });
      return { filtered, nodes: document.querySelectorAll("*").length, animations: document.getAnimations().filter(animation => animation.playState === "running").length, fontRequests: performance.getEntriesByType("resource").filter(resource => resource.name.endsWith("woff2")).map(resource => ({ url: new URL(resource.name).pathname, bytes: resource.transferSize })) };
    });
    expect(rendering.filtered.length).toBeLessThanOrEqual(3);
    expect(rendering.filtered.some(selector => /session-tile|activity-row|day-lane/.test(selector))).toBe(false);
    expect(rendering.animations).toBe(0);
    expect(rendering.fontRequests.some(resource => resource.url.includes("outfit"))).toBe(true);
    const screenshot = await page.screenshot({ path: testInfo.outputPath("settled.png") });
    const pixels = await page.evaluate(async source => {
      const image = new Image();
      image.src = source;
      await image.decode();
      const canvas = document.createElement("canvas");
      canvas.width = 144;
      canvas.height = 96;
      const context = canvas.getContext("2d");
      context.drawImage(image, 0, 0, 144, 96);
      const data = context.getImageData(0, 0, 144, 96).data;
      const colors = new Set();
      for (let offset = 0; offset < data.length; offset += 4) colors.add(`${data[offset] >> 4},${data[offset + 1] >> 4},${data[offset + 2] >> 4}`);
      return { width: image.width, height: image.height, distinctColorBuckets: colors.size };
    }, `data:image/png;base64,${screenshot.toString("base64")}`);
    expect(pixels.width).toBe(1440);
    expect(pixels.height).toBe(960);
    expect(pixels.distinctColorBuckets).toBeGreaterThan(20);
    await testInfo.attach("rendering-budget.json", { body: JSON.stringify({ ...rendering, ...pixels }, null, 2), contentType: "application/json" });
  });
}

for (const width of [320, 820, 1440]) {
  test(`Prism enlarged text ${width}px`, async ({ page }, testInfo) => {
    test.setTimeout(60000);
    await page.setViewportSize({ width, height: 960 });
    for (const route of ["training", "training/month", "training/plan", "health", "health/pace", "health/journal", "health/milestones", "settings/thresholds", "settings/appearance", "help"]) {
      await page.goto(`/desk.html#${route}`);
      await page.addStyleTag({ content: ":root { font-size: 200%; }" });
      await assertLayout(page, `200% text ${route} ${width}`);
      await page.screenshot({ path: testInfo.outputPath(`${route.replaceAll("/", "-")}.png`), fullPage: true });
    }
  });
}

test("Prism compact month selects a date and exposes its sessions", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 740 });
  await page.goto("/desk.html#training/month");
  await expect(page.locator(".month-number")).toHaveCount(42);
  const datedSession = await page.locator(".month-day").evaluateAll(days => days
    .filter(day => day.querySelector(".session-tile") && day.querySelector(".month-number").getAttribute("aria-pressed") !== "true")
    .map(day => ({ date: day.querySelector(".month-number").dataset.date, title: day.querySelector(".session-tile strong").textContent }))[0]);
  expect(datedSession).toBeTruthy();
  const date = page.locator(`.month-number[data-date="${datedSession.date}"]`);
  await date.click();
  await expect(date).toHaveAttribute("aria-pressed", "true");
  const session = page.locator(".day-itinerary .session-tile").filter({ hasText: datedSession.title }).first();
  await expect(session).toBeVisible();
  await session.click();
  await expect(page.locator(".desk-inspector")).toContainText(datedSession.title);
  await page.keyboard.press("Escape");
  await expect(session).toBeFocused();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

for (const width of [390, 820, 1440]) {
  for (const theme of ["light", "dark"]) {
    test(`Prism reference ${theme} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 960 });
      await page.goto(`/desk.html?theme=${theme}#training`);
      await expect(page.locator(".session-tile:visible").first()).toBeVisible();
      await page.evaluate(() => document.fonts.ready);
      const geometry = await page.evaluate(() => {
        const rail = document.querySelector(".desk-appbar").getBoundingClientRect();
        const workspace = document.querySelector("#workspace").getBoundingClientRect();
        return {
          overflow: document.documentElement.scrollWidth > innerWidth,
          font: getComputedStyle(document.querySelector("h1")).fontFamily,
          loaded: [...document.fonts].some(font => font.family === "Outfit" && font.status === "loaded"),
          separated: workspace.left >= rail.right,
        };
      });
      expect(geometry.overflow).toBe(false);
      expect(geometry.font).toContain("Outfit");
      expect(geometry.loaded).toBe(true);
      if (width > 800) expect(geometry.separated).toBe(true);
      await page.screenshot({ path: testInfo.outputPath("reference.png"), fullPage: true });
      await page.locator(".session-tile:visible").first().click();
      await expect(page.locator(".desk-inspector")).toBeVisible();
      const bounds = await page.locator(".desk-inspector").boundingBox();
      expect(bounds.x).toBeGreaterThanOrEqual(0);
      expect(bounds.x + bounds.width).toBeLessThanOrEqual(width + 1);
      await page.screenshot({ path: testInfo.outputPath("inspector.png"), fullPage: true });
      await page.getByRole("button", { name: "Close Session", exact: true }).click();
      await expect(page.locator(".desk-inspector")).toHaveCount(0);
    });
  }
}