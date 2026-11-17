import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { installDeskFixture } from "./desk.fixture.mjs";

test.beforeEach(async ({ page }) => installDeskFixture(page));
const snapshot = page => page.evaluate(async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const sync = await invoke("plan_sync_status");
  const { lastChecked, ...config } = sync.config;
  return { plan: await invoke("current_plan"), settings: await invoke("get_settings"), calendar: await invoke("calendar_publishing"), sync: { ...sync, config }, strava: await invoke("strava_status"), activities: (await invoke("all_activities")).map(activity => activity.id), name: localStorage.getItem("runalytics.runnerName"), draft: localStorage.getItem("runalytics.onboarding.v1") };
});
const signedOut = page => expect(page.getByRole("heading", { name: "Sign in to your profile", exact: true })).toBeVisible();
async function logout(page) {
  const earned = page.getByRole("dialog", { name: "Earned", exact: true });
  for (let count = 0; count < 5 && await earned.isVisible(); count++) await earned.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(earned).not.toBeVisible();
  await page.getByRole("button", { name: "Log out", exact: true }).click();
  await page.getByRole("dialog", { name: "Log out of COROS?" }).getByRole("button", { name: "Log out", exact: true }).click();
  await signedOut(page);
}
async function login(page, profileKey = "synthetic-profile") {
  await page.evaluate(key => { window.__deskNextProfile = key; }, profileKey);
  await page.getByRole("button", { name: "Connect COROS", exact: true }).click();
  await expect.poll(() => page.evaluate(async () => (await window.__TAURI_INTERNALS__.invoke("account_session")).profileKey)).toBe(profileKey);
  await expect(page.getByRole("heading", { name: "Sign in to your profile", exact: true })).toHaveCount(0);
}

