import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
import AxeBuilder from "@axe-core/playwright";

test.use({ extraHTTPHeaders: { "X-Runalytics-Legacy": "1" } });

const dashboardFixture = readFileSync(new URL("../__harness.html", import.meta.url), "utf8").match(/<script>([\s\S]*?)<\/script>/)[1];
const reviewFixture = readFileSync(new URL("../__review_stub.js", import.meta.url), "utf8");

test.beforeEach(async ({ page }) => {
  await page.addInitScript({ content: `(function () {
    ${reviewFixture}
    ${dashboardFixture}
    const original = window.__TAURI_INTERNALS__.invoke;
    let created = null;
    let savedSettings = null;
    if (new URLSearchParams(location.search).has("noPlan")) sessionStorage.setItem("test.noPlan", "true");
    window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
      const setup = new URLSearchParams(location.search).get("setup");
      if (cmd === "provider_status" && setup) return [{provider:"coros", connected:setup !== "connect", lastSync:null, detail:""}];
      if (cmd === "current_plan" && created) return created;
      if (cmd === "current_plan" && sessionStorage.getItem("test.noPlan")) return null;
      if (cmd === "pain_entries" && new URLSearchParams(location.search).has("emptyJournal")) return [];
      if (cmd === "get_settings") return savedSettings ?? {...await original(cmd, args), theme: "light", palette: "reference"};
      if (cmd === "save_settings") { savedSettings = args.settings; return null; }
      if (cmd === "plan_defaults") return {currentWeeklyKm: 32, thresholdSecKm: 280, predict5kS: 1200, predict10kS: 2500, predictHalfS: 5400, predictMarathonS: 11400, thisWeekKm: 12, totalKm: 300, weeksObserved: 4, lastSync: new Date().toISOString(), activePain: null};
      if (cmd === "create_plan") {
        sessionStorage.setItem("test.planRequest", JSON.stringify(args.request));
        created = [{id:"test-plan", ...args.request, status:"active", createdAt:new Date().toISOString(), pushedId:null, baselineWeeklyKm:args.request.currentWeeklyKm, qualityOffsetSecKm:0, qualityOffsetFrom:null}, []];
        return created;
      }
      if (cmd === "calendar_events") {
        const pair = await original("current_plan");
        return pair[1].filter(workout => workout.date >= args.from && workout.date <= args.to).map(workout => ({...workout, workoutId:workout.id, activityId:null, notes:"Warm-up\\n2 km easy\\n\\nRepeat 4 times:\\n  1 km at 4:40/km\\n  60 seconds walking rest\\n\\nCool-down\\n1 km easy", adjusted:null, moved:false, plannedDate:workout.date, status:"scheduled", missed:false}));
      }
      if (["pain_days", "pain_by_region", "week_deviations", "proposals"].includes(cmd)) return [];
      if (["plan_receipt", "pending_proposal", "pending_pacing_proposal", "pacing_report"].includes(cmd)) return null;
      if (cmd === "calendar_publishing") return {enabled:false, lastPublished:null};
      if (cmd === "llm_status") return {configured:false, enabled:false, model:null, endpoint:null};
      if (cmd.startsWith("plugin:event|")) return 1;
      return original(cmd, args);
    };
  })();` });
});

test("mobile More is the single entry point for secondary navigation", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator(".page-header a")).toHaveCount(0);
  for (const route of ["settings.html", "injury.html", "support.html"]) {
    await expect(page.locator(`#nav a[href="${route}"]`)).toHaveCount(1);
    await expect(page.locator(`#nav a[href="${route}"]`)).toBeHidden();
  }
  const more = page.locator("#nav-footer summary");
  await more.click();
  await expect(page.locator('#nav a[href="settings.html"]')).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(more).toBeFocused();
  await expect(page.locator("#nav-footer")).not.toHaveAttribute("open");
  await more.click();
  await page.locator('#nav a[href="settings.html"]').click();
  await expect(page.getByRole("heading", { name: "Watch connection", exact: true })).toBeVisible();
  await expect(page.locator(".connection-details")).not.toHaveAttribute("open");
  await page.locator(".connection-details summary").click();
  await expect(page.getByRole("button", { name: "Check API", exact: true })).toBeVisible();
});

