import { test, expect } from "@playwright/test";
import { installDeskFixture } from "./desk.fixture.mjs";

test.beforeEach(async ({ page }) => installDeskFixture(page));

test("setup starts with the transfer choice", async ({ page }) => {
  await page.goto("/desk.html?noPlan&noWatch#training");
  await expect(page.getByRole("heading", { name: "Import connections with a QR code?" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Continue", exact: true })).toBeDisabled();
  await page.getByRole("radio", { name: "Not now" }).check();
  await expect(page.getByRole("button", { name: "Continue", exact: true })).toBeEnabled();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByLabel("Name", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Continue", exact: true })).toBeDisabled();
  await page.getByLabel("Name", { exact: true }).fill("Ada");
  await expect(page.getByRole("button", { name: "Continue", exact: true })).toBeEnabled();
});

for (const viewport of [{ width: 320, height: 640 }, { width: 390, height: 844 }, { width: 667, height: 375 }]) {
  test(`setup respects mobile safe areas at ${viewport.width}x${viewport.height}`, async ({ page }, testInfo) => {
    await page.setViewportSize(viewport);
    await page.goto("/desk.html?noPlan&noWatch#training");
    const shell = page.locator(".onboarding-shell");
    await shell.evaluate(element => {
      element.style.setProperty("--setup-safe-top", "47px");
      element.style.setProperty("--setup-safe-right", "44px");
      element.style.setProperty("--setup-safe-bottom", "34px");
      element.style.setProperty("--setup-safe-left", "44px");
    });
    const header = await page.locator(".onboarding-header strong").boundingBox();
    const navigation = await page.locator(".onboarding-header nav").boundingBox();
    expect(header.y).toBeGreaterThanOrEqual(47);
    expect(header.x).toBeGreaterThanOrEqual(44);
    expect(navigation.x + navigation.width).toBeLessThanOrEqual(viewport.width - 44);
    await page.screenshot({ path: testInfo.outputPath("setup-safe-top.png") });
    await page.getByRole("radio", { name: "Not now" }).check();
    await shell.evaluate(element => { element.scrollTop = element.scrollHeight; });
    const button = page.getByRole("button", { name: "Continue", exact: true });
    await expect(button).toBeInViewport();
    const bounds = await button.boundingBox();
    expect(bounds.x).toBeGreaterThanOrEqual(44);
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(viewport.width - 44);
    expect(bounds.y + bounds.height).toBeLessThanOrEqual(viewport.height - 34);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(viewport.width);
    await button.click();
    await expect(page.getByLabel("Name", { exact: true })).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath("setup-safe-scrolled.png") });
  });
}

for (const scenario of ["matching", "mismatch", "sync-failure", "restored-plan"]) test(`QR-first setup ${scenario}`, async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/desk.html?noPlan&noWatch#training");
  const image = await page.evaluate(async scenario => {
    const { sealConnections } = await import("/src/desk/connection-transfer.tsx");
    const { default: QRCode } = await import("/node_modules/.vite/deps/qrcode.js");
    const source = { version: 1, created: 1, calendar: { uploadUrl: "", subscriptionUrl: "", username: "", password: null }, githubToken: "synthetic-token", identity: { corosCode: scenario === "mismatch" ? "other-account" : "synthetic-account-channel" } };
    const code = await sealConnections(source, "synthetic passphrase");
    if (scenario === "sync-failure") window.__deskFail = "sync_now";
    return QRCode.toDataURL(code, { width: 1000, margin: 4 });
  }, scenario);
  await page.getByRole("radio", { name: "Yes" }).check();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.locator(".onboarding-actions").getByRole("button", { name: "Continue" })).toBeDisabled();
  await page.getByRole("button", { name: "Import transfer QR" }).click();
  const dialog = page.getByRole("region", { name: "Import connection transfer" });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(dialog.getByRole("button", { name: "Review settings" })).toBeDisabled();
  await dialog.getByLabel("QR image").setInputFiles({ name: "transfer.png", mimeType: "image/png", buffer: Buffer.from(image.split(",")[1], "base64") });
  await dialog.getByLabel("Transfer passphrase").fill("synthetic passphrase");
  await dialog.getByRole("button", { name: "Review settings" }).click();
  const contents = dialog.getByRole("region", { name: "Transfer contents" });
  await expect(contents).toContainText("COROS account fingerprint: Included");
  await expect(contents).toContainText("Strava application Client ID and Client Secret: Not included");
  await expect(contents).toContainText("Calendar settings: Not included");
  await expect(contents).toContainText("GitHub token for device sync: Included");
  await expect(dialog.getByText("COROS not connected")).toBeVisible();
  if (scenario === "matching") {
    await page.reload();
    await expect(dialog.getByText("QR code loaded.")).toBeVisible();
    await expect(dialog.getByLabel("Transfer passphrase")).toHaveValue("");
    await dialog.getByLabel("Transfer passphrase").fill("synthetic passphrase");
    await dialog.getByRole("button", { name: "Review settings" }).click();
  }
  await dialog.getByRole("button", { name: "Connect COROS" }).click();
  if (scenario === "mismatch") {
    await expect(dialog.getByRole("alert")).toContainText("Import blocked");
    await expect(dialog.getByRole("button", { name: "Replace connections" })).toBeDisabled();
    return;
  }
  await expect(dialog.getByText("COROS account matches this device.")).toBeVisible();
  if (scenario === "restored-plan") await page.evaluate(async () => {
    window.__deskRemotePlan = await window.__TAURI_INTERNALS__.invoke("preview_plan", { request: { startDate: "2026-11-13", targetDate: "2027-01-13", weeklyFrequency: 4, longrunDay: 7, qualityDays: [2], restDays: [], increase: "moderate" } });
  });
  await dialog.getByRole("button", { name: "Replace connections" }).click();
  if (scenario === "sync-failure") {
    await expect(page.getByRole("alert")).toContainText("Unable to save right now");
    await page.evaluate(() => { window.__deskFail = ""; });
    await page.getByRole("button", { name: "Try again" }).click();
    await expect(page.getByLabel("Name", { exact: true })).toBeVisible();
  }
  else await expect(page.getByLabel("Name", { exact: true })).toBeVisible();
  await expect(page.locator(".onboarding-step[data-step-state='present']").getByRole("region", { name: "Transfer contents" })).toContainText("GitHub token for device sync: Included");
  await expect.poll(() => page.evaluate(() => window.__deskCalls.some(call => call.command === "plan_sync_refresh"))).toBe(true);
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "sync_now"))).toBe(true);
  if (scenario === "restored-plan") {
    await page.getByLabel("Name", { exact: true }).fill("Ada");
    await page.getByRole("button", { name: "Continue", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Connect your watch" })).toBeVisible();
    await page.getByRole("button", { name: "Continue", exact: true }).click();
    await expect(page.getByText("Your imported plan is ready.")).toBeVisible();
    expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "create_plan"))).toBe(false);
  }
});

