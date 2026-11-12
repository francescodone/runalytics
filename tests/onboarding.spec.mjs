import { test, expect } from "@playwright/test";
import { installDeskFixture } from "./desk.fixture.mjs";

test.beforeEach(async ({ page }) => installDeskFixture(page));

test("synthetic preview limits four-day plans to one quality session", async ({ page }) => {
  await page.goto("/desk.html?noPlan#training");
  const weeks = await page.evaluate(async () => {
    const request = { startDate: "2027-01-04", targetDate: "2027-01-17", weeklyFrequency: 4, longrunDay: 7, qualityDays: [2, 4], restDays: [], increase: "moderate" };
    return (await window.__TAURI_INTERNALS__.invoke("preview_plan", { request }))[1];
  });
  expect(weeks).toHaveLength(8);
  expect(weeks.filter(workout => workout.title === "Intervals")).toHaveLength(2);
  expect(weeks.filter(workout => workout.title === "Easy run")).toHaveLength(4);
  expect(weeks.slice(0, 4).map(workout => workout.distanceM)).toEqual([7000, 8000, 6000, 10000]);
});

test("synthetic preview shows build progression and holds maintenance", async ({ page }) => {
  await page.goto("/desk.html?noPlan#training");
  const result = await page.evaluate(async () => {
    const request = { startDate: "2027-01-04", targetDate: "2027-01-31", weeklyFrequency: 4, longrunDay: 7, qualityDays: [2], restDays: [], increase: "moderate", recentLongRunKm: 8 };
    const invoke = kind => window.__TAURI_INTERNALS__.invoke("preview_plan", { request: { ...request, kind } });
    return { build: (await invoke("improvement"))[1], maintenance: (await invoke("maintenance"))[1] };
  });
  const total = workouts => workouts.reduce((sum, workout) => sum + workout.distanceM, 0);
  expect(total(result.build.slice(0, 4))).toBe(29000);
  expect(total(result.build.slice(8, 12))).toBe(32000);
  expect(result.build.find(workout => workout.title === "Long run").distanceM).toBe(8000);
  expect(new Set(result.build.slice(0, 4).map(workout => workout.distanceM)).size).toBeGreaterThan(1);
  expect(total(result.maintenance.slice(0, 4))).toBe(total(result.maintenance.slice(8, 12)));
});

for (const width of [320, 1440]) test(`preview week disclosures support keyboard at ${width}px`, async ({ page }, testInfo) => {
  await page.setViewportSize({ width, height: 740 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "preview", name: "Ada", threshold: "4:40" })));
  await page.goto("/desk.html?noPlan#training");
  const week = page.locator(".onboarding-step details").nth(1);
  await expect(week.locator("summary")).toBeVisible();
  await week.locator("summary").focus();
  await page.keyboard.press("Enter");
  await expect(week).toHaveAttribute("open", "");
  await expect(week.locator("li").first()).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  await page.screenshot({ path: testInfo.outputPath(`preview-disclosure-${width}.png`), fullPage: true });
  await page.keyboard.press("Space");
  await expect(week).not.toHaveAttribute("open", "");
});

for (const width of [320, 1440]) test(`recent long run persists and reaches preview and creation at ${width}px`, async ({ page }, testInfo) => {
  await page.setViewportSize({ width, height: 740 });
  await page.addInitScript(() => { if (!localStorage.getItem("runalytics.onboarding.v1")) localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "recent-long-run", name: "Ada", threshold: "4:40" })); });
  await page.goto("/desk.html?noPlan#training");
  const field = page.getByLabel("What is your longest recent long run (km)?");
  await field.fill("2");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Enter a distance between 3 and 100 km");
  await field.fill("8.5");
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem("runalytics.onboarding.v1")).recentLongRun)).toBe("8.5");
  await page.reload();
  await expect(field).toHaveValue("8.5");
  await page.screenshot({ path: testInfo.outputPath(`recent-long-run-${width}.png`), fullPage: true });
  for (const heading of ["Runs per week", "Which days must stay free?", "Long run day", "Tempo and interval days", "Your plan preview"]) {
    await page.getByRole("button", { name: "Continue", exact: true }).click();
    await expect(page.getByRole("heading", { name: heading, exact: true })).toBeVisible();
  }
  await expect(page.getByRole("button", { name: "Build plan" })).toBeEnabled();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "preview_plan").at(-1).args.request.recentLongRunKm)).toBe(8.5);
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  await page.getByRole("button", { name: "Build plan" }).click();
  expect(await page.evaluate(() => window.__deskCalls.find(call => call.command === "create_plan").args.request.recentLongRunKm)).toBe(8.5);
});

