import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { installDeskFixture } from "./desk.fixture.mjs";

test.beforeEach(async ({ page }) => installDeskFixture(page));

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
      if (width <= 800) {
        await expect(details).not.toHaveAttribute("open", "");
        const geometry = await page.evaluate(() => ({
          metrics: document.querySelector(".plan-overview .metrics-line").getBoundingClientRect().height,
          row: document.querySelector(".roadmap-row.is-current").getBoundingClientRect().height,
          targets: [...document.querySelectorAll(".training-toolbar button, .training-toolbar a")].map(element => element.getBoundingClientRect().height),
        }));
        expect(geometry.metrics).toBeLessThan(130);
        expect(geometry.row).toBeLessThan(110);
        expect(geometry.targets.every(height => height >= 44)).toBe(true);
        await details.locator("summary").focus();
        await page.keyboard.press("Enter");
      }
      await expect(details.getByRole("heading", { name: "Plan basis", exact: true })).toBeVisible();
      await details.getByRole("button", { name: "End current plan", exact: true }).click();
      const end = page.getByRole("dialog", { name: "End current plan?", exact: true });
      await expect(end).toBeVisible();
      await end.getByRole("button", { name: "Keep plan", exact: true }).click();
      await expect(end).toHaveCount(0);
      if (width <= 800) await details.locator("summary").click();
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
      expect(Math.abs(geometry.alignment)).toBeLessThan(1);
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
  test(`Prism mobile scroll-aware topbar ${reducedMotion}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: 390, height: 640 });
    await page.emulateMedia({ reducedMotion });
    await page.goto("/desk.html#training/month");
    await expect(page.locator(".month-board")).toBeVisible();
    await expect(page.locator(".training-context")).toHaveAttribute("data-reveal", "settled");
    const navigation = page.getByRole("navigation", { name: "Workspaces" });
    const before = await navigation.boundingBox();
    await page.mouse.move(8, 400);
    await page.mouse.wheel(0, 180);
    await expect(page.locator(".desk-appbar")).toHaveClass(/is-scrolled-away/);
    await expect(page.getByRole("button", { name: "Settings", exact: true })).toBeHidden();
    await expect(navigation).toBeVisible();
    expect(await navigation.boundingBox()).toEqual(before);
    await page.screenshot({ path: testInfo.outputPath("topbar-hidden.png") });
    await page.mouse.wheel(0, -8);
    await expect(page.locator(".desk-appbar")).not.toHaveClass(/is-scrolled-away/);
    await expect(page.getByRole("button", { name: "Settings", exact: true })).toBeVisible();
    await page.locator("#workspace").evaluate(element => { element.scrollTop += 100; });
    await expect(page.locator(".desk-appbar")).toHaveClass(/is-scrolled-away/);
    await navigation.getByRole("link", { name: "Health", exact: true }).click();
    await expect(page.locator(".desk-appbar")).not.toHaveClass(/is-scrolled-away/);
    await expect(page.getByRole("button", { name: "Settings", exact: true })).toBeVisible();
  });
}

test("Prism mobile opaque topbar hides its background without hiding navigation", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 640 });
  await page.emulateMedia({ contrast: "more" });
  await page.goto("/desk.html#training/month");
  await expect(page.locator(".month-board")).toBeVisible();
  await page.locator("#workspace").evaluate(element => { element.scrollTop = 180; });
  await expect(page.locator(".desk-appbar")).toHaveClass(/is-scrolled-away/);
  await expect.poll(() => page.locator(".desk-appbar").evaluate(element => getComputedStyle(element, "::before").visibility)).toBe("hidden");
  await expect(page.locator(".desk-appbar")).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  await expect(page.getByRole("navigation", { name: "Workspaces" })).toBeVisible();
  await expect(page.locator(".desk-navigation")).toHaveCSS("background-image", "none");
});

for (const width of [320, 375, 390, 820, 1440]) {
  test(`Prism brand stays centered and follows every accent at ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/desk.html#training");
    const brand = page.getByRole("link", { name: "Runalytics training" });
    const icon = brand.locator(".desk-brand-icon");
    await expect(icon).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    const geometry = await brand.evaluate(element => {
      const bounds = element.getBoundingClientRect();
      const mark = element.querySelector(".desk-brand-icon").getBoundingClientRect();
      const name = element.querySelector(".desk-brand-name");
      const text = name.getBoundingClientRect();
      const textVisible = getComputedStyle(name).display !== "none";
      return { width: mark.width, height: mark.height, textVisible, centerOffset: (mark.left + (textVisible ? text.right : mark.right)) / 2 - (bounds.left + bounds.width / 2), verticalOffset: textVisible ? (mark.top + mark.height / 2) - (text.top + text.height / 2) : 0 };
    });
    expect(geometry.width).toBe(width > 800 ? 44 : 36);
    expect(geometry.height).toBe(geometry.width);
    expect(geometry.textVisible).toBe(width === 375 || width === 390 || width === 1440);
    expect(Math.abs(geometry.centerOffset)).toBeLessThan(1);
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
      document.addEventListener("animationstart", event => {
        if (event.target.matches(".desk-brand-icon, .desk-brand-name")) window.__brandEntrances.push({ target: event.target.className, animation: event.animationName, delay: getComputedStyle(event.target).animationDelay });
      });
    });
    await page.goto("/desk.html#journal");
    const brand = page.getByRole("link", { name: "Runalytics training" });
    await expect(brand.locator(".desk-brand-icon")).toHaveAttribute("data-reveal", "settled");
    await expect(brand.locator(".desk-brand-name")).toHaveAttribute("data-reveal", "settled");
    const entrances = await page.evaluate(() => window.__brandEntrances);
    if (reducedMotion === "reduce") expect(entrances).toEqual([]);
    else {
      expect(entrances.map(entry => entry.target)).toEqual(["desk-brand-icon", "desk-brand-name"]);
      expect(entrances.every(entry => entry.animation === "content-arrive")).toBe(true);
      expect(parseFloat(entrances[1].delay)).toBeGreaterThan(parseFloat(entrances[0].delay));
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
    await expect(page.getByRole("heading", { name: "Risk contributors" })).toBeVisible();
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
      const cell = element.querySelector(".consistency-grid > div").getBoundingClientRect();
      return { center: bounds.x + bounds.width / 2, gridCenter: grid.x + grid.width / 2, cellWidth: cell.width, cellHeight: cell.height, overflow: element.scrollWidth - element.clientWidth };
    });
    expect(Math.abs(layout.center - layout.gridCenter)).toBeLessThan(1);
    expect(Math.abs(layout.cellWidth - layout.cellHeight)).toBeLessThan(1);
    expect(layout.overflow).toBeLessThanOrEqual(1);
    await page.getByRole("checkbox", { name: "Earned only" }).check();
    await expect(page.locator(".milestone-list article:not(.earned)")).toHaveCount(0);
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

const routes = ["training", "training/month", "training/plan", "journal", "health", "health/pace", "health/journal", "health/milestones", "settings", "help"];
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
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await expect(page.locator(".desk-inspector")).toHaveCount(0);
    });
  }
}