for (const [step, question] of [["strava", "Connect Strava?"], ["calendar", "Connect a calendar or GitHub Gist?"], ["devices", "Sync across devices?"]]) test(`setup requires a choice on ${step}`, async ({ page }) => {
  await page.addInitScript(step => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step, name: "Ada" })), step);
  await page.goto("/desk.html?noPlan#training");
  await expect(page.getByRole("heading", { name: question, exact: true })).toBeVisible();
  const next = page.locator(".onboarding-actions").getByRole("button", { name: "Continue", exact: true });
  await expect(next).toBeDisabled();
  await page.getByRole("radio", { name: "Not now" }).check();
  await expect(next).toBeEnabled();
});

for (const [step, query] of [["strava-config", ""], ["calendar-config", ""], ["devices-config", "&syncSetup"]]) test(`setup allows deferring incomplete ${step}`, async ({ page }) => {
  await page.addInitScript(step => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step, name: "Ada", strava: true, calendar: true, devices: true })), step);
  await page.goto(`/desk.html?noPlan${query}#training`);
  const actions = page.locator(".onboarding-actions");
  await expect(actions.getByRole("button", { name: "Continue", exact: true })).toBeDisabled();
  await actions.getByRole("button", { name: "Not now" }).click();
  await expect(page.locator('.onboarding-step[data-step-state="present"] h1')).not.toHaveText(step);
});