for (const width of [320, 1440]) test(`all optional branch combinations remain navigable at ${width}px`, async ({ page }) => {
  await page.setViewportSize({ width, height: 740 });
  await page.goto("/desk.html?noPlan#training");
  const questions = [
    ["Import connections with a QR code?", "Import your connections"],
    ["Connect Strava?", "Connect with Strava"],
    ["Connect a calendar or GitHub Gist?", "Calendar apps"],
    ["Sync across devices?", "Set up device sync"],
  ];
  for (let combination = 0; combination < 16; combination++) {
    await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem("runalytics.onboarding.v1") || "{}").step)).toBe(combination === 0 ? "name" : "goal");
    await page.evaluate(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "transfer", name: "Ada", threshold: "4:40" })));
    await page.reload();
    for (const [index, [question, configuration]] of questions.entries()) {
      await expect(page.getByRole("heading", { name: question, exact: true })).toBeVisible();
      const enabled = Boolean(combination & (1 << index));
      await page.getByRole("radio", { name: enabled ? "Yes" : "Not now" }).check();
      await page.getByRole("button", { name: "Continue", exact: true }).click();
      if (enabled) {
        await expect(page.getByRole("heading", { name: configuration, exact: true })).toBeVisible();
        await page.getByRole("button", { name: "Continue", exact: true }).click();
        if (index === 1) {
          await expect(page.getByRole("heading", { name: "Update your Strava activities?" })).toBeVisible();
          await page.getByRole("button", { name: "Continue", exact: true }).click();
        }
      }
    }
    await expect(page.getByRole("heading", { name: "What are you training for?" })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  }
});

for (const width of [320, 1440]) test(`wizard steps fit and remain reachable at ${width}px`, async ({ page }, testInfo) => {
  await page.setViewportSize({ width, height: 640 });
  await page.goto("/desk.html?noPlan&syncSetup#training");
  const steps = ["name", "watch", "transfer", "transfer-config", "strava", "strava-config", "publishing", "calendar", "calendar-config", "devices", "devices-config", "goal", "method", "distance", "start", "end", "pace", "mileage", "recent-long-run", "frequency", "rest-days", "long-day", "quality", "preview"];
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem("runalytics.onboarding.v1") || "{}").step)).toBe("name");
  for (const step of steps) {
    await page.evaluate(step => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step, name: "Ada", threshold: "4:40", transfer: true, strava: true, calendar: true, devices: true })), step);
    await page.reload();
    const heading = page.locator('.onboarding-step[data-step-state="present"] h1');
    await expect(heading).toBeVisible();
    await expect(heading).toBeFocused();
    await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem("runalytics.onboarding.v1") || "{}").step)).toBe(step);
    const next = page.locator(".onboarding-actions").getByRole("button", { name: /Continue|Build plan/ });
    await next.scrollIntoViewIfNeeded();
    await expect(next).toBeInViewport();
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), step).toBeLessThanOrEqual(1);
    if (["name", "transfer-config", "strava-config", "calendar-config", "devices-config", "quality", "preview"].includes(step)) {
      await page.locator(".onboarding-shell").evaluate(element => { element.scrollTop = 0; });
      await page.screenshot({ path: testInfo.outputPath(`step-${step}-${width}.png`), fullPage: true });
    }
  }
});

