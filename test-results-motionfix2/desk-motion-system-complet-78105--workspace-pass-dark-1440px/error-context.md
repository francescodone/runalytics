# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> motion system >> complete motion-enabled workspace pass dark 1440px
- Location: tests/desk.spec.mjs:176:7

# Error details

```
Error: health/pace

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
+               "bgColor": "#1d2420",
+               "contrastRatio": 1.87,
+               "expectedContrastRatio": "4.5:1",
+               "fgColor": "#474f4a",
+               "fontSize": "12.0pt (16px)",
+               "fontWeight": "normal",
+               "messageKey": null,
+             },
+             "id": "color-contrast",
+             "impact": "serious",
+             "message": "Element has insufficient color contrast of 1.87 (foreground color: #474f4a, background color: #1d2420, font size: 12.0pt (16px), font weight: normal). Expected contrast ratio of 4.5:1",
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
+   Element has insufficient color contrast of 1.87 (foreground color: #474f4a, background color: #1d2420, font size: 12.0pt (16px), font weight: normal). Expected contrast ratio of 4.5:1",
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
          - heading "Pace insights" [level=2] [ref=e46]
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
      - region "Pace insights" [ref=e57]:
        - group "Pace workout type" [ref=e58]:
          - button "Speed" [pressed] [ref=e59] [cursor=pointer]
          - button "Long" [ref=e60] [cursor=pointer]
        - generic [ref=e61]:
          - generic [ref=e62]:
            - strong [ref=e63]: Next speed workout in 3 days
            - paragraph [ref=e64]: 04/11/2026 / Tempo 5 km / 8.0 km
          - button "View plan" [ref=e65] [cursor=pointer]
        - generic [ref=e70]:
          - generic [ref=e71]:
            - heading "Pace on point" [level=3] [ref=e72]
            - paragraph [ref=e73]: 02/10/2026 - 22/10/2026
          - generic [ref=e74]: 2/4 sessions within target
        - figure [ref=e75]:
          - generic [ref=e76]:
            - img "Speed pace insights for 5 sessions. Higher means faster relative to each rep's target. Select a dated session below for its values." [ref=e77]:
              - 'generic "02/10/2026: Interval session 0. Fastest relative to target 2.40; slowest -2.20. Outdated." [ref=e79]'
              - 'generic "07/10/2026: Interval session 1. Fastest relative to target 0.20; slowest -0.20." [ref=e82]'
              - 'generic "12/10/2026: Continuous tempo. Fastest relative to target 0.00; slowest 0.00." [ref=e85]'
              - 'generic "17/10/2026: Interval session 3. Fastest relative to target 0.80; slowest -4.00." [ref=e87]'
              - 'generic "22/10/2026: Interval session 4. Fastest relative to target 3.00; slowest 0.20." [ref=e90]'
            - generic [aria-hidden] [ref=e93]:
              - generic [ref=e94]: Faster
              - generic [ref=e95]: Slower
          - generic [ref=e96]:
            - generic [ref=e97]: Pace on point
            - generic [ref=e99]: Fastest / slowest relative rep
        - group "Inspect pace session" [ref=e101]:
          - 'button "02/10/2026: Interval session 0, Outdated" [ref=e102] [cursor=pointer]':
            - text: 02/10/2026
            - generic [ref=e103]: Outdated
          - 'button "07/10/2026: Interval session 1" [ref=e104] [cursor=pointer]': 07/10/2026
          - 'button "12/10/2026: Continuous tempo" [ref=e105] [cursor=pointer]': 12/10/2026
          - 'button "17/10/2026: Interval session 3" [ref=e106] [cursor=pointer]': 17/10/2026
          - 'button "22/10/2026: Interval session 4" [pressed] [ref=e107] [cursor=pointer]': 22/10/2026
        - region "Selected pace session" [ref=e108]:
          - generic [ref=e110]:
            - strong [ref=e111]: Interval session 4
            - paragraph [ref=e112]: 22/10/2026 / 2 measured reps
          - button "Open activity review" [ref=e113] [cursor=pointer]
          - region "Pace insight repetitions" [ref=e118]:
            - table [ref=e119]:
              - caption [ref=e120]: Measured work reps relative to their own prescribed target
              - rowgroup [ref=e121]:
                - row [ref=e122]:
                  - columnheader "Rep" [ref=e123]
                  - columnheader "Pace /km" [ref=e124]
                  - columnheader "Target /km" [ref=e125]
                  - columnheader "Relative pace" [ref=e126]
              - rowgroup [ref=e127]:
                - row [ref=e128]:
                  - rowheader "2" [ref=e129]
                  - cell "4:25" [ref=e130]
                  - cell "4:35 - 4:45" [ref=e131]
                  - cell "Faster than target" [ref=e132]
                - row [ref=e133]:
                  - rowheader "4" [ref=e134]
                  - cell "4:39" [ref=e135]
                  - cell "4:35 - 4:45" [ref=e136]
                  - cell "Within target" [ref=e137]
        - group [ref=e138]:
          - generic "About pace insights" [ref=e139] [cursor=pointer]
        - generic [ref=e141]:
          - heading "Pacing evidence" [level=3] [ref=e142]
          - paragraph [ref=e143]: No changes awaiting your review.
```

