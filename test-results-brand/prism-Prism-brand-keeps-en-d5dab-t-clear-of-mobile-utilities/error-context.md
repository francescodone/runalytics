# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: prism.spec.mjs >> Prism brand keeps enlarged text clear of mobile utilities
- Location: tests/prism.spec.mjs:47:1

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#training
Call log:
  - navigating to "http://127.0.0.1:1500/desk.html#training", waiting until "load"

```

# Page snapshot

```yaml
- generic [ref=e3]:
  - generic [ref=e6]:
    - heading "This site can’t be reached" [level=1] [ref=e7]
    - paragraph [ref=e8]:
      - strong [ref=e9]: 127.0.0.1
      - text: refused to connect.
    - generic [ref=e10]:
      - paragraph [ref=e11]: "Try:"
      - list [ref=e12]:
        - listitem [ref=e13]: Checking the connection
        - listitem [ref=e14]:
          - link "Checking the proxy and the firewall" [ref=e15] [cursor=pointer]:
            - /url: "#buttons"
    - generic [ref=e16]: ERR_CONNECTION_REFUSED
  - generic [ref=e17]:
    - button "Reload" [ref=e19] [cursor=pointer]
    - button "Details" [ref=e20] [cursor=pointer]
```

# Test source

```ts
  1   | import { test, expect } from "@playwright/test";
  2   | import AxeBuilder from "@axe-core/playwright";
  3   | import { installDeskFixture } from "./desk.fixture.mjs";
  4   | 
  5   | test.beforeEach(async ({ page }) => installDeskFixture(page));
  6   | 
  7   | for (const width of [320, 375, 390, 820, 1440]) {
  8   |   test(`Prism brand stays centered and follows every accent at ${width}px`, async ({ page }, testInfo) => {
  9   |     await page.setViewportSize({ width, height: 900 });
  10  |     await page.goto("/desk.html#training");
  11  |     const brand = page.getByRole("link", { name: "Runalytics training" });
  12  |     const icon = brand.locator(".desk-brand-icon");
  13  |     await expect(icon).toBeVisible();
  14  |     await page.evaluate(() => document.fonts.ready);
  15  |     const geometry = await brand.evaluate(element => {
  16  |       const bounds = element.getBoundingClientRect();
  17  |       const mark = element.querySelector(".desk-brand-icon").getBoundingClientRect();
  18  |       const name = element.querySelector(".desk-brand-name");
  19  |       const text = name.getBoundingClientRect();
  20  |       const textVisible = getComputedStyle(name).display !== "none";
  21  |       return { width: mark.width, height: mark.height, textVisible, centerOffset: (mark.left + (textVisible ? text.right : mark.right)) / 2 - (bounds.left + bounds.width / 2), verticalOffset: textVisible ? (mark.top + mark.height / 2) - (text.top + text.height / 2) : 0 };
  22  |     });
  23  |     expect(geometry.width).toBe(width > 800 ? 44 : 36);
  24  |     expect(geometry.height).toBe(geometry.width);
  25  |     expect(geometry.textVisible).toBe(width === 375 || width === 390 || width === 1440);
  26  |     expect(Math.abs(geometry.centerOffset)).toBeLessThan(1);
  27  |     expect(Math.abs(geometry.verticalOffset)).toBeLessThan(1);
  28  |     await assertLayout(page, `brand ${width}`);
  29  |     for (const theme of ["light", "dark"]) {
  30  |       for (const palette of ["reference", "amethyst", "glacier", "evergreen", "sakura", "ember"]) {
  31  |         const colors = await icon.evaluate((element, appearance) => {
  32  |           document.documentElement.dataset.deskTheme = appearance.theme;
  33  |           document.documentElement.dataset.deskPalette = appearance.palette;
  34  |           const context = document.createElement("canvas").getContext("2d");
  35  |           const normalized = color => { context.fillStyle = color; return context.fillStyle; };
  36  |           const style = getComputedStyle(element);
  37  |           return { accent: normalized(style.getPropertyValue("--desk-action").trim()), foreground: normalized(style.getPropertyValue("--desk-on-action").trim()), paths: [...element.querySelectorAll("path")].map(path => normalized(getComputedStyle(path).fill)), tile: normalized(getComputedStyle(element.querySelector("rect")).fill) };
  38  |         }, { theme, palette });
  39  |         expect(colors.paths, `${theme}/${palette}`).toEqual([colors.accent, colors.accent]);
  40  |         expect(colors.tile, `${theme}/${palette}`).toBe(colors.foreground);
  41  |       }
  42  |       await page.screenshot({ path: testInfo.outputPath(`brand-${theme}.png`) });
  43  |     }
  44  |   });
  45  | }
  46  | 
  47  | test("Prism brand keeps enlarged text clear of mobile utilities", async ({ page }) => {
  48  |   for (const width of [375, 390]) {
  49  |     await page.setViewportSize({ width, height: 900 });
> 50  |     await page.goto("/desk.html#training");
      |                ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#training
  51  |     await page.addStyleTag({ content: ":root { font-size: 200%; }" });
  52  |     await expect(page.locator(".desk-brand-name")).toBeHidden();
  53  |     await expect(page.locator(".desk-brand-icon")).toBeVisible();
  54  |     const icon = await page.locator(".desk-brand-icon").boundingBox();
  55  |     const utilities = await page.locator(".desk-utilities").boundingBox();
  56  |     expect(icon.x + icon.width).toBeLessThan(utilities.x);
  57  |     await assertLayout(page, `brand enlarged text ${width}`);
  58  |   }
  59  | });
  60  | 
  61  | for (const reducedMotion of ["no-preference", "reduce"]) {
  62  |   test(`Prism brand entrance runs once with ${reducedMotion} motion`, async ({ page }) => {
  63  |     await page.setViewportSize({ width: 1440, height: 900 });
  64  |     await page.emulateMedia({ reducedMotion });
  65  |     await page.addInitScript(() => {
  66  |       window.__brandEntrances = [];
  67  |       document.addEventListener("animationstart", event => {
  68  |         if (event.target.matches(".desk-brand-icon, .desk-brand-name")) window.__brandEntrances.push({ target: event.target.className, animation: event.animationName, delay: getComputedStyle(event.target).animationDelay });
  69  |       });
  70  |     });
  71  |     await page.goto("/desk.html#journal");
  72  |     const brand = page.getByRole("link", { name: "Runalytics training" });
  73  |     await expect(brand.locator(".desk-brand-icon")).toHaveAttribute("data-reveal", "settled");
  74  |     await expect(brand.locator(".desk-brand-name")).toHaveAttribute("data-reveal", "settled");
  75  |     const entrances = await page.evaluate(() => window.__brandEntrances);
  76  |     if (reducedMotion === "reduce") expect(entrances).toEqual([]);
  77  |     else {
  78  |       expect(entrances.map(entry => entry.target)).toEqual(["desk-brand-icon", "desk-brand-name"]);
  79  |       expect(entrances.every(entry => entry.animation === "content-arrive")).toBe(true);
  80  |       expect(parseFloat(entrances[1].delay)).toBeGreaterThan(parseFloat(entrances[0].delay));
  81  |     }
  82  |     await page.keyboard.press("Tab");
  83  |     await page.keyboard.press("Tab");
  84  |     await expect(brand).toBeFocused();
  85  |     await page.keyboard.press("Enter");
  86  |     await expect(page.locator(".workspace-heading h1")).toHaveText("Training");
  87  |     await page.getByRole("link", { name: "Activities", exact: true }).click();
  88  |     await expect(page.locator(".workspace-heading h1")).toHaveText("Activities");
  89  |     expect(await page.evaluate(() => window.__brandEntrances)).toEqual(entrances);
  90  |     expect(await brand.evaluate(element => element.getAnimations({ subtree: true }).filter(animation => animation.playState === "running").length)).toBe(0);
  91  |   });
  92  | }
  93  | 
  94  | for (const [first, current, label] of [[60, 70, "improving"], [80, 70, "declining"], [70, 70, "steady"], [null, 70, "history"]]) {
  95  |   test(`Prism readiness summary ${label}`, async ({ page }) => {
  96  |     await page.goto("/desk.html#training");
  97  |     await page.evaluate(({ first, current }) => {
  98  |       const invoke = window.__TAURI_INTERNALS__.invoke;
  99  |       window.__TAURI_INTERNALS__.invoke = async (command, args) => {
  100 |         const result = await invoke(command, args);
  101 |         if (command === "dashboard") {
  102 |           result.scores.globalScore = current;
  103 |           result.scoreHistory = first == null ? [] : [{ date: "2026-10-01", readiness: first }, { date: "2026-10-02", readiness: current }];
  104 |         }
  105 |         return result;
  106 |       };
  107 |     }, { first, current });
  108 |     await page.getByRole("link", { name: "Health", exact: true }).click();
  109 |     await expect(page.getByRole("region", { name: "Readiness trend" }).getByRole("heading")).toContainText(label);
  110 |     await expect(page.locator(".trend-chart:visible")).toHaveCount(1);
  111 |     await expect(page.locator("#health-details")).toBeHidden();
  112 |     await page.getByRole("button", { name: "Show health details" }).click();
  113 |     await expect(page.getByRole("heading", { name: "Risk contributors" })).toBeVisible();
  114 |     await page.getByRole("button", { name: "Hide health details" }).click();
  115 |     await expect(page.locator("#health-details")).toBeHidden();
  116 |   });
  117 | }
  118 | 
  119 | for (const width of [320, 820, 1440]) {
  120 |   test(`Prism centered milestones ${width}px`, async ({ page }) => {
  121 |     await page.setViewportSize({ width, height: 900 });
  122 |     await page.goto("/desk.html#health/milestones");
  123 |     await expect(page.locator(".milestone-list article").first()).toBeVisible();
  124 |     const layout = await page.locator(".milestones").evaluate(element => {
  125 |       const bounds = element.getBoundingClientRect();
  126 |       const grid = element.querySelector(".consistency-grid").getBoundingClientRect();
  127 |       const cell = element.querySelector(".consistency-grid > div").getBoundingClientRect();
  128 |       return { center: bounds.x + bounds.width / 2, gridCenter: grid.x + grid.width / 2, cellWidth: cell.width, cellHeight: cell.height, overflow: element.scrollWidth - element.clientWidth };
  129 |     });
  130 |     expect(Math.abs(layout.center - layout.gridCenter)).toBeLessThan(1);
  131 |     expect(Math.abs(layout.cellWidth - layout.cellHeight)).toBeLessThan(1);
  132 |     expect(layout.overflow).toBeLessThanOrEqual(1);
  133 |     await page.getByRole("checkbox", { name: "Earned only" }).check();
  134 |     await expect(page.locator(".milestone-list article:not(.earned)")).toHaveCount(0);
  135 |   });
  136 | }
  137 | 
  138 | for (const reducedMotion of ["reduce", "no-preference"]) {
  139 |   test(`Prism activity switch settles correctly ${reducedMotion}`, async ({ page }) => {
  140 |     await page.setViewportSize({ width: 1440, height: 900 });
  141 |     await page.emulateMedia({ reducedMotion });
  142 |     await page.goto("/desk.html#journal/1");
  143 |     await expect(page.locator(".activity-review h2")).toBeVisible();
  144 |     await expect(page.locator(".activity-transition .route-figure")).toHaveAttribute("data-reveal", "settled");
  145 |     await page.evaluate(() => {
  146 |       window.__activityDepartures = [];
  147 |       document.addEventListener("animationstart", event => {
  148 |         if (event.animationName === "content-depart" && event.target.closest(".activity-transition")) window.__activityDepartures.push(getComputedStyle(event.target).animationDelay);
  149 |       });
  150 |     });
```