for (const step of ["connect", "sync"]) {
  test(`Today offers one next action for the ${step} setup step`, async ({ page }) => {
    await page.goto(`/?setup=${step}`);
    await expect(page.locator("#dash")).toBeHidden();
    await expect(page.locator("#empty").getByRole(step === "connect" ? "link" : "button")).toHaveCount(1);
    await expect(page.locator("#empty")).toContainText(step === "connect" ? "Connect device" : "Sync now");
    await expect(page.locator(".onboarding-overlay")).toHaveCount(0);
  });
}

test("a synced account creates a plan directly without returning through Settings", async ({ page }) => {
  await page.goto("/?noPlan=1");
  await expect(page.locator(".today-action").getByRole("link")).toHaveCount(1);
  await page.locator(".today-action").getByRole("link", { name: "Create a plan", exact: true }).click();
  await expect(page.locator(".wizard-title")).toHaveText("What are you training for?");
  await expect(page.locator("#plan-view")).toBeHidden();
});

test("plan details disclose without duplicating creation or management controls", async ({ page }) => {
  await page.goto("/plan.html");
  await expect(page.locator("#plan-form")).toBeHidden();
  await expect(page.locator(".plan-facts")).toBeHidden();
  await expect(page.locator(".plan-management .actions")).toBeHidden();
  await page.locator(".plan-details summary").click();
  await expect(page.locator(".plan-facts")).toBeVisible();
  await expect(page.getByRole("combobox", { name: "Training week" })).toBeVisible();
  await expect(page.locator("#push-plan")).toHaveCount(1);
  await expect(page.locator("#abort-plan")).toHaveCount(1);
});

test("shared disclosures keep decorative text out of accessible names", async ({ page }) => {
  await page.goto("/plan.html");
  const summary = page.locator(".plan-details > summary");
  await expect(summary).toHaveAccessibleName("Plan details and predictions");
  expect(await summary.evaluate(node => getComputedStyle(node, "::after").content)).toBe('""');
  await expect(page.locator(".plan-facts")).toBeHidden();
  await summary.press("Enter");
  await expect(page.locator(".plan-facts")).toBeVisible();
  await summary.press("Enter");
  await expect(page.locator(".plan-facts")).toBeHidden();
  await expect(summary).toBeFocused();
});

test("mobile Settings uses one category selector and restores focus", async ({ page }) => {
  await page.goto("/settings.html");
  const category = page.locator("#settings-category");
  await expect(category).toHaveText("Watch connection");
  await expect(page.locator("#settings-nav")).toBeHidden();
  await category.click();
  await page.getByRole("tab", { name: "Appearance", exact: true }).click();
  await expect(category).toHaveText("Appearance");
  await expect(category).toBeFocused();
  await expect(page.locator("#settings-nav")).toBeHidden();
  await expect(page.locator("#pane-appearance")).toBeVisible();
  await expect(page.locator("#pane-setup")).toBeHidden();
  await page.setViewportSize({ width: 1440, height: 1000 });
  await expect(category).toBeHidden();
  await expect(page.locator("#settings-nav")).toBeVisible();
  await expect(page.getByRole("tab", { name: "Appearance", exact: true })).toHaveAttribute("aria-selected", "true");
});

test("Progress separates workout history, analytics and milestones", async ({ page }) => {
  await page.goto("/achievements.html");
  await expect(page.locator("#streak-stats")).toBeHidden();
  await page.getByRole("tab", { name: "Performance", exact: true }).click();
  await expect(page.locator("#status-panel")).toContainText("Optimized");
  await expect(page.locator("#activities-table, #plan-timeline")).toHaveCount(0);
  await expect(page.locator("#streak-stats")).toBeHidden();
  await page.getByRole("tab", { name: "Performance", exact: true }).press("ArrowRight");
  await expect(page.getByRole("tab", { name: "Milestones", exact: true })).toBeFocused();
  await expect(page.locator("#streak-stats")).toBeVisible();
  await expect(page.locator("#progress-performance")).toBeHidden();
  await page.getByRole("tab", { name: "Milestones", exact: true }).press("ArrowRight");
  await expect(page.locator("#history-list")).toBeVisible();
});

