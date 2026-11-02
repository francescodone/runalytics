# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: prism.spec.mjs >> Prism route matrix light 360px
- Location: tests/prism.spec.mjs:308:5

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html?theme=light#training
Call log:
  - navigating to "http://127.0.0.1:1500/desk.html?theme=light#training", waiting until "load"

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
  212 |   for (const width of [820, 1180, 1440]) {
  213 |     test(`Prism today marker and compact navigation ${theme} ${width}px`, async ({ page }, testInfo) => {
  214 |       await page.setViewportSize({ width, height: 900 });
  215 |       await page.goto(`/desk.html?theme=${theme}#training/week`);
  216 |       const today = page.locator(".day-lane.is-today");
  217 |       await expect(today).toBeVisible();
  218 |       const days = page.locator(".week-board > .day-lane");
  219 |       await expect(days).toHaveCount(7);
  220 |       const heights = await days.evaluateAll(elements => elements.map(element => element.getBoundingClientRect().height));
  221 |       expect(Math.max(...heights) - Math.min(...heights)).toBeLessThan(1);
  222 |       await expect(today).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  223 |       const circle = today.locator("header strong");
  224 |       await expect(circle).toHaveCSS("border-radius", "50%");
  225 |       await expect(circle).not.toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  226 |       const navigation = page.getByRole("navigation", { name: "Workspaces" });
  227 |       for (const label of ["Training", "Activities", "Health"]) {
  228 |         const link = navigation.getByRole("link", { name: label, exact: true });
  229 |         await expect(link).toBeVisible();
  230 |         const dimensions = await link.evaluate(element => ({ width: element.offsetWidth, height: element.offsetHeight }));
  231 |         if (width <= 1180) {
  232 |           expect(dimensions.width).toBe(dimensions.height);
  233 |           expect(dimensions.width).toBeGreaterThanOrEqual(44);
  234 |           await expect(link.locator("span:not(.selection-marker)")).toBeHidden();
  235 |         } else {
  236 |           await expect(link.locator("span:not(.selection-marker)")).toBeVisible();
  237 |         }
  238 |       }
  239 |       await page.screenshot({ path: testInfo.outputPath("week-navigation.png") });
  240 |       await navigation.getByRole("link", { name: "Activities", exact: true }).focus();
  241 |       await page.keyboard.press("Enter");
  242 |       await expect(page.getByRole("heading", { name: "Activities", exact: true })).toBeVisible();
  243 |     });
  244 |   }
  245 | 
  246 |   for (const viewport of [{ width: 320, height: 740 }, { width: 390, height: 844 }, { width: 820, height: 900 }, { width: 913, height: 480 }, { width: 1440, height: 900 }, { width: 820, height: 480, enlarged: true }]) {
  247 |     test(`Prism full-height activity filters ${theme} ${viewport.width}x${viewport.height}${viewport.enlarged ? " enlarged" : ""}`, async ({ page }, testInfo) => {
  248 |       await page.setViewportSize({ width: viewport.width, height: viewport.height });
  249 |       await page.goto(`/desk.html?theme=${theme}#journal`);
  250 |       if (viewport.enlarged) await page.addStyleTag({ content: ":root { font-size: 200%; }" });
  251 |       const filters = page.locator("#activity-filters");
  252 |       await expect(filters).toBeHidden();
  253 |       await page.getByRole("button", { name: "Filters", exact: true }).click();
  254 |       await expect(filters).toBeVisible();
  255 |       await expect(filters).toHaveCSS("overflow-y", "visible");
  256 |       const geometry = await filters.evaluate(element => ({ height: element.clientHeight, scrollHeight: element.scrollHeight, pageOverflow: document.documentElement.scrollWidth - innerWidth }));
  257 |       expect(geometry.scrollHeight).toBeLessThanOrEqual(geometry.height + 1);
  258 |       expect(geometry.pageOverflow).toBeLessThanOrEqual(1);
  259 |       await page.screenshot({ path: testInfo.outputPath("expanded-filters.png") });
  260 |       await filters.getByRole("combobox", { name: "Sport", exact: true }).selectOption("Run");
  261 |       await expect(page.locator(".activity-row")).toHaveCount(6);
  262 |       const reset = filters.getByRole("button", { name: "Reset filters", exact: true });
  263 |       await reset.scrollIntoViewIfNeeded();
  264 |       await expect(reset).toBeInViewport();
  265 |       await reset.click();
  266 |       await expect(page.locator(".activity-row")).toHaveCount(9);
  267 |       await page.getByRole("button", { name: "Hide filters", exact: true }).click();
  268 |       await expect(filters).toBeHidden();
  269 |     });
  270 |   }
  271 | }
  272 | 
  273 | test("Prism preview uses native date shapes and populated initial progress", async ({ page }) => {
  274 |   await page.goto("/desk.html#training");
  275 |   await expect(page.locator(".session-tile:visible").first()).toBeVisible();
  276 |   await expect(page.locator(".training-context")).not.toContainText("Invalid Date");
  277 |   const fixture = await page.evaluate(async () => {
  278 |     const plan = await window.__TAURI_INTERNALS__.invoke("current_plan");
  279 |     const progress = await window.__TAURI_INTERNALS__.invoke("plan_week_progress");
  280 |     return { target: plan[0].targetDate, planned: progress.reduce((total, week) => total + week.plannedM, 0), workouts: plan[1].reduce((total, workout) => total + workout.distanceM, 0) };
  281 |   });
  282 |   expect(fixture.target).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  283 |   expect(fixture.planned).toBeGreaterThan(0);
  284 |   expect(fixture.planned).toBe(fixture.workouts);
  285 |   await expect(page.locator(".schedule-summary")).not.toContainText("/ 0.0 km");
  286 | });
  287 | 
  288 | const routes = ["training", "training/month", "training/plan", "journal", "health", "health/pace", "health/journal", "health/milestones", "settings", "help"];
  289 | const widths = [320, 360, 375, 390, 414, 430, 600, 768, 820, 1024, 1280, 1366, 1440, 1600, 1920, 2560];
  290 | 
  291 | async function assertLayout(page, context) {
  292 |   await expect(page.locator(".workspace-heading h1")).toBeVisible();
  293 |   await expect(page.locator(".desk-loading")).toHaveCount(0);
  294 |   await page.evaluate(() => document.fonts.ready);
  295 |   const geometry = await page.evaluate(() => ({
  296 |     pageOverflow: document.documentElement.scrollWidth - innerWidth,
  297 |     overflowingText: [...document.querySelectorAll("button, a, h1, h2, h3, p, summary, label")].filter(element => {
  298 |       if (!element.checkVisibility() || element.clientWidth === 0 || element.matches(".icon-button") || element.closest(".sr-only, .data-table-scroll, .review-tabs, .horizon-switch, .pace-session-picker, .lap-chart-scroll")) return false;
  299 |       return element.scrollWidth > element.clientWidth + 2;
  300 |     }).map(element => `${element.className || element.tagName}: ${element.textContent.trim().slice(0, 80)}`),
  301 |   }));
  302 |   expect(geometry.pageOverflow, context).toBeLessThanOrEqual(1);
  303 |   expect(geometry.overflowingText, context).toEqual([]);
  304 | }
  305 | 
  306 | for (const theme of ["light", "dark"]) {
  307 |   for (const width of widths) {
  308 |     test(`Prism route matrix ${theme} ${width}px`, async ({ page }, testInfo) => {
  309 |       test.setTimeout(90000);
  310 |       await page.setViewportSize({ width, height: 960 });
  311 |       for (const route of routes) {
> 312 |         await page.goto(`/desk.html?theme=${theme}#${route}`);
      |                    ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html?theme=light#training
  313 |         await assertLayout(page, `${route} ${theme} ${width}`);
  314 |         if ([320, 820, 1440, 2560].includes(width)) await page.screenshot({ path: testInfo.outputPath(`${route.replaceAll("/", "-")}.png`), fullPage: true });
  315 |       }
  316 |     });
  317 |   }
  318 | }
  319 | 
  320 | for (const viewport of [{ width: 537, height: 740 }, { width: 913, height: 480 }, { width: 667, height: 375 }]) {
  321 |   test(`Prism intermediate and short viewport ${viewport.width}x${viewport.height}`, async ({ page }) => {
  322 |     test.setTimeout(60000);
  323 |     await page.setViewportSize(viewport);
  324 |     for (const route of routes) {
  325 |       await page.goto(`/desk.html#${route}`);
  326 |       await assertLayout(page, route);
  327 |     }
  328 |   });
  329 | }
  330 | 
  331 | for (const theme of ["light", "dark"]) {
  332 |   test(`Prism detail views and settings at rail breakpoint ${theme}`, async ({ page }, testInfo) => {
  333 |     test.setTimeout(90000);
  334 |     await page.setViewportSize({ width: 820, height: 960 });
  335 |     for (const route of ["journal/1", "journal/1/execution", "journal/1/splits", "journal/1/context", ...["watch", "appearance", "thresholds", "training", "calendar", "assistant", "maps", "about"].map(section => `settings/${section}`)]) {
  336 |       await page.goto(`/desk.html?theme=${theme}#${route}`);
  337 |       await assertLayout(page, `${route} ${theme}`);
  338 |       if (route.startsWith("settings")) {
  339 |         await expect(page.getByRole("button", { name: "Settings", exact: true })).toHaveAttribute("aria-pressed", "true");
  340 |         for (const detail of await page.locator(".settings-editor details:not([open])").all()) await detail.locator(":scope > summary").click();
  341 |       }
  342 |       expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations, route).toEqual([]);
  343 |       await page.screenshot({ path: testInfo.outputPath(`${route.replaceAll("/", "-")}.png`), fullPage: true });
  344 |     }
  345 |   });
  346 | 
  347 |   test(`Prism settled rendering budget ${theme}`, async ({ page }, testInfo) => {
  348 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  349 |     await page.setViewportSize({ width: 1440, height: 960 });
  350 |     await page.goto(`/desk.html?theme=${theme}#training`);
  351 |     await expect(page.locator(".session-tile:visible").first()).toBeVisible();
  352 |     await page.waitForFunction(() => {
  353 |       const running = document.getAnimations().some(animation => animation.playState === "running");
  354 |       window.__prismQuiet = running ? 0 : (window.__prismQuiet ?? 0) + 1;
  355 |       return window.__prismQuiet >= 5;
  356 |     }, null, { polling: 100 });
  357 |     const rendering = await page.evaluate(() => {
  358 |       const filtered = [...document.querySelectorAll("body *")].flatMap(element => {
  359 |         if (!element.checkVisibility()) return [];
  360 |         return [null, "::before", "::after"].filter(pseudo => {
  361 |           const style = getComputedStyle(element, pseudo);
  362 |           return style.backdropFilter !== "none" && (!pseudo || !["none", "normal"].includes(style.content));
  363 |         }).map(pseudo => `${element.className}${pseudo ?? ""}`);
  364 |       });
  365 |       return { filtered, nodes: document.querySelectorAll("*").length, animations: document.getAnimations().filter(animation => animation.playState === "running").length, fontRequests: performance.getEntriesByType("resource").filter(resource => resource.name.endsWith("woff2")).map(resource => ({ url: new URL(resource.name).pathname, bytes: resource.transferSize })) };
  366 |     });
  367 |     expect(rendering.filtered.length).toBeLessThanOrEqual(3);
  368 |     expect(rendering.filtered.some(selector => /session-tile|activity-row|day-lane/.test(selector))).toBe(false);
  369 |     expect(rendering.animations).toBe(0);
  370 |     expect(rendering.fontRequests.some(resource => resource.url.includes("outfit"))).toBe(true);
  371 |     const screenshot = await page.screenshot({ path: testInfo.outputPath("settled.png") });
  372 |     const pixels = await page.evaluate(async source => {
  373 |       const image = new Image();
  374 |       image.src = source;
  375 |       await image.decode();
  376 |       const canvas = document.createElement("canvas");
  377 |       canvas.width = 144;
  378 |       canvas.height = 96;
  379 |       const context = canvas.getContext("2d");
  380 |       context.drawImage(image, 0, 0, 144, 96);
  381 |       const data = context.getImageData(0, 0, 144, 96).data;
  382 |       const colors = new Set();
  383 |       for (let offset = 0; offset < data.length; offset += 4) colors.add(`${data[offset] >> 4},${data[offset + 1] >> 4},${data[offset + 2] >> 4}`);
  384 |       return { width: image.width, height: image.height, distinctColorBuckets: colors.size };
  385 |     }, `data:image/png;base64,${screenshot.toString("base64")}`);
  386 |     expect(pixels.width).toBe(1440);
  387 |     expect(pixels.height).toBe(960);
  388 |     expect(pixels.distinctColorBuckets).toBeGreaterThan(20);
  389 |     await testInfo.attach("rendering-budget.json", { body: JSON.stringify({ ...rendering, ...pixels }, null, 2), contentType: "application/json" });
  390 |   });
  391 | }
  392 | 
  393 | for (const width of [320, 820, 1440]) {
  394 |   test(`Prism enlarged text ${width}px`, async ({ page }, testInfo) => {
  395 |     test.setTimeout(60000);
  396 |     await page.setViewportSize({ width, height: 960 });
  397 |     for (const route of ["training", "training/month", "settings/thresholds", "settings/appearance", "help"]) {
  398 |       await page.goto(`/desk.html#${route}`);
  399 |       await page.addStyleTag({ content: ":root { font-size: 200%; }" });
  400 |       await assertLayout(page, `200% text ${route} ${width}`);
  401 |       await page.screenshot({ path: testInfo.outputPath(`${route.replaceAll("/", "-")}.png`), fullPage: true });
  402 |     }
  403 |   });
  404 | }
  405 | 
  406 | test("Prism compact month selects a date and exposes its sessions", async ({ page }) => {
  407 |   await page.setViewportSize({ width: 320, height: 740 });
  408 |   await page.goto("/desk.html#training/month");
  409 |   await expect(page.locator(".month-number")).toHaveCount(42);
  410 |   const datedSession = await page.locator(".month-day").evaluateAll(days => days
  411 |     .filter(day => day.querySelector(".session-tile") && day.querySelector(".month-number").getAttribute("aria-pressed") !== "true")
  412 |     .map(day => ({ date: day.querySelector(".month-number").dataset.date, title: day.querySelector(".session-tile strong").textContent }))[0]);
```