test("Strava setup enables Continue after connecting", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "strava-config", name: "Ada", strava: true })));
  await page.goto("/desk.html?noPlan#training");
  await expect(page.getByText("Create your own API application in Strava settings.", { exact: false })).toBeVisible();
  await expect(page.getByRole("button", { name: "Open Strava API settings" })).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  const next = page.locator(".onboarding-actions").getByRole("button", { name: "Continue", exact: true });
  await expect(next).toBeDisabled();
  await page.getByLabel("Strava Client ID").fill("12345");
  await page.getByLabel("Strava Client Secret").fill("synthetic-secret");
  await page.getByRole("button", { name: "Save API credentials" }).click();
  await expect(page.locator('.onboarding-step[data-step-state="present"]').getByRole("status").filter({ hasText: "Strava API credentials saved." })).toBeVisible();
  await expect(page.locator(".notification-viewport")).toHaveCount(0);
  await expect(next).toBeDisabled();
  await page.getByRole("button", { name: "Connect with Strava" }).click();
  await expect(next).toBeEnabled();
  await expect(page.locator('.onboarding-step[data-step-state="present"]').getByRole("status").filter({ hasText: "Strava connected. Description updates remain paused until enabled." })).toBeVisible();
  await next.click();
  await page.getByRole("checkbox", { name: "Update paired activity descriptions" }).click();
  const confirmation = page.getByRole("region", { name: "Enable Strava description updates?" });
  await expect(confirmation).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "strava_set_enabled"))).toBe(false);
  await confirmation.getByRole("button", { name: "Enable updates" }).click();
  await expect.poll(() => page.evaluate(() => window.__deskCalls.some(call => call.command === "strava_set_enabled" && call.args.enabled))).toBe(true);
});

test("setup suppresses background notifications and achievement celebrations", async ({ page }) => {
  await page.goto("/desk.html?noPlan#training");
  await expect(page.getByRole("heading", { name: "Import connections with a QR code?" })).toBeVisible();
  await page.evaluate(() => {
    window.__deskEmit("sync://completed", { ok: false });
    window.__deskEmit("plan://stage", { op: "sync", target: "calendar", state: "working" });
    window.dispatchEvent(new CustomEvent("runalytics:achievements-unlocked", { detail: [{ id: "test", name: "Test", desc: "Test", icon: "" }] }));
  });
  await expect(page.locator(".notification-viewport")).toHaveCount(0);
  await expect(page.getByRole("dialog", { name: "Earned" })).toHaveCount(0);
});

test("setup keeps calendar continuation disabled after an upload failure", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "calendar-config", name: "Ada", calendar: true })));
  await page.goto("/desk.html?noPlan&calendarError&calendarUploadFails#training");
  const next = page.locator(".onboarding-actions").getByRole("button", { name: "Continue", exact: true });
  await expect(page.getByText("Calendar host unavailable")).toBeVisible();
  await expect(next).toBeDisabled();
  await page.getByRole("button", { name: "Retry publishing" }).click();
  await expect(page.getByRole("alert")).toContainText("Calendar upload failed");
  await expect(next).toBeDisabled();
  await expect(page.locator(".notification-viewport")).toHaveCount(0);
});

test("QR review identifies all included connection types", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "transfer-config", transfer: true })));
  await page.goto("/desk.html?noPlan#training");
  const image = await page.evaluate(async () => {
    const { sealConnections } = await import("/src/desk/connection-transfer.tsx");
    const { default: QRCode } = await import("/node_modules/.vite/deps/qrcode.js");
    const source = { version: 1, created: 1, calendar: { uploadUrl: "https://example.test/feed", subscriptionUrl: "https://example.test/feed", username: "runner", password: "calendar-secret" }, githubToken: "synthetic-token", strava: { clientId: "12345", clientSecret: "strava-secret" }, identity: { corosCode: "synthetic-account-channel" } };
    return QRCode.toDataURL(await sealConnections(source, "synthetic passphrase"), { width: 1000, margin: 4 });
  });
  await page.getByRole("button", { name: "Import transfer QR" }).click();
  const dialog = page.getByRole("region", { name: "Import connection transfer" });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await dialog.getByLabel("QR image").setInputFiles({ name: "transfer.png", mimeType: "image/png", buffer: Buffer.from(image.split(",")[1], "base64") });
  await dialog.getByLabel("Transfer passphrase").fill("synthetic passphrase");
  await dialog.getByRole("button", { name: "Review settings" }).click();
  const contents = dialog.getByRole("region", { name: "Transfer contents" });
  for (const label of ["COROS account fingerprint", "Strava application Client ID and Client Secret", "Calendar settings", "GitHub token for device sync"]) await expect(contents).toContainText(`${label}: Included`);
  await expect(contents).not.toContainText("synthetic-token");
  await expect(contents).not.toContainText("strava-secret");
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  await contents.scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath("qr-summary-320.png") });
});