test("step transitions animate in and out while the exiting step is inert", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/desk.html?noPlan#training");
  await page.getByLabel("Name", { exact: true }).fill("Ada");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  const outgoing = page.locator('.onboarding-step[data-step-state="exiting"]');
  const incoming = page.locator('.onboarding-step[data-step-state="present"]');
  await expect(outgoing).toHaveAttribute("inert", "");
  await expect(incoming.getByRole("heading", { name: "Connect your watch" })).toBeFocused();
  const values = await page.evaluate(() => {
    const oldStep = document.querySelector('.onboarding-step[data-step-state="exiting"]');
    const newStep = document.querySelector('.onboarding-step[data-step-state="present"]');
    return { old: Number(getComputedStyle(oldStep).opacity), next: Number(getComputedStyle(newStep).opacity) };
  });
  expect(values.old).toBeLessThan(1);
  expect(values.next).toBeLessThan(1);
  await expect(outgoing).toHaveCount(0);
  await expect.poll(() => incoming.evaluate(element => Number(getComputedStyle(element).opacity))).toBe(1);
});

test("reduced motion switches steps immediately and preserves keyboard focus", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/desk.html?noPlan#training");
  await page.getByLabel("Name", { exact: true }).fill("Ada");
  await page.getByRole("button", { name: "Continue", exact: true }).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Connect your watch" })).toBeFocused();
  await expect(page.locator('.onboarding-step[data-step-state="exiting"]')).toHaveCount(0);
  await expect(page.locator('.onboarding-step[data-step-state="present"]')).toHaveCSS("opacity", "1");
});

for (const width of [320, 1440]) test(`each wizard step transitions in both directions at ${width}px`, async ({ page }) => {
  test.setTimeout(120000);
  await page.setViewportSize({ width, height: 740 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/desk.html?noPlan#training");
  await page.getByLabel("Name", { exact: true }).fill("Ada");
  const expected = ["Connect your watch", "Import connections with a QR code?", "Import your connections", "Connect Strava?", "Connect with Strava", "Update your Strava activities?", "Connect a calendar or GitHub Gist?", "Calendar apps", "Sync across devices?", "Set up device sync", "What are you training for?", "Training method", "Distance", "Start date", "End date", "Threshold pace (min/km)", "Current weekly mileage (km)", "What is your longest recent long run (km)?", "Runs per week", "Which days must stay free?", "Long run day", "Tempo and interval days", "Your plan preview"];
  for (const heading of expected) {
    const choice = page.getByRole("radio", { name: "Yes" });
    if (await choice.isVisible()) await choice.check();
    await page.locator(".onboarding-actions").getByRole("button", { name: "Continue", exact: true }).click();
    await expect(page.locator('.onboarding-step[data-step-state="exiting"]')).toHaveAttribute("inert", "");
    await expect(page.locator('.onboarding-step[data-step-state="present"]').getByRole("heading", { name: heading, exact: true })).toBeFocused();
    await expect(page.locator('.onboarding-step[data-step-state="exiting"]')).toHaveCount(0);
  }
  for (const heading of expected.slice(0, -1).reverse()) {
    await page.locator(".onboarding-actions").getByRole("button", { name: "Back", exact: true }).click();
    await expect(page.locator('.onboarding-step[data-step-state="exiting"]')).toHaveAttribute("inert", "");
    await expect(page.locator('.onboarding-step[data-step-state="present"]').getByRole("heading", { name: heading, exact: true })).toBeFocused();
    await expect(page.locator('.onboarding-step[data-step-state="exiting"]')).toHaveCount(0);
  }
});

test("configuration controls remain reachable on a short mobile viewport", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 480 });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "calendar-config", name: "Ada", calendar: true })));
  await page.goto("/desk.html?noPlan#training");
  await page.getByRole("button", { name: "Connect a calendar", exact: true }).click();
  const field = page.getByLabel("Upload URL");
  await field.scrollIntoViewIfNeeded();
  await expect(field).toBeInViewport();
  await expect(page.getByRole("button", { name: "Save calendar settings" })).toBeVisible();
  await page.locator(".onboarding-actions").getByRole("button", { name: "Continue" }).click();
  await expect(page.getByRole("heading", { name: "Sync across devices?" })).toBeFocused();
  expect(await page.locator(".onboarding-shell").evaluate(element => element.scrollTop)).toBe(0);
});

test("QR setup can return to the watch connection without leaving the wizard", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "transfer-config", name: "Ada", transfer: true })));
  await page.goto("/desk.html?noPlan&noWatch#training");
  await page.getByRole("button", { name: "Connect your watch" }).click();
  await expect(page.getByRole("heading", { name: "Connect your watch" })).toBeFocused();
});