for (const width of [320, 1440]) for (const theme of ["light", "dark"]) test(`COROS profiles restore A and B independently at ${width}px ${theme}`, async ({ page }) => {
  test.setTimeout(60_000);
  await page.setViewportSize({ width, height: 844 });
  await page.addInitScript(() => {
    if (localStorage.getItem("synthetic.seeded")) return;
    localStorage.setItem("synthetic.seeded", "1");
    localStorage.setItem("runalytics.runnerName", "Ada");
    localStorage.setItem("runalytics.onboarding.complete", "1");
    localStorage.setItem("runalytics.onboarding.v1", JSON.stringify({ step: "pace", name: "Ada", threshold: "4:40" }));
  });
  await page.goto(`/desk.html?multiUser&stravaReview&theme=${theme}#settings/watch`);
  await expect(page.getByRole("button", { name: "Log out", exact: true })).toBeVisible();
  await page.evaluate(async () => {
    const settings = await window.__TAURI_INTERNALS__.invoke("get_settings");
    settings.thresholds.paceToleranceSecKm = 11;
    await window.__TAURI_INTERNALS__.invoke("save_settings", { settings });
  });
  const a = await snapshot(page);
  await page.getByRole("button", { name: "Log out", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Log out of COROS?" });
  await expect(dialog).toContainText("same COROS account");
  expect((await new AxeBuilder({ page }).include(".desk-dialog").analyze()).violations).toEqual([]);
  await page.screenshot({ path: test.info().outputPath("profile-logout.png") });
  await dialog.getByRole("button", { name: "Log out", exact: true }).click();
  await signedOut(page);
  await expect(page.getByRole("navigation", { name: "Workspaces" })).toHaveCount(0);
  expect(await page.evaluate(() => localStorage.getItem("runalytics.runnerName"))).toBeNull();
  expect((await snapshot(page)).plan).toBeNull();
  await page.reload();
  await signedOut(page);
  await page.screenshot({ path: test.info().outputPath("signed-out.png") });
  await login(page, "other-profile");
  await expect(page.getByRole("heading", { name: "Import connections with a QR code?", exact: true })).toBeVisible();
  const emptyB = await snapshot(page);
  expect(emptyB.plan).toBeNull();
  expect(emptyB.name).toBeNull();
  expect(emptyB.draft).not.toContain("Ada");
  expect(emptyB.activities).toEqual([]);
  expect(emptyB.settings.llmEndpoint).toBeNull();
  expect(emptyB.sync.config.hasToken).toBe(false);
  expect(emptyB.calendar.hasPassword).toBe(false);
  expect(emptyB.strava.configured).toBe(false);
  await page.evaluate(async () => {
    localStorage.setItem("runalytics.runnerName", "Bea");
    localStorage.setItem("runalytics.onboarding.complete", "1");
    localStorage.removeItem("runalytics.onboarding.v1");
    await window.__TAURI_INTERNALS__.invoke("create_plan", { request: { name: "Bea plan", startDate: "2026-11-18", targetDate: "2027-02-01", distance: "10K", kind: "race", weeklyFrequency: 3, longrunDay: 7, qualityDays: [3], restDays: [], increase: "conservative", thresholdSecKm: 320, currentWeeklyKm: 20 } });
    await window.__TAURI_INTERNALS__.invoke("push_plan_to_provider", { provider: "coros" });
  });
  await page.goto(`${page.url().split("#")[0]}#settings/watch`);
  await page.reload();
  await expect(page.getByRole("button", { name: "Log out", exact: true })).toBeVisible();
  const b = await snapshot(page);
  await logout(page);
  await login(page);
  await expect(page.getByRole("heading", { name: "Training", exact: true })).toBeVisible();
  expect(await snapshot(page)).toEqual(a);
  await page.reload();
  await expect(page.getByRole("heading", { name: "Training", exact: true })).toBeVisible();
  expect(await snapshot(page)).toEqual(a);
  await page.goto(`${page.url().split("#")[0]}#settings/watch`);
  await logout(page);
  await login(page, "other-profile");
  await expect(page.getByRole("heading", { name: "Training", exact: true })).toBeVisible();
  expect(await snapshot(page)).toEqual(b);
  expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
});

test("logout cancellation, errors and pending retries retain the current COROS profile", async ({ page }) => {
  await page.goto("/desk.html?multiUser#settings/watch");
  const before = await snapshot(page);
  const action = page.getByRole("button", { name: "Log out", exact: true });
  await action.press("Enter");
  await page.keyboard.press("Escape");
  await expect(action).toBeFocused();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "disconnect_provider"))).toEqual([]);
  await action.click();
  const dialog = page.getByRole("dialog", { name: "Log out of COROS?" });
  const confirm = dialog.getByRole("button", { name: "Log out", exact: true });
  await page.evaluate(() => { window.__deskFail = "disconnect_provider"; });
  await confirm.click();
  await expect(dialog.getByRole("alert")).toContainText("Unable to save right now");
  expect(await snapshot(page)).toEqual(before);
  await page.evaluate(() => {
    window.__deskFail = "";
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, args) => {
      if (command === "disconnect_provider") await new Promise(resolve => { window.__releaseLogout = resolve; });
      return invoke(command, args);
    };
  });
  await confirm.click();
  await expect(confirm).toBeDisabled();
  await confirm.evaluate(button => { button.click(); button.click(); });
  await page.keyboard.press("Escape");
  await expect(dialog).toBeVisible();
  await page.evaluate(() => window.__releaseLogout());
  await signedOut(page);
  await page.evaluate(() => { window.__deskFail = "connect_provider"; });
  await page.getByRole("button", { name: "Connect COROS", exact: true }).click();
  await expect(page.getByRole("alert").first()).toContainText("Unable to save right now");
  await expect(page.getByRole("button", { name: "Connect COROS", exact: true })).toBeEnabled();
  expect(await page.evaluate(() => localStorage.getItem("runalytics.runnerName"))).toBeNull();
  await page.evaluate(() => { window.__deskFail = ""; });
  await login(page);
  await expect(page.getByRole("heading", { name: "Training", exact: true })).toBeVisible();
  expect(await snapshot(page)).toEqual(before);
});

