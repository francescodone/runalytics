# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> motion system >> semantic reveals and compact presses no-preference 1440px
- Location: tests/desk.spec.mjs:196:7

# Error details

```
Error: expect(received).toBe(expected) // Object.is equality

Expected: 22
Received: 23
```

# Page snapshot

```yaml
- generic [ref=e2]:
  - link "Skip to workspace" [ref=e3] [cursor=pointer]:
    - /url: "#workspace"
  - banner [ref=e4]:
    - link "Runalytics training" [ref=e5] [cursor=pointer]:
      - /url: "#training"
      - generic [ref=e11]: Runalytics
    - navigation "Workspaces" [ref=e12]:
      - link "Training" [ref=e13] [cursor=pointer]:
        - /url: "#training"
      - link "Activities" [ref=e17] [cursor=pointer]:
        - /url: "#journal"
      - link "Health" [ref=e24] [cursor=pointer]:
        - /url: "#health"
    - generic [ref=e28]:
      - button "Sync activities" [ref=e29] [cursor=pointer]
      - button "Help" [ref=e34] [cursor=pointer]
      - button "Settings" [pressed] [ref=e39] [cursor=pointer]
  - main [ref=e44]:
    - generic [ref=e45]:
      - heading "Settings" [level=1] [ref=e46]
      - generic [ref=e47]: COROS connected
    - generic [ref=e50]:
      - button "All settings" [ref=e51] [cursor=pointer]
      - heading "Offline maps" [level=2] [ref=e59]
      - generic [ref=e60]:
        - generic [ref=e61]:
          - text: Raster PMTiles archive
          - textbox "Raster PMTiles archive" [active] [ref=e62]:
            - /placeholder: /path/to/region.pmtiles
            - text: /synthetic/new-map.pmtiles
        - paragraph [ref=e63]: Leave empty to use online OpenStreetMap tiles. Online tiles send map-area requests to the tile provider.
        - button "Save map source" [ref=e65] [cursor=pointer]
```

# Test source