test("QR import dialog opens and returns focus inside the wizard", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "transfer-config", name: "Ada", transfer: true })));
  await page.goto("/desk.html?noPlan#training");
  const opener = page.getByRole("button", { name: "Import transfer QR" });
  await opener.click();
  const dialog = page.getByRole("dialog", { name: "Import connection transfer" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByLabel("Transfer passphrase")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(opener).toBeFocused();
});

for (const width of [320, 1440]) test(`onboarding preview supports dark theme at ${width}px`, async ({ page }, testInfo) => {
  await page.setViewportSize({ width, height: 844 });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "preview", name: "Ada", threshold: "4:40" })));
  await page.goto("/desk.html?noPlan&theme=dark#training");
  await expect(page.getByRole("heading", { name: "Your plan preview" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Build plan" })).toBeEnabled();
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  await page.screenshot({ path: testInfo.outputPath(`onboarding-dark-${width}.png`), fullPage: true });
});

test("the watch connection can be deferred", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "watch", name: "Ada" })));
  await page.goto("/desk.html?noPlan&noWatch#training");
  await expect(page.getByText("COROS not connected")).toBeVisible();
  await page.getByRole("button", { name: "Not now" }).click();
  await expect(page.getByRole("heading", { name: "Import connections with a QR code?" })).toBeVisible();
});

for (const width of [320, 390, 820, 1440]) test(`onboarding resumes and previews before confirming at ${width}px`, async ({ page }, testInfo) => {
  await page.setViewportSize({ width, height: 844 });
  await page.goto("/desk.html?noPlan#journal");
  await expect(page.getByRole("navigation", { name: "Workspaces" })).toContainText("Setup");
  await expect(page.locator(".desk-navigation")).toHaveCount(0);
  await page.getByLabel("Name", { exact: true }).fill("Ada");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Connect your watch" })).toBeVisible();
  await page.reload();
  await expect(page.getByRole("heading", { name: "Connect your watch" })).toBeVisible();
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect(page.getByLabel("Name", { exact: true })).toHaveValue("Ada");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  for (const question of ["Import connections with a QR code?", "Connect Strava?", "Connect a calendar or GitHub Gist?", "Sync across devices?"]) {
    await expect(page.getByRole("heading", { name: question, exact: true })).toBeVisible();
    await page.getByRole("radio", { name: "Not now" }).check();
    await page.getByRole("button", { name: "Continue", exact: true }).click();
  }
  for (const question of ["What are you training for?", "Training method", "Distance", "Start date", "End date", "Threshold pace (min/km)", "Current weekly mileage (km)", "What is your longest recent long run (km)?", "Runs per week", "Which days must stay free?", "Long run day", "Tempo and interval days"]) {
    await expect(page.getByRole("heading", { name: question, exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Continue", exact: true }).click();
  }
  await expect(page.getByRole("heading", { name: "Your plan preview" })).toBeVisible();
  const weeks = page.locator(".onboarding-step details");
  await expect.poll(() => weeks.count()).toBeGreaterThan(1);
  const firstDistance = weeks.first().locator("li span:last-child").first();
  await expect(firstDistance).toBeVisible();
  const initialDistance = await firstDistance.textContent();
  await weeks.nth(1).locator("summary").click();
  await expect.poll(() => weeks.nth(1).locator("li").count()).toBeGreaterThan(1);
  await expect(weeks.nth(1).locator("li").first()).toBeVisible();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "create_plan"))).toHaveLength(0);
  await page.getByLabel("Mileage progression").selectOption("ambitious");
  await expect(firstDistance).not.toHaveText(initialDistance);
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  await page.screenshot({ path: testInfo.outputPath(`onboarding-${width}.png`), fullPage: true });
  await page.getByRole("button", { name: "Build plan" }).click();
  await expect(page.locator(".desk-navigation a")).toHaveCount(4);
  await expect(page.getByText("Hello, Ada")).toBeVisible();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "create_plan").at(-1).args.request.increase)).toBe("ambitious");
});