test("setup wordmark shares the app typography and original logo", async ({ page }, testInfo) => {
  for (const theme of ["light", "dark"]) for (const width of [320, 1440]) {
    await page.setViewportSize({ width, height: 740 });
    await page.goto(`/desk.html?noPlan&theme=${theme}#training`);
    const wordmark = page.locator(".onboarding-header strong");
    await expect(wordmark).toBeVisible();
    await expect(wordmark).toHaveCSS("font-family", /^Outfit, "?Avenir Next"?, sans-serif$/);
    await expect(wordmark).toHaveCSS("font-size", "17px");
    await expect(wordmark).toHaveCSS("font-weight", "700");
    await expect(wordmark).toHaveCSS("font-synthesis", "none");
    await expect(wordmark).toHaveCSS("letter-spacing", /^(normal|0px)$/);
    await expect(wordmark.locator(".desk-brand-icon")).toBeVisible();
    await expect(wordmark.locator(".desk-brand-icon")).toHaveCSS("width", "32px");
    await expect(page.getByRole("heading").first()).toHaveCSS("font-family", /^Outfit, "?Avenir Next"?, sans-serif$/);
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
    await page.screenshot({ path: testInfo.outputPath(`wordmark-setup-${theme}-${width}.png`), fullPage: true });
  }
});

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
    await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem("runalytics.onboarding.v1") || "{}").step)).toBe(combination === 0 ? "transfer" : "goal");
    await page.evaluate(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "transfer", name: "Ada", threshold: "4:40" })));
    await page.reload();
    for (const [index, [question, configuration]] of questions.entries()) {
      await expect(page.getByRole("heading", { name: question, exact: true })).toBeVisible();
      const enabled = Boolean(combination & (1 << index));
      await page.locator('.onboarding-step[data-step-state="present"]').getByRole("radio", { name: enabled ? "Yes" : "Not now" }).check();
      await page.getByRole("button", { name: "Continue", exact: true }).click();
      if (enabled) {
        await expect(page.getByRole("heading", { name: configuration, exact: true })).toBeVisible();
        await page.locator(".onboarding-actions").getByRole("button", { name: "Not now", exact: true }).click();
      }
      if (index === 0) {
        await expect(page.getByLabel("Name", { exact: true })).toHaveValue("Ada");
        await page.getByRole("button", { name: "Continue", exact: true }).click();
        await page.getByRole("button", { name: "Continue", exact: true }).click();
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
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem("runalytics.onboarding.v1") || "{}").step)).toBe("transfer");
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

for (const width of [320, 1440]) test(`setup viewport reveals keep long previews scroll-driven at ${width}px`, async ({ page }, testInfo) => {
  await page.setViewportSize({ width, height: 640 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "preview", name: "Ada", threshold: "4:40" })));
  await page.goto("/desk.html?noPlan#training");
  const heading = page.locator('.onboarding-step[data-step-state="present"] h1');
  if (width > 800) await expect(heading).toHaveAttribute("data-reveal", "settled");
  else await expect(heading).toBeVisible();
  const week = page.locator(".onboarding-step details").nth(1);
  await week.locator("summary").click();
  const row = week.locator("li").last();
  await row.scrollIntoViewIfNeeded();
  if (width > 800) await expect(row).toHaveAttribute("data-reveal", "settled");
  else await expect(row).toBeVisible();
  await expect(page.locator(".onboarding-step")).toHaveCSS("transform", "none");
  await page.screenshot({ path: testInfo.outputPath(`setup-scroll-${width}.png`) });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.locator('[data-reveal="waiting"], [data-reveal="visible"]')).toHaveCount(0);
});

