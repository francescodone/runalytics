# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> motion system >> complete motion-enabled workspace pass light 1440px
- Location: tests/desk.spec.mjs:176:7

# Error details

```
Error: health/journal

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
+               "bgColor": "#eef6f2",
+               "contrastRatio": 4.38,
+               "expectedContrastRatio": "4.5:1",
+               "fgColor": "#68756d",
+               "fontSize": "12.0pt (16px)",
+               "fontWeight": "normal",
+               "messageKey": null,
+             },
+             "id": "color-contrast",
+             "impact": "serious",
+             "message": "Element has insufficient color contrast of 4.38 (foreground color: #68756d, background color: #eef6f2, font size: 12.0pt (16px), font weight: normal). Expected contrast ratio of 4.5:1",
+             "relatedNodes": Array [
+               Object {
+                 "html": "<div class=\"pace-mode\" role=\"group\" aria-label=\"Pace workout type\"><button type=\"button\" aria-pressed=\"true\"><span class=\"selection-marker\" aria-hidden=\"true\" style=\"opacity: 1;\"></span>Speed</button><button type=\"button\" aria-pressed=\"false\">Long</button></div>",
+                 "target": Array [
+                   ".pace-mode",
+                 ],
+               },
+             ],
+           },
+         ],
+         "failureSummary": "Fix any of the following:
+   Element has insufficient color contrast of 4.38 (foreground color: #68756d, background color: #eef6f2, font size: 12.0pt (16px), font weight: normal). Expected contrast ratio of 4.5:1",
+         "html": "<button type=\"button\" aria-pressed=\"false\">Long</button>",
+         "impact": "serious",
+         "none": Array [],
+         "target": Array [
+           ".pace-mode > button:nth-child(2)",
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
      - heading "Health" [active] [level=1] [ref=e41]
      - generic [ref=e42]: COROS connected
    - generic [ref=e43]:
      - generic [ref=e44]:
        - generic [ref=e45]:
          - text: Training and recovery
          - heading "Listen to your body" [level=2] [ref=e46]
        - button "Log pain" [ref=e47] [cursor=pointer]
      - navigation "Health view" [ref=e51]:
        - link "Overview" [ref=e52] [cursor=pointer]:
          - /url: "#health/overview"
        - link "Pace insights" [ref=e53] [cursor=pointer]:
          - /url: "#health/pace"
        - link "Pain journal" [ref=e54] [cursor=pointer]:
          - /url: "#health/journal"
        - link "Milestones" [ref=e55] [cursor=pointer]:
          - /url: "#health/milestones"
      - generic [ref=e57]:
        - generic [ref=e58]:
          - generic [ref=e59]:
            - heading "Episodes" [level=3] [ref=e60]
            - generic [ref=e61]:
              - checkbox "Include resolved" [ref=e62]
              - text: Include resolved
          - generic [ref=e63]:
            - heading "No active pain entries" [level=2] [ref=e64]
            - paragraph [ref=e65]: Keep a record when something does not feel right.
            - button "Log pain" [ref=e66] [cursor=pointer]
        - complementary [ref=e69]:
          - generic [ref=e70]:
            - heading "Recovery adjustments" [level=3] [ref=e71]
            - paragraph [ref=e72]: No changes awaiting your review.
          - heading "Last 30 days" [level=3] [ref=e73]
          - figure "Daily pain intensity No data" [ref=e74]:
            - generic [ref=e75]:
              - generic [ref=e76]: Daily pain intensity
              - strong [ref=e77]: No data
            - paragraph [ref=e78]: No measurements in this period.
            - group [ref=e79]:
              - generic "Data for daily pain intensity" [ref=e80] [cursor=pointer]
```

# Test source