for (const width of [320, 1440]) test(`rest days and long run selection persist at ${width}px`, async ({ page }, testInfo) => {
  await page.setViewportSize({ width, height: 740 });
  await page.addInitScript(() => { if (!localStorage.getItem("runalytics.onboarding.v1")) localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "rest-days", name: "Ada", threshold: "4:40", kind: "maintenance", frequency: 4 })); });
  await page.goto("/desk.html?noPlan#training");
  await page.getByRole("checkbox", { name: "Monday" }).check();
  await page.getByRole("checkbox", { name: "Friday" }).check();
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem("runalytics.onboarding.v1")).restDays)).toEqual([1, 5]);
  await page.reload();
  await expect(page.getByRole("checkbox", { name: "Monday" })).toBeChecked();
  await expect(page.getByRole("checkbox", { name: "Friday" })).toBeChecked();
  await page.screenshot({ path: testInfo.outputPath(`rest-days-${width}.png`), fullPage: true });
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("radio", { name: "Monday" })).toBeDisabled();
  await page.getByRole("radio", { name: "Saturday" }).check();
  await page.screenshot({ path: testInfo.outputPath(`long-run-${width}.png`), fullPage: true });
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("checkbox", { name: "Monday" })).toBeDisabled();
  await page.getByRole("checkbox", { name: "Tuesday" }).check();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("button", { name: "Build plan" })).toBeEnabled();
  const preview = await page.evaluate(() => window.__deskCalls.filter(call => call.command === "preview_plan").at(-1).args.request);
  expect(preview).toMatchObject({ restDays: [1, 5], longrunDay: 6 });
  await page.getByRole("button", { name: "Build plan" }).click();
  expect(await page.evaluate(() => window.__deskCalls.find(call => call.command === "create_plan").args.request)).toMatchObject({ restDays: [1, 5], longrunDay: 6 });
});

test("rest-day conflicts explain how to continue", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "rest-days", name: "Ada", threshold: "4:40", kind: "maintenance", frequency: 7 })));
  await page.goto("/desk.html?noPlan#training");
  await page.getByRole("checkbox", { name: "Monday" }).check();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Rest days must leave enough running days");
  await page.getByRole("checkbox", { name: "Monday" }).uncheck();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Long run day" })).toBeVisible();
});

test("a failed preview keeps the draft and cannot create a plan", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "quality", name: "Ada", threshold: "4:40" })));
  await page.goto("/desk.html?noPlan#training");
  await expect(page.getByRole("heading", { name: "Tempo and interval days" })).toBeVisible();
  await page.evaluate(() => { window.__deskFail = "preview_plan"; });
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Unable to save right now");
  await expect(page.getByRole("button", { name: "Build plan" })).toBeDisabled();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "create_plan"))).toHaveLength(0);
  await page.evaluate(() => { window.__deskFail = ""; });
  await page.getByRole("button", { name: "Back" }).click();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("button", { name: "Build plan" })).toBeEnabled();
});

test("an imported plan is reviewed without creating another plan", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "preview", name: "Ada", threshold: "4:40" })));
  await page.goto("/desk.html#training");
  await expect(page.getByText("Your imported plan is ready.")).toBeVisible();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.locator(".desk-navigation a")).toHaveCount(4);
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "create_plan"))).toHaveLength(0);
});

test("Strava connection and publishing are separate optional steps", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "strava", name: "Ada" })));
  await page.goto("/desk.html?noPlan#training");
  await page.getByRole("radio", { name: "Yes" }).check();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByLabel("Strava Client ID")).toBeVisible();
  await expect(page.getByLabel("Update paired activity descriptions")).toHaveCount(0);
  await page.getByLabel("Strava Client ID").fill("12345");
  await page.getByLabel("Strava Client Secret").fill("secret-not-in-draft");
  await page.getByRole("button", { name: "Save API credentials" }).click();
  await page.getByRole("button", { name: "Connect with Strava" }).click();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Update your Strava activities?" })).toBeVisible();
  await expect(page.getByLabel("Update paired activity descriptions")).not.toBeChecked();
  expect(await page.evaluate(() => localStorage.getItem("runalytics.onboarding.v1"))).not.toContain("secret-not-in-draft");
});