for (const width of [320, 1440]) test(`step transitions animate in and out while the exiting step is inert at ${width}px`, async ({ page }) => {
  await page.setViewportSize({ width, height: 844 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "name" })));
  await page.goto("/desk.html?noPlan#training");
  await page.getByLabel("Name", { exact: true }).fill("Ada");
  const actions = page.locator(".onboarding-actions");
  const initialTop = await actions.evaluate(element => element.offsetTop);
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  const outgoing = page.locator('.onboarding-step[data-step-state="exiting"]');
  const incoming = page.locator('.onboarding-step[data-step-state="present"]');
  await expect(outgoing).toHaveAttribute("inert", "");
  await expect(incoming).toHaveCount(0);
  expect(await actions.evaluate(element => element.offsetTop)).toBe(initialTop);
  await expect(actions).toBeHidden();
  const values = await page.evaluate(() => {
    const oldStep = document.querySelector('.onboarding-step[data-step-state="exiting"]');
    return { old: getComputedStyle(oldStep).transform, animations: [oldStep, ...oldStep.querySelectorAll('[data-reveal]')].filter(element => element.getAnimations().length).length };
  });
  expect(values.old).toBe("none");
  expect(values.animations).toBeGreaterThan(0);
  await expect(outgoing).toHaveCount(0);
  await expect(incoming.getByRole("heading", { name: "Connect your watch" })).toBeFocused();
  await expect(actions).toBeVisible();
  await expect(incoming).toHaveCSS("transform", "none");
  await expect.poll(() => incoming.evaluate(element => Number(getComputedStyle(element).opacity))).toBe(1);
});

test("setup choices use staggered blur on departure", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 844 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "goal", name: "Ada" })));
  await page.goto("/desk.html?noPlan#training");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  const outgoing = page.locator('.onboarding-step[data-step-state="exiting"]');
  const choices = outgoing.locator(".onboarding-options label");
  await expect(choices).toHaveCount(3);
  const timings = await choices.evaluateAll(elements => elements.map(element => ({ name: getComputedStyle(element).animationName, delay: getComputedStyle(element).animationDelay })));
  expect(timings.map(timing => timing.name)).toEqual(["setup-focus-out", "setup-focus-out", "setup-focus-out"]);
  expect(new Set(timings.map(timing => timing.delay)).size).toBe(3);
  await expect(outgoing).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Training method" })).toBeFocused();
});

for (const step of ["transfer-config", "watch", "strava-config", "publishing", "calendar-config", "devices-config", "preview"]) test(`nested setup content fades when leaving ${step}`, async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 844 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.addInitScript(step => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step, name: "Ada", transfer: true, strava: true, calendar: true, devices: true, threshold: "4:40" })), step);
  await page.goto("/desk.html?noPlan&syncSetup#training");
  const current = page.locator('.onboarding-step[data-step-state="present"]');
  await expect(current.locator("h1")).toBeVisible();
  await page.locator(".onboarding-actions").getByRole("button", { name: "Back", exact: true }).click();
  const outgoing = page.locator('.onboarding-step[data-step-state="exiting"]');
  await expect(outgoing).toHaveAttribute("inert", "");
  await expect.poll(() => outgoing.evaluate(element => Number(getComputedStyle(element).opacity))).toBeLessThan(1);
  if (step === "transfer-config") {
    const copy = outgoing.locator("section > p").first();
    await expect(copy).toHaveCSS("animation-name", "setup-text-out");
    await expect.poll(() => copy.evaluate(element => Number(getComputedStyle(element).opacity))).toBeLessThan(1);
    expect(await copy.evaluate(element => new DOMMatrixReadOnly(getComputedStyle(element).transform).m42)).toBeLessThan(0);
  }
  await expect(outgoing).toHaveCount(0);
  await expect(page.locator('.onboarding-step[data-step-state="present"] h1')).toBeFocused();
});

