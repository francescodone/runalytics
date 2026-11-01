# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> motion system >> complete motion-enabled workspace pass light 1440px
- Location: tests/desk.spec.mjs:176:7

# Error details

```
Error: settings

expect(received).toEqual(expected) // deep equality

- Expected  -  1
+ Received  + 58

- Array []
+ Array [
+   Object {
+     "description": "Ensure the contrast between foreground and background colors meets WCAG 2 AA minimum contrast ratio thresholds",
+     "help": "Elements must meet minimum color contrast ratio thresholds",
+     "helpUrl": "https://dequeuniversity.com/rules/axe/4.14/color-contrast?application=playwright",
+     "id": "color-contrast",
+     "impact": "serious",
+     "nodes": Array [
+       Object {
+         "all": Array [],
+         "any": Array [
+           Object {
+             "data": Object {
+               "bgColor": "#e3ecea",
+               "contrastRatio": 3.09,
+               "expectedContrastRatio": "4.5:1",
+               "fgColor": "#7c8781",
+               "fontSize": "12.0pt (16px)",
+               "fontWeight": "normal",
+               "messageKey": null,
+             },
+             "id": "color-contrast",
+             "impact": "serious",
+             "message": "Element has insufficient color contrast of 3.09 (foreground color: #7c8781, background color: #e3ecea, font size: 12.0pt (16px), font weight: normal). Expected contrast ratio of 4.5:1",
+             "relatedNodes": Array [
+               Object {
+                 "html": "<span class=\"selection-marker\" aria-hidden=\"true\" style=\"transform: translate3d(240.124px, 0px, 0px) scale(1.05461, 1); transform-origin: 50% 50% 0px; opacity: 1;\"></span>",
+                 "target": Array [
+                   ".selection-marker",
+                 ],
+               },
+             ],
+           },
+         ],
+         "failureSummary": "Fix any of the following:
+   Element has insufficient color contrast of 3.09 (foreground color: #7c8781, background color: #e3ecea, font size: 12.0pt (16px), font weight: normal). Expected contrast ratio of 4.5:1",
+         "html": "<a href=\"#health/journal\">Pain journal</a>",
+         "impact": "serious",
+         "none": Array [],
+         "target": Array [
+           "a[href$=\"#health/journal\"]",
+         ],
+       },
+     ],
+     "tags": Array [
+       "cat.color",
+       "wcag2aa",
+       "wcag143",
+       "TTv5",
+       "TT13.c",
+       "EN-301-549",
+       "EN-9.1.4.3",
+       "ACT",
+       "RGAAv4",
+       "RGAA-3.2.1",
+     ],
+   },
+ ]
```

# Page snapshot

```yaml
- generic [ref=e2]:
  - link "Skip to workspace" [ref=e3] [cursor=pointer]:
    - /url: "#workspace"
  - banner [ref=e4]:
    - link "Runalytics training" [ref=e5] [cursor=pointer]:
      - /url: "#training"
      - generic [ref=e6]: Runalytics
    - navigation "Workspaces" [ref=e7]:
      - link "Training" [ref=e8] [cursor=pointer]:
        - /url: "#training"
      - link "Activities" [ref=e12] [cursor=pointer]:
        - /url: "#journal"
      - link "Health" [ref=e19] [cursor=pointer]:
        - /url: "#health"
    - generic [ref=e23]:
      - button "Sync activities" [ref=e24] [cursor=pointer]
      - button "Help" [ref=e29] [cursor=pointer]
      - button "Settings" [ref=e34] [cursor=pointer]
  - main [ref=e39]:
    - generic [ref=e40]:
      - heading "Settings" [active] [level=1] [ref=e41]
      - generic [ref=e42]: COROS connected
    - generic [ref=e45]:
      - searchbox "Search settings" [ref=e50]
      - generic [ref=e51]:
        - link "Watch connection COROS, sign-in, history sync, API diagnostics" [ref=e52] [cursor=pointer]:
          - /url: "#settings/watch"
          - generic [ref=e57]:
            - strong [ref=e58]: Watch connection
            - generic [ref=e59]: COROS, sign-in, history sync, API diagnostics
        - link "Appearance System theme, light, dark, colour palette" [ref=e62] [cursor=pointer]:
          - /url: "#settings/appearance"
          - generic [ref=e68]:
            - strong [ref=e69]: Appearance
            - generic [ref=e70]: System theme, light, dark, colour palette
        - link "Training boundaries Load, recovery, build cycle, pace tolerance" [ref=e73] [cursor=pointer]:
          - /url: "#settings/thresholds"
          - generic [ref=e78]:
            - strong [ref=e79]: Training boundaries
            - generic [ref=e80]: Load, recovery, build cycle, pace tolerance
        - link "Calendar and devices Calendar subscription, publishing, plan sync" [ref=e83] [cursor=pointer]:
          - /url: "#settings/calendar"
          - generic [ref=e88]:
            - strong [ref=e89]: Calendar and devices
            - generic [ref=e90]: Calendar subscription, publishing, plan sync
        - link "Injury assistant Provider, model, secure API key, privacy" [ref=e93] [cursor=pointer]:
          - /url: "#settings/assistant"
          - generic [ref=e96]:
            - strong [ref=e97]: Injury assistant
            - generic [ref=e98]: Provider, model, secure API key, privacy
        - link "Offline maps Local PMTiles archive and map source" [ref=e101] [cursor=pointer]:
          - /url: "#settings/maps"
          - generic [ref=e104]:
            - strong [ref=e105]: Offline maps
            - generic [ref=e106]: Local PMTiles archive and map source
        - link "About and updates App version, releases, install an update" [ref=e109] [cursor=pointer]:
          - /url: "#settings/about"
          - generic [ref=e113]:
            - strong [ref=e114]: About and updates
            - generic [ref=e115]: App version, releases, install an update
```

