# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: prism.spec.mjs >> Prism activity switch settles correctly no-preference
- Location: tests/prism.spec.mjs:125:3

# Error details

```
Error: expect(received).toBe(expected) // Object.is equality

Expected: true
Received: false
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
      - button "Settings" [ref=e39] [cursor=pointer]
  - main [ref=e44]:
    - generic [ref=e45]:
      - heading "Activities" [level=1] [ref=e46]
      - generic [ref=e47]: COROS connected
    - generic [ref=e49]:
      - generic [ref=e50]:
        - generic [ref=e52]:
          - button "Filters" [ref=e53] [cursor=pointer]
          - generic [ref=e57]: 9 activities
        - region "Activity history" [ref=e58]:
          - generic [ref=e59]: Most recent first
          - generic [ref=e61]:
            - button "Easy 6 km Build endurance at conversational pace so the body adapts without accumulating fatigue. 17:23 02/11/2026 8.2 km 40m" [ref=e62] [cursor=pointer]:
              - generic [ref=e68]:
                - strong [ref=e69]: Easy 6 km
                - generic [ref=e70]: Build endurance at conversational pace so the body adapts without accumulating fatigue.
                - generic [ref=e71]: 17:23 02/11/2026
              - generic [ref=e72]:
                - strong [ref=e73]: 8.2 km
                - generic [ref=e74]: 40m
            - button "Ride 17:23 02/11/2026 90.0 km 2h 30m" [active] [pressed] [ref=e75] [cursor=pointer]:
              - generic [ref=e81]:
                - strong [ref=e82]: Ride
                - generic [ref=e83]: 17:23 02/11/2026
              - generic [ref=e84]:
                - strong [ref=e85]: 90.0 km
                - generic [ref=e86]: 2h 30m
            - button "Tempo 5 km Build endurance at conversational pace so the body adapts without accumulating fatigue. 17:23 01/11/2026 9.7 km 50m" [ref=e87] [cursor=pointer]:
              - generic [ref=e93]:
                - strong [ref=e94]: Tempo 5 km
                - generic [ref=e95]: Build endurance at conversational pace so the body adapts without accumulating fatigue.
                - generic [ref=e96]: 17:23 01/11/2026
              - generic [ref=e97]:
                - strong [ref=e98]: 9.7 km
                - generic [ref=e99]: 50m
            - button "Recovery 5 km Build endurance at conversational pace so the body adapts without accumulating fatigue. 17:23 31/10/2026 11.2 km 1h 00m" [ref=e100] [cursor=pointer]:
              - generic [ref=e106]:
                - strong [ref=e107]: Recovery 5 km
                - generic [ref=e108]: Build endurance at conversational pace so the body adapts without accumulating fatigue.
                - generic [ref=e109]: 17:23 31/10/2026
              - generic [ref=e110]:
                - strong [ref=e111]: 11.2 km
                - generic [ref=e112]: 1h 00m
            - button "Run 17:23 29/10/2026 12.7 km 1h 10m" [ref=e113] [cursor=pointer]:
              - generic [ref=e119]:
                - strong [ref=e120]: Run
                - generic [ref=e121]: 17:23 29/10/2026
              - generic [ref=e122]:
                - strong [ref=e123]: 12.7 km
                - generic [ref=e124]: 1h 10m
            - button "Run 17:23 28/10/2026 14.2 km 1h 20m" [ref=e125] [cursor=pointer]:
              - generic [ref=e131]:
                - strong [ref=e132]: Run
                - generic [ref=e133]: 17:23 28/10/2026
              - generic [ref=e134]:
                - strong [ref=e135]: 14.2 km
                - generic [ref=e136]: 1h 20m
            - button "Strength 17:23 28/10/2026 0.0 km 45m" [ref=e137] [cursor=pointer]:
              - generic [ref=e143]:
                - strong [ref=e144]: Strength
                - generic [ref=e145]: 17:23 28/10/2026
              - generic [ref=e146]:
                - strong [ref=e147]: 0.0 km
                - generic [ref=e148]: 45m
            - button "Run 17:23 25/10/2026 15.7 km 1h 30m" [ref=e149] [cursor=pointer]:
              - generic [ref=e155]:
                - strong [ref=e156]: Run
                - generic [ref=e157]: 17:23 25/10/2026
              - generic [ref=e158]:
                - strong [ref=e159]: 15.7 km
                - generic [ref=e160]: 1h 30m
            - button "Ride 18:23 24/10/2026 62.0 km 2h 00m" [ref=e161] [cursor=pointer]:
              - generic [ref=e167]:
                - strong [ref=e168]: Ride
                - generic [ref=e169]: 18:23 24/10/2026
              - generic [ref=e170]:
                - strong [ref=e171]: 62.0 km
                - generic [ref=e172]: 2h 00m
      - complementary "Activity review" [ref=e173]:
        - generic [ref=e174]:
          - button "Back" [ref=e175] [cursor=pointer]
          - generic [ref=e180]: Activity review
        - article [ref=e182]:
          - generic [ref=e183]:
            - generic [ref=e184]: COROS / 17:23 02/11/2026
            - heading "Ride" [level=2] [ref=e185]
          - tablist "Activity detail" [ref=e186]:
            - tab "Overview" [selected] [ref=e187] [cursor=pointer]
            - tab "Session execution" [ref=e188] [cursor=pointer]
            - tab "Splits" [ref=e189] [cursor=pointer]
            - tab "Training context" [ref=e190] [cursor=pointer]
          - tabpanel "Overview" [ref=e192]:
            - generic [ref=e193]:
              - generic [ref=e194]:
                - term [ref=e195]: Distance
                - definition [ref=e196]: 90.00km
              - generic [ref=e197]:
                - term [ref=e198]: Moving time
                - definition [ref=e199]: 2h 30m
              - generic [ref=e200]:
                - term [ref=e201]: Average pace
                - definition [ref=e202]: 1:40/km
              - generic [ref=e203]:
                - term [ref=e204]: Heart rate
                - definition [ref=e205]: 132bpm
              - generic [ref=e206]:
                - term [ref=e207]: Elevation gain
                - definition [ref=e208]: 340m
            - figure [ref=e209]:
              - region "Recorded GPS route" [ref=e210]:
                - generic:
                  - region "Map" [ref=e211]
                  - img "Route start"
                  - img "Route finish"
                - group [ref=e212]:
                  - generic "Toggle attribution" [ref=e213] [cursor=pointer]
                  - link "OpenStreetMap contributors" [ref=e215] [cursor=pointer]:
                    - /url: https://www.openstreetmap.org/copyright
              - generic [ref=e216]:
                - button "Zoom in" [ref=e217] [cursor=pointer]
                - button "Zoom out" [ref=e220] [cursor=pointer]
                - button "Fit route" [ref=e223] [cursor=pointer]
```

