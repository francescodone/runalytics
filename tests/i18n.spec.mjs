import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
import { installDeskFixture } from "./desk.fixture.mjs";

const catalog = language => JSON.parse(readFileSync(new URL(`../src/desk/locales/${language}.json`, import.meta.url), "utf8"));

for (const [language, training, settings, expectedLocale] of [
  ["en-US", "Training", "Settings", "en"],
  ["es-MX", "Entrenamiento", "Ajustes", "es"],
  ["fr-FR", "Entraînement", "Paramètres", "fr"],
  ["de-DE", "Training", "Einstellungen", "de"],
  ["it-IT", "Allenamento", "Impostazioni", "it"],
  ["pt-BR", "Treino", "Definições", "pt"],
  ["ja-JP", "トレーニング", "設定", "ja"],
  ["zh-CN", "训练", "设置", "zh"],
  ["nl-NL", "Training", "Settings", "en"],
]) {
  test(`shell language for ${language}`, async ({ page }) => {
    await installDeskFixture(page);
    await page.addInitScript(language => {
      Object.defineProperty(navigator, "language", { configurable: true, value: language });
    }, language);
    await page.goto("/desk.html#training");
    await expect(page.locator("html")).toHaveAttribute("lang", expectedLocale);
    await expect(page.locator(".desk-appbar")).toHaveAttribute("lang", expectedLocale);
    await expect(page.locator(".workspace-heading h1")).toHaveText(training);
    await page.getByRole("button", { name: settings, exact: true }).click();
    await expect(page.locator(".workspace-heading h1")).toHaveText(settings);
  });
}

for (const language of ["es", "fr", "de", "it", "pt", "ja", "zh"]) {
  test(`translated plan form preserves backend values for ${language}`, async ({ page }) => {
    const messages = catalog(language);
    await installDeskFixture(page);
    await page.addInitScript(language => {
      Object.defineProperty(navigator, "language", { configurable: true, value: language });
    }, language);
    await page.goto("/desk.html?noPlan=1");
    await page.getByRole("button", { name: messages["Create plan"], exact: true }).click();
    const dialog = page.getByRole("dialog");
    await dialog.locator('input[maxlength="60"]').fill("My unchanged custom plan");
    await dialog.getByRole("button", { name: messages.Continue, exact: true }).click();
    await dialog.locator('input[type="number"][max="250"]').fill("40");
    await dialog.getByRole("button", { name: messages.Continue, exact: true }).click();
    await dialog.getByRole("button", { name: messages["Build plan"], exact: true }).click();
    await expect(dialog).toHaveCount(0);
    const request = await page.evaluate(() => window.__deskCalls.find(call => call.command === "create_plan").args.request);
    expect(request).toMatchObject({ kind: "race", trainingMethod: "norwegian", distance: "21K", thresholdSecKm: 280, weeklyFrequency: 4, currentWeeklyKm: 40, name: "My unchanged custom plan" });
    await expect(page.locator(".training-context")).toContainText("My unchanged custom plan");
  });

  test(`translated routes and settings for ${language}`, async ({ page }) => {
    const messages = catalog(language);
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await installDeskFixture(page);
    await page.addInitScript(language => {
      Object.defineProperty(navigator, "language", { configurable: true, value: language });
    }, language);
    for (const [route, label] of [
      ["training/plan", "Training horizon"],
      ["journal", "Activity review"],
      ["health/overview", "Your training, in balance"],
      ["health/pace", "Pace on point"],
      ["health/journal", "Episodes"],
      ["health/milestones", "Last 12 weeks"],
      ["settings", "Third-party connections"],
      ["settings/watch", "Watch connection"],
      ["settings/strava", "Activity descriptions"],
      ["settings/appearance", "Colour palette"],
      ["settings/thresholds", "Training boundaries"],
      ["settings/training", "Training data"],
      ["settings/calendar", "Calendar apps"],
      ["settings/devices", "Device sync"],
      ["settings/transfer", "Transfer connections"],
      ["settings/assistant", "Injury assistant"],
      ["settings/maps", "Offline maps"],
      ["settings/about", "About and updates"],
      ["help", "A recent workout is missing"],
    ]) {
      await page.goto(`/desk.html#${route}`);
      await expect(page.locator("#workspace")).toContainText(messages[label]);
      await expect(page.locator("#workspace")).not.toContainText(/\{\{\w+\}\}|__7\d{3}__/);
    }
    expect(errors).toEqual([]);
  });
}

test("locale runtime preserves source data, interpolation and fallback", async ({ page }) => {
  await installDeskFixture(page);
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "language", { configurable: true, value: "it-IT" });
  });
  await page.goto("/desk.html#settings");
  const result = await page.evaluate(async () => {
    const { t, text, resolveLocale, date, number, planName } = await import("/src/desk/i18n.ts");
    return {
      missing: t("New untranslated English copy"),
      unknown: text("Provider diagnostic code ZYX-123"),
      optional: [text(null), text(undefined)],
      interpolated: t("Moved to {{value0}}.", { value0: "07/11/2026" }),
      date: date("2026-11-07"),
      decimal: number(12.5, 1),
      locale: [resolveLocale("IT_it"), resolveLocale("nl-NL"), resolveLocale(undefined), resolveLocale("constructor")],
      name: planName({ name: "My unchanged custom plan" }),
    };
  });
  expect(result.missing).toBe("New untranslated English copy");
  expect(result.unknown).toBe("Provider diagnostic code ZYX-123");
  expect(result.optional).toEqual([null, undefined]);
  expect(result.interpolated).toContain("07/11/2026");
  expect(result.interpolated).not.toContain("{{");
  expect(result.date).toBe("07/11/2026");
  expect(result.decimal).toBe("12,5");
  expect(result.locale).toEqual(["it", "en", "en", "en"]);
  expect(result.name).toBe("My unchanged custom plan");
});

test("date-only values keep their day west of UTC", async ({ browser }) => {
  const context = await browser.newContext({ locale: "it-IT", timezoneId: "America/Los_Angeles" });
  const page = await context.newPage();
  await installDeskFixture(page);
  await page.goto("/desk.html#settings");
  expect(await page.evaluate(async () => (await import("/src/desk/i18n.ts")).date("2026-11-07"))).toBe("07/11/2026");
  await context.close();
});

for (const language of ["de", "it", "ja", "zh"]) for (const width of [320, 390, 1440]) {
  test(`localized settings layout ${language} ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await installDeskFixture(page);
    await page.addInitScript(language => {
      Object.defineProperty(navigator, "language", { configurable: true, value: language });
    }, language);
    await page.goto("/desk.html#settings");
    await expect(page.locator(".settings-directory")).toBeVisible();
    expect(await page.evaluate(() => {
      const workspace = document.getElementById("workspace");
      return document.documentElement.scrollWidth <= innerWidth && workspace.scrollWidth <= workspace.clientWidth + 1;
    })).toBe(true);
    await page.screenshot({ path: test.info().outputPath(`settings-${language}-${width}.png`), fullPage: true });
    await page.goto("/desk.html#health/overview");
    await expect(page.locator(".health-verdict")).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  });
}