test("weekday choices stagger in top-to-bottom order", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 844 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "long-day", name: "Ada" })));
  await page.goto("/desk.html?noPlan#training");
  const choices = page.locator('.onboarding-step[data-step-state="present"] .onboarding-options label');
  await expect(choices).toHaveCount(7);
  const delays = await choices.evaluateAll(elements => elements.map(element => parseFloat(getComputedStyle(element).animationDelay)));
  for (let index = 1; index < delays.length; index++) expect(delays[index]).toBeGreaterThan(delays[index - 1]);
  await page.locator(".onboarding-actions").getByRole("button", { name: "Back", exact: true }).click();
  const outgoing = page.locator('.onboarding-step[data-step-state="exiting"] .onboarding-options label');
  await expect(outgoing).toHaveCount(7);
  const exits = await outgoing.evaluateAll(elements => elements.map(element => ({ delay: parseFloat(getComputedStyle(element).animationDelay), name: getComputedStyle(element).animationName })));
  expect(exits.map(exit => exit.name)).toEqual(Array(7).fill("setup-focus-out"));
  for (let index = 1; index < exits.length; index++) expect(exits[index].delay).toBeGreaterThan(exits[index - 1].delay);
});

test("reduced motion switches steps immediately and preserves keyboard focus", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "name" })));
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
  const expected = ["What should we call you?", "Connect your watch", "Connect Strava?", "Connect a calendar or GitHub Gist?", "Sync across devices?", "What are you training for?", "Training method", "Distance", "Start date", "End date", "Threshold pace (min/km)", "Current weekly mileage (km)", "What is your longest recent long run (km)?", "Runs per week", "Which days must stay free?", "Long run day", "Tempo and interval days", "Your plan preview"];
  for (const heading of expected) {
    const choice = page.getByRole("radio", { name: "Not now" });
    if (await choice.isVisible()) await choice.check();
    if (await page.getByLabel("Name", { exact: true }).isVisible()) await page.getByLabel("Name", { exact: true }).fill("Ada");
    await page.locator(".onboarding-actions").getByRole("button", { name: "Continue", exact: true }).click();
    if (width > 800) await expect(page.locator('.onboarding-step[data-step-state="exiting"]')).toHaveAttribute("inert", "");
    await expect(page.locator('.onboarding-step[data-step-state="present"]').getByRole("heading", { name: heading, exact: true })).toBeFocused();
    await expect(page.locator('.onboarding-step[data-step-state="exiting"]')).toHaveCount(0);
  }
  for (const heading of expected.slice(0, -1).reverse()) {
    await page.locator(".onboarding-actions").getByRole("button", { name: "Back", exact: true }).click();
    if (width > 800) await expect(page.locator('.onboarding-step[data-step-state="exiting"]')).toHaveAttribute("inert", "");
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
  await page.locator(".onboarding-actions").getByRole("button", { name: "Not now" }).click();
  await expect(page.getByRole("heading", { name: "Sync across devices?" })).toBeFocused();
  expect(await page.locator(".onboarding-shell").evaluate(element => element.scrollTop)).toBe(0);
});

test("mobile setup shows sync loading through calendar work and preserves the step", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "watch", name: "Ada" })));
  await page.goto("/desk.html?noPlan#training");
  await expect(page.getByRole("heading", { name: "Connect your watch" })).toBeVisible();
  await page.evaluate(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, args) => {
      if (command === "sync_now") {
        window.__deskEmit("sync://started", null);
        await new Promise(resolve => { window.__finishSync = resolve; });
        window.__deskEmit("sync://completed", { ok: true });
      }
      return invoke(command, args);
    };
  });
  await page.getByRole("button", { name: "Sync history" }).click();
  await expect(page.getByRole("status").filter({ hasText: "Syncing activities" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Connect your watch" })).toBeHidden();
  await page.evaluate(() => window.__deskEmit("plan://stage", { op: "sync", target: "calendar", state: "working" }));
  await expect(page.locator(".onboarding-loading").getByRole("status")).toContainText("Updating calendar");
  await page.evaluate(() => window.__finishSync());
  await expect(page.getByRole("heading", { name: "Connect your watch" })).toBeFocused();
  await expect(page.getByRole("button", { name: "Continue", exact: true })).toBeVisible();
});

test("mobile steps remain visible without intersection callbacks", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.addInitScript(() => {
    localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "name", name: "Ada" }));
    window.IntersectionObserver = class { observe() {} unobserve() {} disconnect() {} takeRecords() { return []; } };
  });
  await page.goto("/desk.html?noPlan#training");
  await expect(page.getByLabel("Name", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Connect your watch" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Sync history" })).toBeVisible();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("radio", { name: "Not now" })).toBeVisible();
});