# Test source

```ts
  91  |         await session.send("Emulation.setEmulatedMedia", { features: [] });
  92  |         await session.detach();
  93  |         const removed = await page.evaluate(() => {
  94  |           let count = 0;
  95  |           for (const sheet of document.styleSheets) {
  96  |             for (let index = sheet.cssRules.length - 1; index >= 0; index--) {
  97  |               const rule = sheet.cssRules[index];
  98  |               if (rule instanceof CSSSupportsRule && rule.conditionText.includes("backdrop-filter")) { sheet.deleteRule(index); count++; }
  99  |             }
  100 |           }
  101 |           return count;
  102 |         });
  103 |         expect(removed).toBeGreaterThan(0);
  104 |         await expect(dialog).toHaveCSS("backdrop-filter", "none");
  105 |         await expect(dialog).toHaveCSS("background-color", opaqueColor);
  106 |         await page.keyboard.press("Escape");
  107 |         await expect(dialog).toHaveCount(0);
  108 |         expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  109 |       });
  110 |     }
  111 |   }
  112 | 
  113 |   for (const width of [390, 1440]) {
  114 |     for (const reducedMotion of ["no-preference", "reduce"]) {
  115 |       test(`semantic reveals and compact presses ${reducedMotion} ${width}px`, async ({ page }) => {
  116 |         await page.setViewportSize({ width, height: 900 });
  117 |         await page.emulateMedia({ reducedMotion });
  118 |         await page.addInitScript(() => {
  119 |           window.__contentReveals = [];
  120 |           for (const type of ["animationstart", "animationend"]) {
  121 |             document.addEventListener(type, event => {
  122 |               if (event.animationName !== "content-arrive") return;
  123 |               const bounds = event.target.getBoundingClientRect();
  124 |               window.__contentReveals.push({ type, className: event.target.className, width: bounds.width, height: bounds.height, x: bounds.x, y: bounds.y, elapsed: event.elapsedTime, filter: getComputedStyle(event.target).filter });
  125 |             });
  126 |           }
  127 |         });
  128 |         await page.goto("/desk.html#health/overview");
  129 |         const verdict = page.locator(".health-verdict");
  130 |         await expect(verdict).toBeVisible();
  131 |         if (reducedMotion === "no-preference") {
  132 |           await expect.poll(() => page.evaluate(() => window.__contentReveals.filter(event => event.className === "health-verdict" && event.type === "animationend").length)).toBe(1);
  133 |           const events = await page.evaluate(() => window.__contentReveals.filter(event => event.className === "health-verdict"));
  134 |           expect(events[1].width).toBeCloseTo(events[0].width, 3);
  135 |           expect(events[1].height).toBeCloseTo(events[0].height, 3);
  136 |           expect(events[1].elapsed).toBe(.42);
  137 |           expect(events[0].filter).toContain("blur(");
  138 |           await expect(verdict).toHaveAttribute("data-reveal", "settled");
  139 |           await expect(verdict).toHaveCSS("filter", "none");
  140 |           await verdict.hover();
  141 |           await page.mouse.move(0, 0);
  142 |           await expect(verdict).toHaveCSS("animation-name", "none");
  143 |         } else {
  144 |           expect(await page.evaluate(() => window.__contentReveals)).toEqual([]);
  145 |           await expect(verdict).toHaveCSS("animation-name", "none");
  146 |         }
  147 |         await page.getByRole("link", { name: "Pace insights", exact: true }).click();
  148 |         const selector = page.getByRole("button", { name: "Long", exact: true });
  149 |         await expect(selector).toBeVisible();
  150 |         const bounds = await selector.boundingBox();
  151 |         await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  152 |         await page.mouse.down();
  153 |         await expect.poll(() => selector.evaluate(element => getComputedStyle(element).scale)).toBe(reducedMotion === "reduce" ? "1" : "0.96");
  154 |         expect(await selector.evaluate(element => element.offsetWidth)).toBe(Math.round(bounds.width));
  155 |         await page.mouse.up();
  156 |         await expect(selector).toHaveCSS("scale", "none");
  157 |         await expect(selector).toHaveAttribute("aria-pressed", "true");
  158 |         await page.goto("/desk.html#settings");
  159 |         await page.getByRole("link", { name: /Offline maps/ }).click();
  160 |         const editor = page.locator(".settings-editor");
  161 |         await expect(editor).toBeVisible();
  162 |         if (reducedMotion === "no-preference") {
  163 |           await expect.poll(() => page.evaluate(() => window.__contentReveals.filter(event => event.className === "settings-editor" && event.type === "animationend").length)).toBe(1);
  164 |         }
  165 |         const before = await page.evaluate(() => window.__contentReveals.length);
  166 |         await page.getByLabel("Raster PMTiles archive").fill("/synthetic/new-map.pmtiles");
  167 |         expect(await page.evaluate(() => window.__contentReveals.length)).toBe(before);
  168 |         expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  169 |         await page.screenshot({ path: `test-results/micro-motion-${reducedMotion}-${width}.png`, fullPage: true });
  170 |       });
  171 |     }
  172 |   }
  173 | 
  174 |   for (const width of [390, 1440]) {
  175 |     for (const theme of ["light", "dark"]) {
  176 |       test(`complete motion-enabled workspace pass ${theme} ${width}px`, async ({ page }) => {
  177 |         test.setTimeout(90000);
  178 |         await page.setViewportSize({ width, height: 1000 });
  179 |         await page.emulateMedia({ reducedMotion: "no-preference" });
  180 |         const errors = [];
  181 |         page.on("pageerror", error => errors.push(error.message));
  182 |         for (const route of ["training", "training/month", "training/plan", "journal", "health", "health/pace", "health/journal", "health/milestones", "settings", "settings/appearance", "help"]) {
  183 |           await page.goto(`/desk.html?theme=${theme}#${route}`);
  184 |           await expect(page.locator("#workspace h1")).toBeVisible();
  185 |           await expect(page.locator(".desk-loading")).toHaveCount(0);
  186 |           expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), route).toBe(true);
  187 |           // axe must sample settled colours: with animations left running it
  188 |           // catches reveal animations mid-flight and reports blended colours that
  189 |           // no user ever sees (e.g. a pace-mode button at 3.59:1 while fading in).
  190 |           await page.addStyleTag({ content: "*, *::before, *::after { animation: none !important; transition: none !important; }" });