# Test source

```ts
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
  188 |           // catches reveals and the sliding selection marker mid-flight and
  189 |           // reports blended colours that no user ever sees. finish() settles
  190 |           // CSS animations, transitions, and WAAPI springs alike.
  191 |           await page.evaluate(() => document.getAnimations().forEach(animation => { try { animation.finish(); } catch { animation.pause(); } }));
> 192 |           expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations, route).toEqual([]);
      |                                                                                                                            ^ Error: health/pace
  193 |           await page.screenshot({ path: `test-results/motion-${route.replaceAll("/", "-")}-${theme}-${width}.png`, fullPage: true, animations: "disabled" });
  194 |         }
  195 |         await page.goto(`/desk.html?theme=${theme}#health/pace`);
  196 |         await page.getByRole("button", { name: "Log pain", exact: true }).click();
  197 |         await expect(page.getByRole("dialog")).toBeVisible();
  198 |         await page.screenshot({ path: `test-results/motion-sheet-${theme}-${width}.png`, animations: "disabled" });
  199 |         await page.keyboard.press("Escape");
  200 |         await expect(page.getByRole("dialog")).toHaveCount(0);
  201 |         await page.getByRole("link", { name: "Training", exact: true }).click();
  202 |         if (width < 800) await page.locator(".mobile-date-ribbon button").first().click();
  203 |         await page.locator(width < 800 ? ".day-itinerary .session-tile" : ".week-board .session-tile").first().click();
  204 |         await expect(page.locator(".desk-inspector")).toBeVisible();
  205 |         await page.screenshot({ path: `test-results/motion-inspector-${theme}-${width}.png`, animations: "disabled" });
  206 |         await page.keyboard.press("Escape");
  207 |         await expect(page.locator(".desk-inspector")).toHaveCount(0);
  208 |         expect(errors).toEqual([]);
  209 |       });
  210 |     }
  211 |   }
  212 | 
  213 |   test("selection continuity, tooltips and resize remain interruptible", async ({ page }) => {
  214 |     await page.setViewportSize({ width: 1440, height: 900 });
  215 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  216 |     await page.goto("/desk.html#training");
  217 |     const navigation = page.getByRole("navigation", { name: "Workspaces" });
  218 |     for (const workspace of ["Activities", "Health", "Training", "Health"]) {
  219 |       await navigation.getByRole("link", { name: workspace, exact: true }).click();
  220 |       await expect(page.locator("#workspace h1")).toHaveText(workspace);
  221 |     }
  222 |     const active = navigation.locator("a[aria-current]");
  223 |     await expect.poll(async () => {
  224 |       const link = await active.boundingBox();
  225 |       const marker = await active.locator(".selection-marker").boundingBox();
  226 |       return Math.abs(link.x - marker.x) + Math.abs(link.width - marker.width);
  227 |     }).toBeLessThan(1);
  228 |     const help = page.getByRole("button", { name: "Help", exact: true });
  229 |     await help.hover();
  230 |     await expect(help.locator(".control-tooltip")).toBeVisible();
  231 |     await page.keyboard.press("Escape");
  232 |     await expect(help.locator(".control-tooltip")).toBeHidden();
  233 |     await page.mouse.move(100, 200);
  234 |     await expect(help.locator(".control-tooltip")).toBeHidden();
  235 |     await help.focus();
  236 |     await page.keyboard.press("Tab");
  237 |     await page.keyboard.press("Shift+Tab");
  238 |     await expect(help).toBeFocused();
  239 |     await expect(help.locator(".control-tooltip")).toBeVisible();
  240 |     await navigation.getByRole("link", { name: "Training", exact: true }).click();
  241 |     await page.locator(".week-board .session-tile").first().click();
  242 |     await expect(page.locator(".desk-inspector")).toBeVisible();
  243 |     await page.setViewportSize({ width: 390, height: 844 });
  244 |     await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe("hidden");
  245 |     await page.emulateMedia({ reducedMotion: "reduce" });
  246 |     await expect.poll(async () => Math.round((await page.locator(".desk-inspector").boundingBox()).x)).toBe(0);
  247 |     await page.keyboard.press("Escape");
  248 |     await expect(page.locator(".desk-inspector")).toHaveCount(0);
  249 |     expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
  250 |   });
  251 | 
  252 |   test("buttons preserve geometry through pending, success, error and keyboard retry", async ({ page }) => {
  253 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  254 |     await page.addInitScript(() => {
  255 |       const invoke = window.__TAURI_INTERNALS__.invoke;
  256 |       window.__TAURI_INTERNALS__.invoke = async (command, args) => {
  257 |         if (command === "save_settings") await new Promise(resolve => { window.__releaseMotionAction = resolve; });
  258 |         return invoke(command, args);
  259 |       };
  260 |     });
  261 |     await page.goto("/desk.html#settings/maps");
  262 |     const save = page.getByRole("button", { name: "Save map source", exact: true });
  263 |     await expect(save).toBeVisible();
  264 |     await page.evaluate(() => document.fonts.ready);
  265 |     const resting = await save.boundingBox();
  266 |     await save.click();
  267 |     await expect(save).toHaveAttribute("aria-busy", "true");
  268 |     await expect(save).toBeDisabled();
  269 |     expect((await save.boundingBox()).width).toBeCloseTo(resting.width, 1);
  270 |     expect((await save.boundingBox()).height).toBeCloseTo(resting.height, 1);
  271 |     await page.evaluate(() => window.__releaseMotionAction());
  272 |     await expect(save).toHaveAttribute("data-result", "success");
  273 |     await expect(save).toBeEnabled();
  274 |     await expect(page.getByRole("status")).toContainText("Map source saved");
  275 |     await page.evaluate(() => { window.__deskFail = "save_settings"; });
  276 |     await page.getByLabel("Raster PMTiles archive").fill("/synthetic/map.pmtiles");
  277 |     await page.getByLabel("Raster PMTiles archive").press("Enter");
  278 |     await expect(save).toHaveAttribute("aria-busy", "true");
  279 |     await page.evaluate(() => window.__releaseMotionAction());
  280 |     await expect(save).toHaveAttribute("data-result", "error");
  281 |     await expect(page.getByRole("alert")).toContainText("Unable to save");
  282 |     await expect(page.getByLabel("Raster PMTiles archive")).toHaveValue("/synthetic/map.pmtiles");
  283 |     expect((await save.boundingBox()).width).toBeCloseTo(resting.width, 1);
  284 |     await page.evaluate(() => { window.__deskFail = ""; });
  285 |     await save.press("Enter");
  286 |     await page.evaluate(() => window.__releaseMotionAction());
  287 |     await expect(save).toHaveAttribute("data-result", "success");
  288 |     const notice = page.locator(".desk-notice");
  289 |     await page.getByRole("button", { name: "Dismiss notification" }).evaluate(button => button.click());
  290 |     await expect(notice).toBeAttached();
  291 |     await expect(notice).toHaveCount(0);
  292 |   });
```