test.describe("touch injury workflow", () => {
  test.use({ isMobile: true, hasTouch: true, viewport: { width: 390, height: 844 } });

  test("empty journal opens logging and preserves the draft on failure", async ({ page }) => {
    await page.goto("/injury.html?emptyJournal=1");
    await expect(page.getByRole("heading", { name: "No pain logged" })).toBeVisible();
    await page.locator("#pain-empty-log").click();
    const region = page.getByRole("combobox", { name: "Body region", exact: true });
    await expect(region).toBeFocused();
    await region.selectOption("knee.left");
    await page.locator("#pain-notes").fill("Tender after an easy run");
    await page.evaluate(() => {
      const original = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = (command, args) => {
        if (command === "save_pain_entry") return Promise.reject(new Error("Unable to save right now"));
        return original(command, args);
      };
    });
    await page.locator("#pain-save").click();
    await expect(page.locator("#toast")).toContainText("Unable to save right now");
    await expect(page.locator("#pain-save")).toBeEnabled();
    await expect(page.locator("#pain-notes")).toHaveValue("Tender after an easy run");
    await expect(page.locator("#log-pain")).toHaveAttribute("open");
  });

  test("region selection stays compact and preserves form access", async ({ page }) => {
    await page.goto("/injury.html");
    await expect(page.locator("#pain-form")).toBeHidden();
    await page.locator("#log-pain > summary").click();
    const region = page.getByRole("combobox", { name: "Body region", exact: true });
    await expect(region).toBeVisible();
    await region.selectOption("knee.left");
    await expect(page.getByRole("combobox", { name: "Body region", exact: true })).toHaveValue("knee.left");
    await expect(page.locator("#bodymap-selected")).toHaveText("Selected: Left knee");
    await expect(page.locator("#pain-save")).toBeEnabled();
    await expect(page.locator(".bodymap-list-item, .bodymap-svg")).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const result = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze();
    expect(result.violations).toEqual([]);
    await page.screenshot({ path: "test-results/ux-touch-injury.png", fullPage: true });
  });
});

  for (const width of [320, 390, 768, 1440, 1920]) {
  test(`primary shell fits ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.goto("/");
    await expect(page.locator(".today-action")).toBeVisible();
    await expect(page.locator("#nav-destinations .nav-link")).toHaveText(["Today", "Plan", "Calendar", "Progress"]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const bounds = await page.locator("#nav").boundingBox();
    if (width < 1024) expect(bounds.y).toBe(936);
    else expect(bounds.width).toBe(216);
    await page.screenshot({ path: `test-results/today-${width}.png`, fullPage: true });
  });
}

test("shared controls respect theme and reduced motion", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/");
  const nav = page.locator('#nav .nav-link[href="plan.html"]');
  const icon = nav.locator(".nav-icon");
  const idleBackground = await nav.evaluate(node => getComputedStyle(node).backgroundColor);
  await nav.hover();
  await expect(icon).toHaveCSS("transform", "none");
  await expect(nav).not.toHaveCSS("background-color", idleBackground);
  const light = await page.locator("body").evaluate(element => getComputedStyle(element).backgroundColor);
  await page.evaluate(async () => {
    const { setTheme } = await import("/src/lib/theme.ts");
    setTheme("dark");
  });
  const dark = await page.locator("body").evaluate(element => getComputedStyle(element).backgroundColor);
  expect(dark).not.toBe(light);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(icon).toHaveCSS("transform", "none");
  await expect(nav).toHaveCSS("transition-duration", "0s");
});

test("sidebar uses the bundled app icon without visual alterations", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/");
  const logo = page.locator("#nav img.brand-mark");
  await expect(logo).toBeVisible();
  const imageResponse = await page.request.get(await logo.getAttribute("src"));
  expect(await imageResponse.body()).toEqual(readFileSync(new URL("../src-tauri/icons/128x128@2x.png", import.meta.url)));
  for (const theme of ["light", "dark"]) {
    await page.evaluate(async choice => {
      const { setTheme } = await import("/src/lib/theme.ts");
      setTheme(choice);
    }, theme);
    await expect(logo).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
    await expect(logo).toHaveCSS("border-radius", "0px");
    await expect(logo).toHaveCSS("box-shadow", "none");
    expect(await logo.evaluate(image => image.complete && image.naturalWidth === 256)).toBe(true);
  }
});

test("Plan is the home for the full plan and current week", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("#today-hero")).toBeVisible();
  await expect(page.locator("#week-plan")).toHaveCount(0);
  await page.locator("#nav-destinations").getByRole("link", { name: "Plan", exact: true }).click();
  await expect(page.getByRole("combobox", { name: "Training week" })).toBeVisible();
  await expect(page.getByRole("tabpanel")).toBeVisible();
  await page.getByRole("combobox", { name: "Training week" }).selectOption({ index: 0 });
  await expect(page.locator("#plan-week-summary")).toContainText("Week 1");
  await page.goto("/calendar.html");
  await expect(page.locator("#cal-agenda")).toBeVisible();
  await expect(page.locator("#cal-week")).toHaveCount(0);
});

test("Today shows only today's session and two training signals", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator(".today-action")).toContainText("Today's session");
  await expect(page.locator(".pillar-name")).toHaveText(["Training load", "Body state"]);
  await expect(page.locator(".week-dates")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Training check", exact: true })).toBeVisible();
  await expect(page.locator(".today-context").getByRole("link")).toHaveAttribute("href", "plan.html");
  await expect(page.locator("#week, .decision-lead, .hero-chips, .completed-run, .today-greeting, .hero-brief")).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Progress", exact: true })).toBeVisible();
});

test("Progress filters real activity totals and retains analytics", async ({ page }) => {
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto("/achievements.html");
  await expect(page.locator(".activity-row")).toHaveCount(9);
  await page.locator("#history-type").selectOption("cross");
  await expect(page.locator(".activity-row")).toHaveCount(3);
  await expect(page.locator("#history-summary")).toContainText("0.0 km running");
  await page.getByRole("tab", { name: "Performance", exact: true }).click();
  await expect(page.locator("#status-panel")).toContainText("Optimized");
  await expect(page.locator("#fitness-cards")).toContainText("VO₂max");
  await page.getByRole("tab", { name: "Workouts", exact: true }).click();
  await expect(page.locator("#progress-performance")).toBeHidden();
  expect(errors).toEqual([]);
});

test("performance overview discloses details and honours the body-state link", async ({ page }) => {
  await page.goto("/achievements.html#performance");
  await expect(page.locator("#status-panel")).toBeVisible();
  await expect(page.locator("#volume-panel")).toBeVisible();
  await expect(page.locator("#recovery-panel")).toBeHidden();
  await expect(page.locator("#risk-panel")).toHaveCount(0);
  await page.locator("#recovery > summary").click();
  await expect(page.locator("#recovery-panel")).toBeVisible();
  await page.goto("/");
  await page.locator('a.pillar[href="achievements.html#recovery"]').click();
  await expect(page.locator("#recovery")).toHaveAttribute("open");
  await expect(page.locator("#recovery-panel")).toBeVisible();
  await expect(page.locator("#risk-panel")).toHaveCount(0);
});

for (const kind of ["race", "improvement", "maintenance"]) {
  test(`wizard validates and creates a ${kind} plan`, async ({ page }) => {
    await page.goto("/plan.html?noPlan=1");
    const next = page.locator(".wizard-controls").getByRole("button", { name: "Continue" });
    await expect(page.locator(".wizard-title")).toHaveText("What are you training for?");
    await page.locator(`input[name="plan-kind"][value="${kind}"]`).check({ force: true });
    await next.click();
    await expect(page.locator(".wizard-title")).toHaveText("Choose your target");
    if (kind !== "maintenance") {
      await next.click();
      await expect(page.locator(".wizard-title")).toHaveText("Choose your target");
      const target = new Date();
      target.setDate(target.getDate() + 70);
      await page.locator("#plan-target-date").fill(target.toISOString().slice(0, 10));
    }
    await next.click();
    await expect(page.locator(".wizard-title")).toHaveText("Your running baseline");
    await page.locator("#plan-threshold").fill("wrong");
    await next.click();
    await expect(page.locator(".wizard-title")).toHaveText("Your running baseline");
    await page.locator("#plan-threshold").fill("4:40");
    await next.click();
    await expect(page.locator(".wizard-title")).toHaveText("Make it fit your week");
    await page.locator(".wizard-controls").getByRole("button", { name: "Back" }).click();
    await expect(page.locator("#plan-threshold")).toHaveValue("4:40");
    await next.click();
    await next.click();
    await expect(page.locator(".wizard-title")).toHaveText("Your plan, at a glance");
    await page.getByRole("button", { name: "Generate plan", exact: true }).click();
    await expect(page.locator("#create-plan")).toBeHidden();
    const request = await page.evaluate(() => JSON.parse(sessionStorage.getItem("test.planRequest")));
    expect(request).toMatchObject({ kind, thresholdSecKm: 280, weeklyFrequency: 4, currentWeeklyKm: 32 });
  });
}

test("Plan week selection opens the shared prescription detail", async ({ page }) => {
  await page.goto("/plan.html");
  await expect(page.locator(".plan-week-tab")).toHaveCount(9);
  await page.getByRole("combobox", { name: "Training week" }).selectOption({ index: 0 });
  await expect(page.locator(".plan-week-tab").first()).toHaveAttribute("aria-selected", "true");
  await page.locator(".plan-session").first().click();
  await expect(page.locator(".workout-summary")).toContainText("6.0 km");
  await expect(page.locator(".prescription-block")).toHaveCount(3);
  await expect(page.locator(".workout-notes")).toContainText("Repeat 4 times");
  await page.getByRole("button", { name: "Close workout", exact: true }).click();
  await expect(page.locator("dialog[open]")).toHaveCount(0);
});

test("Plan week navigator retains selection between mobile and desktop", async ({ page }) => {
  await page.goto("/plan.html");
  const picker = page.getByRole("combobox", { name: "Training week" });
  await picker.selectOption({ index: 0 });
  await expect(page.getByRole("button", { name: "Previous week", exact: true })).toBeDisabled();
  await expect(page.locator(".plan-session").first()).not.toContainText("/km/km");
  await page.getByRole("button", { name: "Next week", exact: true }).click();
  await expect(page.locator("#plan-week-summary")).toContainText("Week 2");
  await page.setViewportSize({ width: 1440, height: 1000 });
  await expect(picker).toBeHidden();
  const tab = page.getByRole("tab", { name: /^Week 2 ·/ });
  await expect(tab).toHaveAttribute("aria-selected", "true");
  await tab.press("End");
  await expect(page.getByRole("tab", { name: /^Week 9 ·/ })).toBeFocused();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(picker).toHaveValue("2026-12-14");
  await expect(page.getByRole("button", { name: "Next week", exact: true })).toBeDisabled();
});

test("Calendar expands the mobile month and selects a day", async ({ page }) => {
  await page.goto("/calendar.html");
  await expect(page.locator("#cal-agenda .agenda-row").first()).toBeVisible();
  await expect(page.locator("#cal-grid")).toBeHidden();
  await page.getByRole("button", { name: "Month", exact: true }).click();
  await expect(page.locator("#cal-grid")).toBeVisible();
  await expect(page.locator(".cal-day-indicator").first()).toBeVisible();
  await expect(page.locator("#cal-agenda")).toBeHidden();
  await page.locator(".cal-day-num").first().click();
  await expect(page.locator(".cal-day-num").first()).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#cal-grid")).toBeHidden();
  await expect(page.getByRole("button", { name: "All dates", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "All dates", exact: true })).toBeFocused();
  await page.getByRole("button", { name: "All dates", exact: true }).click();
  await expect(page.locator("#cal-agenda .agenda-row").first()).toBeVisible();
  await expect(page.getByRole("button", { name: "Today", exact: true })).toBeFocused();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test("Calendar keeps month context beside the selected day on desktop", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/calendar.html");
  await expect(page.locator("#cal-grid")).toBeVisible();
  await expect(page.locator("#cal-agenda")).toBeVisible();
  await page.locator('[data-select-date="2026-10-30"]').click();
  await expect(page.locator("#cal-grid")).toBeVisible();
  await expect(page.locator("#cal-agenda .agenda-day")).toHaveCount(1);
  await expect(page.locator("#cal-selection")).toHaveText("30/10/2026");
  await expect(page.locator('[data-select-date="2026-10-30"]')).toBeFocused();
  await page.getByRole("button", { name: "All dates", exact: true }).click();
  await expect(page.locator("#cal-agenda .agenda-day")).not.toHaveCount(1);
});

test("Calendar month buttons fit small screens without clipping", async ({ page }) => {
  for (const width of [320, 390, 768]) {
    await page.setViewportSize({ width, height: 844 });
    await page.goto("/calendar.html");
    await page.getByRole("button", { name: "Month", exact: true }).click();
    await expect(page.locator(".cal-day-num").first()).toBeVisible();
    const layout = await page.locator("#cal-grid").evaluate(grid => ({
      overflow: document.documentElement.scrollWidth > innerWidth,
      clipped: [...grid.querySelectorAll(".cal-day-num")].filter(button => {
        const cell = button.closest(".cal-cell").getBoundingClientRect();
        const bounds = button.getBoundingClientRect();
        return bounds.left < cell.left - 1 || bounds.right > cell.right + 1;
      }).length,
    }));
    expect(layout).toEqual({ overflow: false, clipped: 0 });
  }
});

test("Progress combines search and filters with an accessible reset", async ({ page }) => {
  await page.goto("/achievements.html");
  const rows = page.locator(".activity-row");
  await expect(rows.first()).toBeVisible();
  const total = await rows.count();
  const name = await rows.first().locator("h3").innerText();
  const search = page.getByRole("searchbox", { name: "Search workouts" });
  await search.fill(name.toUpperCase());
  await expect(rows.first()).toContainText(name);
  await page.locator("#history-type").selectOption("run");
  await search.fill("no-such-workout");
  await expect(rows).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "No matching workouts" })).toBeVisible();
  await expect(page.locator("#history-summary")).toContainText("0 activities");
  await page.locator("#history-empty-reset").click();
  await expect(search).toBeFocused();
  await expect(search).toHaveValue("");
  await expect(page.locator("#history-type")).toHaveValue("all");
  await expect(rows).toHaveCount(total);
  await expect(page.locator("#history-reset")).toBeHidden();
});

test("Progress opens the shared completed-run review and restores focus", async ({ page }) => {
  await page.goto("/achievements.html");
  const activity = page.locator(".activity-row").first();
  await activity.click();
  await expect(page.locator(".shared-run-review .run-laps")).toContainText("Lap 2 too fast");
  await expect(page.locator(".shared-run-review .run-splits")).toBeVisible();
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await expect(page.locator("dialog[open]")).toHaveCount(0);
  await expect(activity).toBeFocused();
});

test("touch rescheduling retains its receipt and Undo", async ({ page }) => {
  await page.goto("/__calendar_harness.html");
  await page.locator('.agenda-row[data-event-index="4"]').click();
  await page.getByRole("button", { name: "Move to", exact: false }).click();
  await page.locator("#move-session-date").fill("2026-10-15");
  await page.getByRole("button", { name: "Move session", exact: true }).click();
  await expect(page.locator("#toast")).toContainText("Moved to");
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(page.locator("dialog[open]")).toHaveCount(0);
});

test("No-plan accounts retain Calendar and Progress routes", async ({ page }) => {
  for (const route of ["calendar.html", "achievements.html"]) {
    await page.goto(`/${route}?noPlan=1`);
    await expect(page.locator("#nav-destinations .nav-link")).toHaveCount(4);
    await expect(page).toHaveURL(new RegExp(route));
  }
});

test("Reference and legacy palettes remain selectable", async ({ page }) => {
  await page.goto("/support.html");
  const colours = await page.evaluate(async () => {
    const { setPalette } = await import("/src/lib/theme.ts");
    setPalette("amethyst");
    const legacy = getComputedStyle(document.documentElement).getPropertyValue("--bg").trim();
    setPalette("reference");
    return { legacy, reference: getComputedStyle(document.documentElement).getPropertyValue("--bg").trim(), saved: localStorage.getItem("runalytics.palette") };
  });
  expect(colours.reference).toBe("#f5f5f7");
  expect(colours.legacy).not.toBe(colours.reference);
  expect(colours.saved).toBe("reference");
});

for (const width of [320, 390, 768, 1440]) {
  test(`all screens fit ${width}px in both themes`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    const failures = [];
    for (const route of ["index.html", "plan.html", "calendar.html", "achievements.html", "settings.html", "injury.html", "support.html"]) {
      await page.goto(`/${route}`);
      await expect(page.locator(".page-header")).toBeVisible();
      for (const theme of ["light", "dark"]) {
        await page.evaluate(async (choice) => {
          const { setTheme } = await import("/src/lib/theme.ts");
          setTheme(choice);
        }, theme);
        const checks = await page.evaluate(() => {
          const ids = [...document.querySelectorAll("[id]")].map(node => node.id);
          return {
            overflow: document.documentElement.scrollWidth > innerWidth,
            duplicateIds: ids.filter((id, index) => ids.indexOf(id) !== index),
            visibleHidden: [...document.querySelectorAll("[hidden]")].filter(node => getComputedStyle(node).display !== "none").length,
          };
        });
        if (checks.overflow || checks.duplicateIds.length || checks.visibleHidden) failures.push({ route, theme, ...checks });
        if (width === 390 || width === 1440) {
          await page.screenshot({ path: `test-results/premium-${route}-${theme}-${width}.png`, fullPage: true });
        }
      }
    }
    expect(failures).toEqual([]);
  });
}

test("primary screens meet automated WCAG AA checks", async ({ page }) => {
  const failures = [];
  for (const route of ["index.html", "plan.html", "calendar.html", "achievements.html", "settings.html", "injury.html", "support.html"]) {
    await page.goto(`/${route}`);
    await expect(page.locator(".page-header")).toBeVisible();
    for (const theme of ["light", "dark"]) {
      await page.evaluate(async (choice) => {
        const { setTheme } = await import("/src/lib/theme.ts");
        setTheme(choice);
      }, theme);
      const result = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze();
      failures.push(...result.violations.map(violation => ({ route, theme, id: violation.id, nodes: violation.nodes.map(node => ({target:node.target,summary:node.failureSummary})) })));
    }
  }
  expect(failures).toEqual([]);
});

for (const width of [320, 390, 768, 1440]) {
  test(`daily signal labels and values do not collide at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await expect(page.locator(".pillar")).toHaveCount(3);
    for (const pillar of await page.locator(".pillar").all()) {
      const label = await pillar.locator(".pillar-name").boundingBox();
      const value = await pillar.locator("strong").boundingBox();
      const dial = await pillar.locator(".signal-dial").boundingBox();
      expect(label.y + label.height).toBeLessThanOrEqual(value.y);
      expect(value.x + value.width).toBeLessThanOrEqual(dial.x);
    }
    await expect(page.locator(".today-workout .session-open").first()).toBeVisible();
    await expect(page.locator(".session-metrics").first()).toContainText("Distance");
  });
}