# Test source

```ts
  44  |   });
  45  | }
  46  | 
  47  | for (const reducedMotion of ["no-preference", "reduce"]) {
  48  |   test(`Prism brand entrance runs once with ${reducedMotion} motion`, async ({ page }) => {
  49  |     await page.setViewportSize({ width: 1440, height: 900 });
  50  |     await page.emulateMedia({ reducedMotion });
  51  |     await page.addInitScript(() => {
  52  |       window.__brandEntrances = [];
  53  |       document.addEventListener("animationstart", event => {
  54  |         if (event.target.matches(".desk-brand-icon, .desk-brand-name")) window.__brandEntrances.push({ target: event.target.className, animation: event.animationName, delay: getComputedStyle(event.target).animationDelay });
  55  |       });
  56  |     });
  57  |     await page.goto("/desk.html#journal");
  58  |     const brand = page.getByRole("link", { name: "Runalytics training" });
  59  |     await expect(brand.locator(".desk-brand-icon")).toHaveAttribute("data-reveal", "settled");
  60  |     await expect(brand.locator(".desk-brand-name")).toHaveAttribute("data-reveal", "settled");
  61  |     const entrances = await page.evaluate(() => window.__brandEntrances);
  62  |     if (reducedMotion === "reduce") expect(entrances).toEqual([]);
  63  |     else {
  64  |       expect(entrances.map(entry => entry.target)).toEqual(["desk-brand-icon", "desk-brand-name"]);
  65  |       expect(entrances.every(entry => entry.animation === "content-arrive")).toBe(true);
  66  |       expect(parseFloat(entrances[1].delay)).toBeGreaterThan(parseFloat(entrances[0].delay));
  67  |     }
  68  |     await page.keyboard.press("Tab");
  69  |     await page.keyboard.press("Tab");
  70  |     await expect(brand).toBeFocused();
  71  |     await page.keyboard.press("Enter");
  72  |     await expect(page.locator(".workspace-heading h1")).toHaveText("Training");
  73  |     await page.getByRole("link", { name: "Activities", exact: true }).click();
  74  |     await expect(page.locator(".workspace-heading h1")).toHaveText("Activities");
  75  |     expect(await page.evaluate(() => window.__brandEntrances)).toEqual(entrances);
  76  |     expect(await brand.evaluate(element => element.getAnimations({ subtree: true }).filter(animation => animation.playState === "running").length)).toBe(0);
  77  |   });
  78  | }
  79  | 
  80  | for (const [first, current, label] of [[60, 70, "improving"], [80, 70, "declining"], [70, 70, "steady"], [null, 70, "history"]]) {
  81  |   test(`Prism readiness summary ${label}`, async ({ page }) => {
  82  |     await page.goto("/desk.html#training");
  83  |     await page.evaluate(({ first, current }) => {
  84  |       const invoke = window.__TAURI_INTERNALS__.invoke;
  85  |       window.__TAURI_INTERNALS__.invoke = async (command, args) => {
  86  |         const result = await invoke(command, args);
  87  |         if (command === "dashboard") {
  88  |           result.scores.globalScore = current;
  89  |           result.scoreHistory = first == null ? [] : [{ date: "2026-10-01", readiness: first }, { date: "2026-10-02", readiness: current }];
  90  |         }
  91  |         return result;
  92  |       };
  93  |     }, { first, current });
  94  |     await page.getByRole("link", { name: "Health", exact: true }).click();
  95  |     await expect(page.getByRole("region", { name: "Readiness trend" }).getByRole("heading")).toContainText(label);
  96  |     await expect(page.locator(".trend-chart:visible")).toHaveCount(1);
  97  |     await expect(page.locator("#health-details")).toBeHidden();
  98  |     await page.getByRole("button", { name: "Show health details" }).click();
  99  |     await expect(page.getByRole("heading", { name: "Risk contributors" })).toBeVisible();
  100 |     await page.getByRole("button", { name: "Hide health details" }).click();
  101 |     await expect(page.locator("#health-details")).toBeHidden();
  102 |   });
  103 | }
  104 | 
  105 | for (const width of [320, 820, 1440]) {
  106 |   test(`Prism centered milestones ${width}px`, async ({ page }) => {
  107 |     await page.setViewportSize({ width, height: 900 });
  108 |     await page.goto("/desk.html#health/milestones");
  109 |     await expect(page.locator(".milestone-list article").first()).toBeVisible();
  110 |     const layout = await page.locator(".milestones").evaluate(element => {
  111 |       const bounds = element.getBoundingClientRect();
  112 |       const grid = element.querySelector(".consistency-grid").getBoundingClientRect();
  113 |       const cell = element.querySelector(".consistency-grid > div").getBoundingClientRect();
  114 |       return { center: bounds.x + bounds.width / 2, gridCenter: grid.x + grid.width / 2, cellWidth: cell.width, cellHeight: cell.height, overflow: element.scrollWidth - element.clientWidth };
  115 |     });
  116 |     expect(Math.abs(layout.center - layout.gridCenter)).toBeLessThan(1);
  117 |     expect(Math.abs(layout.cellWidth - layout.cellHeight)).toBeLessThan(1);
  118 |     expect(layout.overflow).toBeLessThanOrEqual(1);
  119 |     await page.getByRole("checkbox", { name: "Earned only" }).check();
  120 |     await expect(page.locator(".milestone-list article:not(.earned)")).toHaveCount(0);
  121 |   });
  122 | }
  123 | 
  124 | for (const reducedMotion of ["reduce", "no-preference"]) {
  125 |   test(`Prism activity switch settles correctly ${reducedMotion}`, async ({ page }) => {
  126 |     await page.setViewportSize({ width: 1440, height: 900 });
  127 |     await page.emulateMedia({ reducedMotion });
  128 |     await page.goto("/desk.html#journal/1");
  129 |     await expect(page.locator(".activity-review h2")).toBeVisible();
  130 |     await expect(page.locator(".activity-transition .route-figure")).toHaveAttribute("data-reveal", "settled");
  131 |     await page.evaluate(() => {
  132 |       window.__activityDepartures = [];
  133 |       document.addEventListener("animationstart", event => {
  134 |         if (event.animationName === "content-depart" && event.target.closest(".activity-transition")) window.__activityDepartures.push(getComputedStyle(event.target).animationDelay);
  135 |       });
  136 |     });
  137 |     const next = page.locator(".activity-row:not([aria-pressed=true])").first();
  138 |     const name = await next.locator("strong").first().textContent();
  139 |     await next.click();
  140 |     await expect(page.locator(".activity-review h2")).toHaveText(name);
  141 |     await expect(page.locator(".activity-transition")).toHaveCount(1);
  142 |     await expect(page.locator(".activity-transition[inert]")).toHaveCount(0);
  143 |     expect(await page.locator(".desk-inspector").evaluate(element => element.scrollTop)).toBe(0);
> 144 |     if (reducedMotion === "no-preference") expect(await page.evaluate(() => window.__activityDepartures.some(delay => parseFloat(delay) > 0))).toBe(true);
      |                                                                                                                                                ^ Error: expect(received).toBe(expected) // Object.is equality
  145 |     else expect(await page.evaluate(() => window.__activityDepartures)).toEqual([]);
  146 |   });
  147 | }
  148 | 
  149 | for (const width of [820, 1440]) {
  150 |   test(`Prism independent activity panes ${width}px`, async ({ page }, testInfo) => {
  151 |     await page.setViewportSize({ width, height: 900 });
  152 |     await page.goto("/desk.html#journal/1/execution");
  153 |     const list = page.getByRole("region", { name: "Activity history" });
  154 |     const detail = page.getByRole("complementary", { name: "Activity review" });
  155 |     await expect(page.locator(".lap-execution-table")).toBeVisible();
  156 |     const geometry = await page.evaluate(() => {
  157 |       const list = document.querySelector(".activity-library").getBoundingClientRect();
  158 |       const detail = document.querySelector(".desk-inspector").getBoundingClientRect();
  159 |       const column = document.querySelector(".lap-execution-table thead th").getBoundingClientRect();
  160 |       return { listBottom: list.bottom, detailBottom: detail.bottom, listRight: list.right, detailLeft: detail.left, columnWidth: column.width };
  161 |     });
  162 |     expect(geometry.listRight).toBeLessThan(geometry.detailLeft);
  163 |     expect(geometry.listBottom).toBeGreaterThan(800);
  164 |     expect(geometry.listBottom).toBeLessThan(900);
  165 |     expect(Math.abs(geometry.detailBottom - geometry.listBottom)).toBeLessThan(3);
  166 |     expect(geometry.columnWidth).toBeLessThanOrEqual(50);
  167 |     await expect(list).toHaveCSS("padding", "0px");
  168 |     const shadow = await list.evaluate(element => {
  169 |       const row = element.querySelector(".activity-row");
  170 |       const edge = getComputedStyle(element, "::after");
  171 |       return { width: parseFloat(edge.width), rowWidth: row.getBoundingClientRect().width, bottom: edge.bottom, radius: edge.borderBottomLeftRadius, clipRadius: getComputedStyle(element).borderBottomLeftRadius };
  172 |     });
  173 |     expect(shadow.width).toBeCloseTo(shadow.rowWidth, 1);
  174 |     expect(shadow.bottom).toBe("0px");
  175 |     expect(shadow.radius).toBe(shadow.clipRadius);
  176 |     await list.evaluate(element => { element.scrollTop = 100; });
  177 |     await expect(list).toHaveAttribute("data-scroll-top", "1");
  178 |     await expect(list).toHaveAttribute("data-scroll-bottom", "1");
  179 |     await list.screenshot({ path: testInfo.outputPath("activity-scroll-edges.png") });
  180 |     await list.evaluate(element => { element.scrollTop = element.scrollHeight; });
  181 |     await expect(list).toHaveAttribute("data-scroll-bottom", "");
  182 |     expect(await detail.evaluate(element => element.scrollTop)).toBe(0);
  183 |     const listPosition = await list.evaluate(element => element.scrollTop);
  184 |     await detail.evaluate(element => { element.scrollTop = element.scrollHeight; });
  185 |     expect(await detail.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
  186 |     expect(await list.evaluate(element => element.scrollTop)).toBe(listPosition);
  187 |     expect(await page.locator("#workspace").evaluate(element => element.scrollTop)).toBe(0);
  188 |     await page.getByRole("button", { name: "Filters", exact: true }).click();
  189 |     await expect(page.locator(".journal-sidebar").getByRole("searchbox")).toBeVisible();
  190 |     await expect(detail.getByRole("searchbox")).toHaveCount(0);
  191 |     await page.getByRole("button", { name: "Hide filters", exact: true }).click();
  192 |     await expect(page.getByRole("searchbox")).toBeHidden();
  193 |     await expect(list).toBeVisible();
  194 |   });
  195 | }
  196 | 
  197 | for (const theme of ["light", "dark"]) {
  198 |   for (const width of [820, 1180, 1440]) {
  199 |     test(`Prism today marker and compact navigation ${theme} ${width}px`, async ({ page }, testInfo) => {
  200 |       await page.setViewportSize({ width, height: 900 });
  201 |       await page.goto(`/desk.html?theme=${theme}#training/week`);
  202 |       const today = page.locator(".day-lane.is-today");
  203 |       await expect(today).toBeVisible();
  204 |       await expect(today).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  205 |       const circle = today.locator("header strong");
  206 |       await expect(circle).toHaveCSS("border-radius", "50%");
  207 |       await expect(circle).not.toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  208 |       const navigation = page.getByRole("navigation", { name: "Workspaces" });
  209 |       for (const label of ["Training", "Activities", "Health"]) {
  210 |         const link = navigation.getByRole("link", { name: label, exact: true });
  211 |         await expect(link).toBeVisible();
  212 |         const dimensions = await link.evaluate(element => ({ width: element.offsetWidth, height: element.offsetHeight }));
  213 |         if (width <= 1180) {
  214 |           expect(dimensions.width).toBe(dimensions.height);
  215 |           expect(dimensions.width).toBeGreaterThanOrEqual(44);
  216 |           await expect(link.locator("span:not(.selection-marker)")).toBeHidden();
  217 |         } else {
  218 |           await expect(link.locator("span:not(.selection-marker)")).toBeVisible();
  219 |         }
  220 |       }
  221 |       await page.screenshot({ path: testInfo.outputPath("week-navigation.png") });
  222 |       await navigation.getByRole("link", { name: "Activities", exact: true }).focus();
  223 |       await page.keyboard.press("Enter");
  224 |       await expect(page.getByRole("heading", { name: "Activities", exact: true })).toBeVisible();
  225 |     });
  226 |   }
  227 | 
  228 |   for (const viewport of [{ width: 320, height: 740 }, { width: 390, height: 844 }, { width: 820, height: 900 }, { width: 913, height: 480 }, { width: 1440, height: 900 }, { width: 820, height: 480, enlarged: true }]) {
  229 |     test(`Prism full-height activity filters ${theme} ${viewport.width}x${viewport.height}${viewport.enlarged ? " enlarged" : ""}`, async ({ page }, testInfo) => {
  230 |       await page.setViewportSize({ width: viewport.width, height: viewport.height });
  231 |       await page.goto(`/desk.html?theme=${theme}#journal`);
  232 |       if (viewport.enlarged) await page.addStyleTag({ content: ":root { font-size: 200%; }" });
  233 |       const filters = page.locator("#activity-filters");
  234 |       await expect(filters).toBeHidden();
  235 |       await page.getByRole("button", { name: "Filters", exact: true }).click();
  236 |       await expect(filters).toBeVisible();
  237 |       await expect(filters).toHaveCSS("overflow-y", "visible");
  238 |       const geometry = await filters.evaluate(element => ({ height: element.clientHeight, scrollHeight: element.scrollHeight, pageOverflow: document.documentElement.scrollWidth - innerWidth }));
  239 |       expect(geometry.scrollHeight).toBeLessThanOrEqual(geometry.height + 1);
  240 |       expect(geometry.pageOverflow).toBeLessThanOrEqual(1);
  241 |       await page.screenshot({ path: testInfo.outputPath("expanded-filters.png") });
  242 |       await filters.getByRole("combobox", { name: "Sport", exact: true }).selectOption("Run");
  243 |       await expect(page.locator(".activity-row")).toHaveCount(6);
  244 |       const reset = filters.getByRole("button", { name: "Reset filters", exact: true });
```