test("background sync preserves an unfinished mobile answer", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "name" })));
  await page.goto("/desk.html?noPlan#training");
  await page.getByLabel("Name", { exact: true }).fill("Ada");
  await page.evaluate(() => window.__deskEmit("sync://started", null));
  await expect(page.locator(".onboarding-loading").getByRole("status")).toBeVisible();
  await page.evaluate(() => window.__deskEmit("sync://completed", { ok: false }));
  await expect(page.getByLabel("Name", { exact: true })).toHaveValue("Ada");
  await expect(page.getByRole("heading", { name: "What should we call you?" })).toBeFocused();
});

test("failed plan creation restores the preview after loading", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "preview", name: "Ada", threshold: "4:40" })));
  await page.goto("/desk.html?noPlan#training");
  await expect(page.getByRole("button", { name: "Build plan" })).toBeEnabled();
  await page.evaluate(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, args) => {
      if (command === "create_plan") {
        await new Promise(resolve => { window.__finishCreate = resolve; });
        throw new Error("Plan could not be saved");
      }
      return invoke(command, args);
    };
  });
  await page.getByRole("button", { name: "Build plan" }).click();
  await expect(page.locator(".onboarding-loading").getByRole("status")).toContainText("Building your baseline");
  await page.evaluate(() => window.__finishCreate());
  await expect(page.getByRole("alert")).toContainText("Plan could not be saved");
  await expect(page.getByRole("heading", { name: "Your plan preview" })).toBeFocused();
  await expect(page.getByRole("button", { name: "Build plan" })).toBeEnabled();
});

test("QR setup can open import before connecting a watch", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "transfer-config", name: "Ada", transfer: true })));
  await page.goto("/desk.html?noPlan&noWatch#training");
  await page.getByRole("button", { name: "Import transfer QR" }).click();
  await expect(page.getByRole("region", { name: "Import connection transfer" })).toBeVisible();
});

test("QR import stays in the wizard and can be cancelled", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await page.addInitScript(() => localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "transfer-config", name: "Ada", transfer: true })));
  await page.goto("/desk.html?noPlan#training");
  const opener = page.getByRole("button", { name: "Import transfer QR" });
  await opener.click();
  const dialog = page.getByRole("region", { name: "Import connection transfer" });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("Transfer passphrase").fill("test passphrase");
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).toHaveCount(0);
  await expect(opener).toBeVisible();
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
  await expect(page.getByRole("heading", { name: "Connect Strava?" })).toBeVisible();
});

for (const width of [320, 390, 820, 1440]) test(`onboarding resumes and previews before confirming at ${width}px`, async ({ page }, testInfo) => {
  await page.setViewportSize({ width, height: 844 });
  await page.goto("/desk.html?noPlan#journal");
  await expect(page.getByRole("navigation", { name: "Workspaces" })).toContainText("Setup");
  await expect(page.locator(".desk-navigation")).toHaveCount(0);
  await page.getByRole("radio", { name: "Not now" }).check();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await page.getByLabel("Name", { exact: true }).fill("Ada");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Connect your watch" })).toBeVisible();
  await page.reload();
  await expect(page.getByRole("heading", { name: "Connect your watch" })).toBeVisible();
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect(page.getByLabel("Name", { exact: true })).toHaveValue("Ada");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  for (const question of ["Connect Strava?", "Connect a calendar or GitHub Gist?", "Sync across devices?"]) {
    await expect(page.getByRole("heading", { name: question, exact: true })).toBeVisible();
    await page.locator('.onboarding-step[data-step-state="present"]').getByRole("radio", { name: "Not now" }).check();
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