test("legacy data is not assigned to an arbitrary login without explicit confirmation", async ({ page }) => {
  await page.addInitScript(() => {
    if (localStorage.getItem("synthetic.seeded")) return;
    localStorage.setItem("synthetic.seeded", "1");
    localStorage.setItem("runalytics.runnerName", "Previous runner");
    localStorage.setItem("runalytics.onboarding.complete", "1");
  });
  await page.goto("/desk.html?multiUser&signedOut&legacyOrphan#training");
  await signedOut(page);
  expect(await page.evaluate(() => localStorage.getItem("runalytics.runnerName"))).toBeNull();
  await login(page);
  await expect(page.getByRole("heading", { name: "Previous local data is available" })).toBeVisible();
  expect((await snapshot(page)).plan).toBeNull();
  await page.getByRole("button", { name: "Import previous local data", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Import previous local data?" });
  await expect(dialog).toContainText("belong to the COROS account");
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "import_legacy_profile"))).toEqual([]);
  await page.getByRole("button", { name: "Import previous local data", exact: true }).click();
  await dialog.getByRole("button", { name: "Import previous local data", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Training", exact: true })).toBeVisible();
  expect((await snapshot(page)).name).toBe("Previous runner");
  expect((await snapshot(page)).plan).not.toBeNull();
});

test("a saved profile can log out when watch credentials are unavailable", async ({ page }) => {
  await page.goto("/desk.html?multiUser&noWatch#settings/watch");
  await expect(page.getByRole("button", { name: "Connect COROS", exact: true })).toBeVisible();
  expect((await snapshot(page)).plan).not.toBeNull();
  await logout(page);
  expect((await snapshot(page)).plan).toBeNull();
});

test("logout stays disabled until active history sync finishes", async ({ page }) => {
  await page.goto("/desk.html?multiUser#settings/watch");
  await page.evaluate(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, args) => {
      if (command === "sync_now") await new Promise(resolve => { window.__releaseSync = resolve; });
      return invoke(command, args);
    };
  });
  await page.getByRole("button", { name: "Sync history", exact: true }).click();
  await expect(page.getByRole("button", { name: "Log out", exact: true })).toBeDisabled();
  await page.evaluate(() => window.__releaseSync());
  await expect(page.getByRole("button", { name: "Log out", exact: true })).toBeEnabled();
  await logout(page);
});

for (const width of [320, 1440]) for (const theme of ["light", "dark"]) test(`sign-in feedback is spaced and callback failure permits retry at ${width}px ${theme}`, async ({ page }) => {
  await page.setViewportSize({ width, height: 844 });
  await page.emulateMedia({ colorScheme: theme });
  await page.goto(`/desk.html?multiUser&signedOut&theme=${theme}#training`);
  await signedOut(page);
  await expect(page.locator("html")).toHaveAttribute("data-desk-theme", theme);
  await page.evaluate(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__restoreSigninInvoke = () => { window.__TAURI_INTERNALS__.invoke = invoke; };
    window.__signinAttempts = 0;
    window.__TAURI_INTERNALS__.invoke = (command, args) => {
      if (command !== "connect_provider") return invoke(command, args);
      window.__signinAttempts++;
      return new Promise((resolve, reject) => { window.__failSignin = () => reject(new Error("COROS sign-in timed out. Try again.")); });
    };
  });
  const button = page.getByRole("button", { name: "Connect COROS", exact: true });
  await button.click();
  await expect(button).toBeDisabled();
  await button.evaluate(element => { element.click(); element.click(); });
  expect(await page.evaluate(() => window.__signinAttempts)).toBe(1);
  const waiting = page.getByRole("status").filter({ hasText: "Waiting for COROS sign-in..." });
  await expect(waiting).toBeVisible();
  const buttonBox = await button.boundingBox();
  const waitingBox = await waiting.boundingBox();
  expect(waitingBox.y - buttonBox.y - buttonBox.height).toBeGreaterThanOrEqual(16);
  expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
  await page.screenshot({ path: test.info().outputPath("signin-pending.png") });
  await page.evaluate(() => window.__failSignin());
  await expect(page.getByRole("alert").first()).toContainText("COROS sign-in timed out. Try again.");
  await expect(button).toBeEnabled();
  await expect(waiting).toHaveCount(0);
  await expect(page.getByText("Finish signing in to COROS when it opens.", { exact: true })).toHaveCount(0);
  await page.evaluate(() => window.__restoreSigninInvoke());
  await login(page, "recovered-profile");
});

test("an unavailable account session fails closed before loading personal workspace data", async ({ page }) => {
  await page.addInitScript(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = (command, args) => command === "account_session" ? Promise.reject(new Error("Profile unavailable")) : invoke(command, args);
  });
  await page.goto("/desk.html#training");
  await expect(page.getByRole("heading", { name: "Could not open your profile" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Try again", exact: true })).toBeVisible();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => ["current_plan", "get_settings", "all_activities"].includes(call.command)))).toEqual([]);
});