```ts
  87  |         const session = await page.context().newCDPSession(page);
  88  |         await session.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-transparency", value: "reduce" }] });
  89  |         await expect(dialog).toHaveCSS("backdrop-filter", "none");
  90  |         await expect(dialog).toHaveCSS("background-color", opaqueColor);
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
> 187 |           expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations, route).toEqual([]);
      |                                                                                                                            ^ Error: health/journal
  188 |           await page.screenshot({ path: `test-results/motion-${route.replaceAll("/", "-")}-${theme}-${width}.png`, fullPage: true, animations: "disabled" });
  189 |         }
  190 |         await page.goto(`/desk.html?theme=${theme}#health/pace`);
  191 |         await page.getByRole("button", { name: "Log pain", exact: true }).click();
  192 |         await expect(page.getByRole("dialog")).toBeVisible();
  193 |         await page.screenshot({ path: `test-results/motion-sheet-${theme}-${width}.png`, animations: "disabled" });
  194 |         await page.keyboard.press("Escape");
  195 |         await expect(page.getByRole("dialog")).toHaveCount(0);
  196 |         await page.getByRole("link", { name: "Training", exact: true }).click();
  197 |         if (width < 800) await page.locator(".mobile-date-ribbon button").first().click();
  198 |         await page.locator(width < 800 ? ".day-itinerary .session-tile" : ".week-board .session-tile").first().click();
  199 |         await expect(page.locator(".desk-inspector")).toBeVisible();
  200 |         await page.screenshot({ path: `test-results/motion-inspector-${theme}-${width}.png`, animations: "disabled" });
  201 |         await page.keyboard.press("Escape");
  202 |         await expect(page.locator(".desk-inspector")).toHaveCount(0);
  203 |         expect(errors).toEqual([]);
  204 |       });
  205 |     }
  206 |   }
  207 | 
  208 |   test("selection continuity, tooltips and resize remain interruptible", async ({ page }) => {
  209 |     await page.setViewportSize({ width: 1440, height: 900 });
  210 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  211 |     await page.goto("/desk.html#training");
  212 |     const navigation = page.getByRole("navigation", { name: "Workspaces" });
  213 |     for (const workspace of ["Activities", "Health", "Training", "Health"]) {
  214 |       await navigation.getByRole("link", { name: workspace, exact: true }).click();
  215 |       await expect(page.locator("#workspace h1")).toHaveText(workspace);
  216 |     }
  217 |     const active = navigation.locator("a[aria-current]");
  218 |     await expect.poll(async () => {
  219 |       const link = await active.boundingBox();
  220 |       const marker = await active.locator(".selection-marker").boundingBox();
  221 |       return Math.abs(link.x - marker.x) + Math.abs(link.width - marker.width);
  222 |     }).toBeLessThan(1);
  223 |     const help = page.getByRole("button", { name: "Help", exact: true });
  224 |     await help.hover();
  225 |     await expect(help.locator(".control-tooltip")).toBeVisible();
  226 |     await page.keyboard.press("Escape");
  227 |     await expect(help.locator(".control-tooltip")).toBeHidden();
  228 |     await page.mouse.move(100, 200);
  229 |     await expect(help.locator(".control-tooltip")).toBeHidden();
  230 |     await help.focus();
  231 |     await page.keyboard.press("Tab");
  232 |     await page.keyboard.press("Shift+Tab");
  233 |     await expect(help).toBeFocused();
  234 |     await expect(help.locator(".control-tooltip")).toBeVisible();
  235 |     await navigation.getByRole("link", { name: "Training", exact: true }).click();
  236 |     await page.locator(".week-board .session-tile").first().click();
  237 |     await expect(page.locator(".desk-inspector")).toBeVisible();
  238 |     await page.setViewportSize({ width: 390, height: 844 });
  239 |     await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe("hidden");
  240 |     await page.emulateMedia({ reducedMotion: "reduce" });
  241 |     await expect.poll(async () => Math.round((await page.locator(".desk-inspector").boundingBox()).x)).toBe(0);
  242 |     await page.keyboard.press("Escape");
  243 |     await expect(page.locator(".desk-inspector")).toHaveCount(0);
  244 |     expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
  245 |   });
  246 | 
  247 |   test("buttons preserve geometry through pending, success, error and keyboard retry", async ({ page }) => {
  248 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  249 |     await page.addInitScript(() => {
  250 |       const invoke = window.__TAURI_INTERNALS__.invoke;
  251 |       window.__TAURI_INTERNALS__.invoke = async (command, args) => {
  252 |         if (command === "save_settings") await new Promise(resolve => { window.__releaseMotionAction = resolve; });
  253 |         return invoke(command, args);
  254 |       };
  255 |     });
  256 |     await page.goto("/desk.html#settings/maps");
  257 |     const save = page.getByRole("button", { name: "Save map source", exact: true });
  258 |     await expect(save).toBeVisible();
  259 |     await page.evaluate(() => document.fonts.ready);
  260 |     const resting = await save.boundingBox();
  261 |     await save.click();
  262 |     await expect(save).toHaveAttribute("aria-busy", "true");
  263 |     await expect(save).toBeDisabled();
  264 |     expect((await save.boundingBox()).width).toBeCloseTo(resting.width, 1);
  265 |     expect((await save.boundingBox()).height).toBeCloseTo(resting.height, 1);
  266 |     await page.evaluate(() => window.__releaseMotionAction());
  267 |     await expect(save).toHaveAttribute("data-result", "success");
  268 |     await expect(save).toBeEnabled();
  269 |     await expect(page.getByRole("status")).toContainText("Map source saved");
  270 |     await page.evaluate(() => { window.__deskFail = "save_settings"; });
  271 |     await page.getByLabel("Raster PMTiles archive").fill("/synthetic/map.pmtiles");
  272 |     await page.getByLabel("Raster PMTiles archive").press("Enter");
  273 |     await expect(save).toHaveAttribute("aria-busy", "true");
  274 |     await page.evaluate(() => window.__releaseMotionAction());
  275 |     await expect(save).toHaveAttribute("data-result", "error");
  276 |     await expect(page.getByRole("alert")).toContainText("Unable to save");
  277 |     await expect(page.getByLabel("Raster PMTiles archive")).toHaveValue("/synthetic/map.pmtiles");
  278 |     expect((await save.boundingBox()).width).toBeCloseTo(resting.width, 1);
  279 |     await page.evaluate(() => { window.__deskFail = ""; });
  280 |     await save.press("Enter");
  281 |     await page.evaluate(() => window.__releaseMotionAction());
  282 |     await expect(save).toHaveAttribute("data-result", "success");
  283 |     const notice = page.locator(".desk-notice");
  284 |     await page.getByRole("button", { name: "Dismiss notification" }).evaluate(button => button.click());
  285 |     await expect(notice).toBeAttached();
  286 |     await expect(notice).toHaveCount(0);
  287 |   });
```