> 191 |           expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations, route).toEqual([]);
      |                                                                                                                            ^ Error: settings
  192 |           await page.screenshot({ path: `test-results/motion-${route.replaceAll("/", "-")}-${theme}-${width}.png`, fullPage: true, animations: "disabled" });
  193 |         }
  194 |         await page.goto(`/desk.html?theme=${theme}#health/pace`);
  195 |         await page.getByRole("button", { name: "Log pain", exact: true }).click();
  196 |         await expect(page.getByRole("dialog")).toBeVisible();
  197 |         await page.screenshot({ path: `test-results/motion-sheet-${theme}-${width}.png`, animations: "disabled" });
  198 |         await page.keyboard.press("Escape");
  199 |         await expect(page.getByRole("dialog")).toHaveCount(0);
  200 |         await page.getByRole("link", { name: "Training", exact: true }).click();
  201 |         if (width < 800) await page.locator(".mobile-date-ribbon button").first().click();
  202 |         await page.locator(width < 800 ? ".day-itinerary .session-tile" : ".week-board .session-tile").first().click();
  203 |         await expect(page.locator(".desk-inspector")).toBeVisible();
  204 |         await page.screenshot({ path: `test-results/motion-inspector-${theme}-${width}.png`, animations: "disabled" });
  205 |         await page.keyboard.press("Escape");
  206 |         await expect(page.locator(".desk-inspector")).toHaveCount(0);
  207 |         expect(errors).toEqual([]);
  208 |       });
  209 |     }
  210 |   }
  211 | 
  212 |   test("selection continuity, tooltips and resize remain interruptible", async ({ page }) => {
  213 |     await page.setViewportSize({ width: 1440, height: 900 });
  214 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  215 |     await page.goto("/desk.html#training");
  216 |     const navigation = page.getByRole("navigation", { name: "Workspaces" });
  217 |     for (const workspace of ["Activities", "Health", "Training", "Health"]) {
  218 |       await navigation.getByRole("link", { name: workspace, exact: true }).click();
  219 |       await expect(page.locator("#workspace h1")).toHaveText(workspace);
  220 |     }
  221 |     const active = navigation.locator("a[aria-current]");
  222 |     await expect.poll(async () => {
  223 |       const link = await active.boundingBox();
  224 |       const marker = await active.locator(".selection-marker").boundingBox();
  225 |       return Math.abs(link.x - marker.x) + Math.abs(link.width - marker.width);
  226 |     }).toBeLessThan(1);
  227 |     const help = page.getByRole("button", { name: "Help", exact: true });
  228 |     await help.hover();
  229 |     await expect(help.locator(".control-tooltip")).toBeVisible();
  230 |     await page.keyboard.press("Escape");
  231 |     await expect(help.locator(".control-tooltip")).toBeHidden();
  232 |     await page.mouse.move(100, 200);
  233 |     await expect(help.locator(".control-tooltip")).toBeHidden();
  234 |     await help.focus();
  235 |     await page.keyboard.press("Tab");
  236 |     await page.keyboard.press("Shift+Tab");
  237 |     await expect(help).toBeFocused();
  238 |     await expect(help.locator(".control-tooltip")).toBeVisible();
  239 |     await navigation.getByRole("link", { name: "Training", exact: true }).click();
  240 |     await page.locator(".week-board .session-tile").first().click();
  241 |     await expect(page.locator(".desk-inspector")).toBeVisible();
  242 |     await page.setViewportSize({ width: 390, height: 844 });
  243 |     await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe("hidden");
  244 |     await page.emulateMedia({ reducedMotion: "reduce" });
  245 |     await expect.poll(async () => Math.round((await page.locator(".desk-inspector").boundingBox()).x)).toBe(0);
  246 |     await page.keyboard.press("Escape");
  247 |     await expect(page.locator(".desk-inspector")).toHaveCount(0);
  248 |     expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
  249 |   });
  250 | 
  251 |   test("buttons preserve geometry through pending, success, error and keyboard retry", async ({ page }) => {
  252 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  253 |     await page.addInitScript(() => {
  254 |       const invoke = window.__TAURI_INTERNALS__.invoke;
  255 |       window.__TAURI_INTERNALS__.invoke = async (command, args) => {
  256 |         if (command === "save_settings") await new Promise(resolve => { window.__releaseMotionAction = resolve; });
  257 |         return invoke(command, args);
  258 |       };
  259 |     });
  260 |     await page.goto("/desk.html#settings/maps");
  261 |     const save = page.getByRole("button", { name: "Save map source", exact: true });
  262 |     await expect(save).toBeVisible();
  263 |     await page.evaluate(() => document.fonts.ready);
  264 |     const resting = await save.boundingBox();
  265 |     await save.click();
  266 |     await expect(save).toHaveAttribute("aria-busy", "true");
  267 |     await expect(save).toBeDisabled();
  268 |     expect((await save.boundingBox()).width).toBeCloseTo(resting.width, 1);
  269 |     expect((await save.boundingBox()).height).toBeCloseTo(resting.height, 1);
  270 |     await page.evaluate(() => window.__releaseMotionAction());
  271 |     await expect(save).toHaveAttribute("data-result", "success");
  272 |     await expect(save).toBeEnabled();
  273 |     await expect(page.getByRole("status")).toContainText("Map source saved");
  274 |     await page.evaluate(() => { window.__deskFail = "save_settings"; });
  275 |     await page.getByLabel("Raster PMTiles archive").fill("/synthetic/map.pmtiles");
  276 |     await page.getByLabel("Raster PMTiles archive").press("Enter");
  277 |     await expect(save).toHaveAttribute("aria-busy", "true");
  278 |     await page.evaluate(() => window.__releaseMotionAction());
  279 |     await expect(save).toHaveAttribute("data-result", "error");
  280 |     await expect(page.getByRole("alert")).toContainText("Unable to save");
  281 |     await expect(page.getByLabel("Raster PMTiles archive")).toHaveValue("/synthetic/map.pmtiles");
  282 |     expect((await save.boundingBox()).width).toBeCloseTo(resting.width, 1);
  283 |     await page.evaluate(() => { window.__deskFail = ""; });
  284 |     await save.press("Enter");
  285 |     await page.evaluate(() => window.__releaseMotionAction());
  286 |     await expect(save).toHaveAttribute("data-result", "success");
  287 |     const notice = page.locator(".desk-notice");
  288 |     await page.getByRole("button", { name: "Dismiss notification" }).evaluate(button => button.click());
  289 |     await expect(notice).toBeAttached();
  290 |     await expect(notice).toHaveCount(0);
  291 |   });
```