test("premium surfaces provide interaction motion and respect reduced motion", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/plan.html");
  const session = page.locator(".plan-session").first();
  await expect(session).toBeVisible();
  expect(await session.evaluate(node => getComputedStyle(node).boxShadow)).not.toBe("none");
  expect(await session.evaluate(node => getComputedStyle(node).transitionDuration)).toContain("0.2s");
  await session.click();
  const card = page.locator("dialog[open] .workout-card");
  await expect(card).toBeVisible();
  expect(await card.evaluate(node => getComputedStyle(node).animationName)).toBe("premium-open");
  await page.getByRole("button", { name: "Close workout", exact: true }).click();
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(session).toHaveCSS("transition-duration", "0s");
  await session.hover();
  await expect(session).toHaveCSS("transform", "none");
  await session.click();
  await expect(card).toHaveCSS("animation-name", "none");
});

for (const width of [390, 1440]) {
  test(`expanded analytics remain accessible and fit ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.goto("/achievements.html#performance");
    await expect(page.locator("#status-panel .band-value")).toBeVisible();
    for (const detail of await page.locator(".analysis-detail").all()) {
      await detail.locator(":scope > summary").click();
      await expect(detail).toHaveAttribute("open");
    }
    for (const theme of ["light", "dark"]) {
      await page.evaluate(async choice => {
        const { setTheme } = await import("/src/lib/theme.ts");
        setTheme(choice);
      }, theme);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      const results = await new AxeBuilder({ page }).include("#performance-analysis").withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze();
      expect(results.violations.map(violation => ({id:violation.id, nodes:violation.nodes.map(node => ({target:node.target, summary:node.failureSummary}))}))).toEqual([]);
      await page.screenshot({ path: `test-results/premium-performance-${theme}-${width}.png`, fullPage: true });
    }
  });
}

test("reference palette keeps mobile status chrome consistent with appearance", async ({ page }) => {
  await page.goto("/support.html");
  for (const [theme, colour] of [["light", "#f5f5f7"], ["dark", "#161618"]]) {
    await page.evaluate(async choice => {
      const { setTheme, setPalette } = await import("/src/lib/theme.ts");
      setPalette("reference");
      setTheme(choice);
    }, theme);
    await expect(page.locator('meta[name="theme-color"]')).toHaveAttribute("content", colour);
  }
});

for (const width of [390, 1440]) {
  test(`system materials and accessibility preferences work at ${width}px`, async ({ page, context }) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.goto("/");
    await expect(page.locator(".pillar")).toHaveCount(3);
    const nav = page.locator("#nav");
    await expect(page.locator("body")).toHaveCSS("font-family", /-apple-system/);
    await expect(page.locator(".pillar").first()).toHaveCSS("border-radius", "0px");
    await expect(page.locator(".pillar").first()).toHaveCSS("box-shadow", "none");
    await expect(nav).toHaveCSS("backdrop-filter", /blur\(28px\)/);
    const session = await context.newCDPSession(page);
    for (const preference of ["prefers-reduced-transparency", "prefers-contrast"]) {
      const value = preference === "prefers-contrast" ? "more" : "reduce";
      await session.send("Emulation.setEmulatedMedia", { features: [{ name: preference, value }] });
      expect(await page.evaluate(query => matchMedia(query).matches, `(${preference}: ${value})`)).toBe(true);
      await expect(nav).toHaveCSS("backdrop-filter", "none");
      await expect(nav).toHaveCSS("background-color", "rgb(255, 255, 255)");
      if (width === 390) {
        await page.locator(".nav-more").click();
        await expect(page.locator(".nav-utilities")).toHaveCSS("backdrop-filter", "none");
        await page.keyboard.press("Escape");
      }
    }
    await session.detach();
  });
}

test("controls respond on press and sheets retain one spring owner", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/plan.html");
  const session = page.locator(".plan-session").first();
  await expect(session).toBeVisible();
  await session.hover();
  await page.mouse.down();
  await expect(session).toHaveCSS("scale", "0.98");
  await page.mouse.move(0, 0);
  await page.mouse.up();
  await expect(session).toHaveCSS("scale", "none");
  for (const width of [390, 700, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await page.goto("/achievements.html");
    await page.locator(".activity-row").first().click();
    const sheet = page.locator(".sheet-dialog .sheet-panel");
    await expect(sheet).toBeVisible();
    await expect(sheet).toHaveCSS("animation-name", "none");
    await expect(sheet).toHaveCSS("border-top-left-radius", "24px");
    await expect(sheet).toHaveCSS("border-top-right-radius", width <= 760 ? "24px" : "0px");
    await expect(sheet).toHaveCSS("width", `${width <= 760 ? width : 480}px`);
    await expect.poll(async () => {
      const bounds = await sheet.boundingBox();
      return Math.abs(width <= 760 ? bounds.y + bounds.height - 1000 : bounds.x + bounds.width - width);
    }).toBeLessThan(1);
    await page.screenshot({ path: `test-results/system-review-${width}.png`, fullPage: true });
    await page.keyboard.press("Escape");
    await expect(page.locator(".sheet-dialog[open]")).toHaveCount(0);
  }
});