```ts
  151 |         const dialog = page.getByRole("dialog", { name: "Log pain" });
  152 |         await expect(dialog).toHaveCSS("backdrop-filter", "blur(24px) saturate(1.45)");
  153 |         await expect.poll(() => dialog.evaluate(element => getComputedStyle(element, "::backdrop").backdropFilter)).toBe("blur(12px)");
  154 |         // Axe samples the painted pixels: measuring while the dialog is still
  155 |         // fading in blends its colours with the page behind and invents a
  156 |         // contrast failure that never exists once the arrival animation ends.
  157 |         await expect.poll(() => page.evaluate(() => document.getAnimations().filter(animation => animation.playState === "running").length)).toBe(0);
  158 |         expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
  159 |         await page.screenshot({ path: `test-results/glass-${theme}-${width}.png`, animations: "disabled" });
  160 |         await page.emulateMedia({ contrast: "more" });
  161 |         await expect(dialog).toHaveCSS("backdrop-filter", "none");
  162 |         await expect.poll(() => dialog.evaluate(element => getComputedStyle(element, "::backdrop").backdropFilter)).toBe("none");
  163 |         for (const selector of [".desk-toolbar", ".health-verdict", ".health-analysis"]) await expect(page.locator(selector)).toHaveCSS("backdrop-filter", "none");
  164 |         const surface = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--desk-surface").trim());
  165 |         const opaqueColor = await page.evaluate(value => { const probe = document.createElement("span"); probe.style.color = value; document.body.append(probe); const color = getComputedStyle(probe).color; probe.remove(); return color; }, surface);
  166 |         await expect(dialog).toHaveCSS("background-color", opaqueColor);
  167 |         await page.emulateMedia({ contrast: "no-preference" });
  168 |         const session = await page.context().newCDPSession(page);
  169 |         await session.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-transparency", value: "reduce" }] });
  170 |         await expect(dialog).toHaveCSS("backdrop-filter", "none");
  171 |         await expect(dialog).toHaveCSS("background-color", opaqueColor);
  172 |         await session.send("Emulation.setEmulatedMedia", { features: [] });
  173 |         await session.detach();
  174 |         const removed = await page.evaluate(() => {
  175 |           let count = 0;
  176 |           for (const sheet of document.styleSheets) {
  177 |             for (let index = sheet.cssRules.length - 1; index >= 0; index--) {
  178 |               const rule = sheet.cssRules[index];
  179 |               if (rule instanceof CSSSupportsRule && rule.conditionText.includes("backdrop-filter")) { sheet.deleteRule(index); count++; }
  180 |             }
  181 |           }
  182 |           return count;
  183 |         });
  184 |         expect(removed).toBeGreaterThan(0);
  185 |         await expect(dialog).toHaveCSS("backdrop-filter", "none");
  186 |         await expect(dialog).toHaveCSS("background-color", opaqueColor);
  187 |         await page.keyboard.press("Escape");
  188 |         await expect(dialog).toHaveCount(0);
  189 |         expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  190 |       });
  191 |     }
  192 |   }
  193 | 
  194 |   for (const width of [390, 1440]) {
  195 |     for (const reducedMotion of ["no-preference", "reduce"]) {
  196 |       test(`semantic reveals and compact presses ${reducedMotion} ${width}px`, async ({ page }) => {
  197 |         await page.setViewportSize({ width, height: 900 });
  198 |         await page.emulateMedia({ reducedMotion });
  199 |         await page.addInitScript(() => {
  200 |           window.__contentReveals = [];
  201 |           for (const type of ["animationstart", "animationend"]) {
  202 |             document.addEventListener(type, event => {
  203 |               if (event.animationName !== "content-arrive") return;
  204 |               const bounds = event.target.getBoundingClientRect();
  205 |               window.__contentReveals.push({ type, className: event.target.className, width: bounds.width, height: bounds.height, x: bounds.x, y: bounds.y, elapsed: event.elapsedTime, filter: getComputedStyle(event.target).filter });
  206 |             });
  207 |           }
  208 |         });
  209 |         await page.goto("/desk.html#health/overview");
  210 |         const verdict = page.locator(".health-verdict");
  211 |         await expect(verdict).toBeVisible();
  212 |         if (reducedMotion === "no-preference") {
  213 |           await expect.poll(() => page.evaluate(() => window.__contentReveals.filter(event => event.className === "health-verdict" && event.type === "animationend").length)).toBe(1);
  214 |           const events = await page.evaluate(() => window.__contentReveals.filter(event => event.className === "health-verdict"));
  215 |           expect(events[1].width).toBeCloseTo(events[0].width, 3);
  216 |           expect(events[1].height).toBeCloseTo(events[0].height, 3);
  217 |           expect(events[1].elapsed).toBe(.32);
  218 |           expect(events[0].filter).toMatch(/^blur\(/);
  219 |           expect(parseFloat(events[0].filter.slice(5))).toBeGreaterThan(0);
  220 |           await expect(verdict).toHaveAttribute("data-reveal", "settled");
  221 |           await expect(verdict).toHaveCSS("filter", "none");
  222 |           await verdict.hover();
  223 |           await page.mouse.move(0, 0);
  224 |           await expect(verdict).toHaveCSS("animation-name", "none");
  225 |         } else {
  226 |           expect(await page.evaluate(() => window.__contentReveals)).toEqual([]);
  227 |           await expect(verdict).toHaveCSS("animation-name", "none");
  228 |         }
  229 |         await page.getByRole("link", { name: "Pace insights", exact: true }).click();
  230 |         const selector = page.getByRole("button", { name: "Long", exact: true });
  231 |         await expect(selector).toBeVisible();
  232 |         const bounds = await selector.boundingBox();
  233 |         await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  234 |         await page.mouse.down();
  235 |         await expect.poll(() => selector.evaluate(element => getComputedStyle(element).scale)).toBe(reducedMotion === "reduce" ? "1" : "0.98");
  236 |         expect(await selector.evaluate(element => element.offsetWidth)).toBe(Math.round(bounds.width));
  237 |         await page.mouse.up();
  238 |         await expect(selector).toHaveCSS("scale", reducedMotion === "reduce" ? "none" : "1.015");
  239 |         await page.mouse.move(0, 0);
  240 |         await expect(selector).toHaveCSS("scale", "none");
  241 |         await expect(selector).toHaveAttribute("aria-pressed", "true");
  242 |         await page.goto("/desk.html#settings");
  243 |         await page.getByRole("link", { name: /Offline maps/ }).click();
  244 |         const editor = page.locator(".settings-editor");
  245 |         await expect(editor).toBeVisible();
  246 |         if (reducedMotion === "no-preference") {
  247 |           await expect.poll(() => editor.locator("header").getAttribute("data-reveal")).toBe("settled");
  248 |         }
  249 |         const before = await page.evaluate(() => window.__contentReveals.length);
  250 |         await page.getByLabel("Raster PMTiles archive").fill("/synthetic/new-map.pmtiles");
> 251 |         expect(await page.evaluate(() => window.__contentReveals.length)).toBe(before);
      |                                                                           ^ Error: expect(received).toBe(expected) // Object.is equality
  252 |         expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  253 |         await page.screenshot({ path: `test-results/micro-motion-${reducedMotion}-${width}.png`, fullPage: true });
  254 |       });
  255 |     }
  256 |   }
  257 | 
  258 |   test("viewport reveals wait for entry, run once, and settle on focus or preference changes", async ({ page }) => {
  259 |     await page.setViewportSize({ width: 390, height: 600 });
  260 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  261 |     await page.goto("/desk.html#settings");
  262 |     const lastTask = page.locator(".settings-task-list > a").last();
  263 |     await expect(lastTask).toHaveAttribute("data-reveal", "waiting");
  264 |     await expect(lastTask).toHaveCSS("opacity", "0");
  265 |     await lastTask.evaluate(element => {
  266 |       window.__viewportReveals = [];
  267 |       document.addEventListener("animationstart", event => {
  268 |         if (event.target === element) window.__viewportReveals.push(event.animationName);
  269 |       });
  270 |       element.scrollIntoView({ block: "center" });
  271 |     });
  272 |     await expect(lastTask).toHaveAttribute("data-reveal", "settled");
  273 |     await expect(lastTask).toHaveCSS("filter", "none");
  274 |     await expect(lastTask).toHaveCSS("opacity", "1");
  275 |     await page.locator("#workspace").evaluate(element => element.scrollTo(0, 0));
  276 |     await expect(page.locator(".workspace-heading")).toBeInViewport();
  277 |     expect(await page.evaluate(() => document.documentElement.scrollHeight <= innerHeight && scrollY === 0)).toBe(true);
  278 |     await lastTask.evaluate(element => element.scrollIntoView({ block: "center" }));
  279 |     expect(await page.evaluate(() => window.__viewportReveals)).toEqual(["content-arrive"]);
  280 | 
  281 |     await page.locator("#workspace").evaluate(element => element.scrollTo(0, 0));
  282 |     await page.reload();
  283 |     await expect(lastTask).toHaveAttribute("data-reveal", "waiting");
  284 |     await lastTask.evaluate(element => element.focus({ preventScroll: true }));
  285 |     await expect(lastTask).toBeFocused();
  286 |     await expect(lastTask).toHaveAttribute("data-reveal", "settled");
  287 |     await expect(lastTask).toHaveCSS("opacity", "1");
  288 |     await expect(lastTask).toHaveCSS("filter", "none");
  289 |     await expect(lastTask).toHaveCSS("animation-name", "none");
  290 | 
  291 |     await page.locator("#workspace").evaluate(element => element.scrollTo(0, 0));
  292 |     await page.reload();
  293 |     await expect(lastTask).toHaveAttribute("data-reveal", "waiting");
  294 |     await page.emulateMedia({ reducedMotion: "reduce" });
  295 |     await expect(page.locator('[data-reveal="waiting"], [data-reveal="visible"]')).toHaveCount(0);
  296 |     await expect(lastTask).toHaveCSS("opacity", "1");
  297 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  298 |     await lastTask.evaluate(element => element.scrollIntoView({ block: "center" }));
  299 |     await expect(lastTask).toHaveCSS("animation-name", "none");
  300 |     expect(await page.evaluate(() => document.querySelectorAll('[data-reveal]:not([data-reveal="container"]) [data-reveal]:not([data-reveal="container"])').length)).toBe(0);
  301 |   });
  302 | 
  303 |   for (const reducedMotion of ["no-preference", "reduce"]) {
  304 |     test(`scatter exits retain content without transforming fixed ancestors ${reducedMotion}`, async ({ page }) => {
  305 |       await page.setViewportSize({ width: 390, height: 900 });
  306 |       await page.emulateMedia({ reducedMotion });
  307 |       await page.addInitScript(() => {
  308 |         window.__scatterExits = [];
  309 |         document.addEventListener("animationstart", event => {
  310 |           if (event.animationName !== "content-depart") return;
  311 |           const host = event.target.closest('.motion-change[data-transition-state="exiting"]');
  312 |           if (!host) return;
  313 |           const effect = event.target.getAnimations().find(animation => animation.animationName === "content-depart")?.effect;
  314 |           window.__scatterExits.push({ connected: host.isConnected, inert: host.inert, filter: getComputedStyle(host).filter, transform: getComputedStyle(host).transform, frames: effect?.getKeyframes() });
  315 |         });
  316 |       });
  317 |       await page.goto("/desk.html#health");
  318 |       await expect(page.locator(".health-verdict")).toHaveAttribute("data-reveal", "settled");
  319 |       await page.getByRole("link", { name: "Activities", exact: true }).click();
  320 |       await expect(page.locator(".activity-library")).toBeVisible();
  321 |       await expect(page.locator('[data-transition-state="exiting"]')).toHaveCount(0);
  322 |       const exits = await page.evaluate(() => window.__scatterExits);
  323 |       if (reducedMotion === "reduce") {
  324 |         expect(exits).toEqual([]);
  325 |       } else {
  326 |         expect(exits.length).toBeGreaterThan(1);
  327 |         for (const exit of exits) {
  328 |           expect(exit).toMatchObject({ connected: true, inert: true, filter: "none", transform: "none" });
  329 |           expect(exit.frames.at(-1).filter).toBe("blur(6px)");
  330 |         }
  331 |         expect(new Set(exits.map(exit => exit.frames.at(-1).translate)).size).toBeGreaterThan(1);
  332 |       }
  333 |       const navigation = await page.locator(".desk-navigation").boundingBox();
  334 |       expect(navigation.x + navigation.width / 2).toBeCloseTo(195, 1);
  335 |       expect(navigation.y + navigation.height).toBeCloseTo(888, 1);
  336 |     });
  337 |   }
  338 | 
  339 |   for (const width of [390, 1440]) {
  340 |     for (const theme of ["light", "dark"]) {
  341 |       test(`complete motion-enabled workspace pass ${theme} ${width}px`, async ({ page }) => {
  342 |         test.setTimeout(90000);
  343 |         await page.setViewportSize({ width, height: 1000 });
  344 |         await page.emulateMedia({ reducedMotion: "no-preference" });
  345 |         const errors = [];
  346 |         page.on("pageerror", error => errors.push(error.message));
  347 |         for (const route of ["training", "training/month", "training/plan", "journal", "health", "health/pace", "health/journal", "health/milestones", "settings", "settings/appearance", "help"]) {
  348 |           await page.goto(`/desk.html?theme=${theme}#${route}`);
  349 |           await expect(page.locator("#workspace h1")).toBeVisible();
  350 |           await expect(page.locator(".desk-loading")).toHaveCount(0);
  351 |           expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), route).toBe(true);
```