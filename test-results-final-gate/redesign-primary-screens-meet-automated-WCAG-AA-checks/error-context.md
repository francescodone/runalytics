# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: redesign.spec.mjs >> primary screens meet automated WCAG AA checks
- Location: tests/redesign.spec.mjs:511:1

# Error details

```
Error: page.evaluate: Execution context was destroyed, most likely because of a navigation
```

# Page snapshot

```yaml
- generic [active] [ref=f3e1]:
  - link "Skip to content" [ref=f3e2] [cursor=pointer]:
    - /url: "#app"
  - navigation "Main" [ref=f3e3]:
    - generic [ref=f3e4]:
      - link [ref=f3e5] [cursor=pointer]:
        - /url: index.html
      - link [ref=f3e10] [cursor=pointer]:
        - /url: plan.html
      - link [ref=f3e14] [cursor=pointer]:
        - /url: calendar.html
      - link [ref=f3e20] [cursor=pointer]:
        - /url: achievements.html
    - group [ref=f3e26]:
      - generic "More" [ref=f3e27] [cursor=pointer]
  - main [ref=f3e35]:
    - heading "Plan" [level=1] [ref=f3e38]
    - generic [ref=f3e39]:
      - heading "Your plan" [level=2] [ref=f3e41]
      - generic [ref=f3e42]:
        - generic [ref=f3e43]:
          - generic [ref=f3e44]:
            - heading "21K race plan" [level=3] [ref=f3e45]
            - paragraph [ref=f3e46]: 20/12/2026 · 4 runs per week
          - generic [ref=f3e47]: On your watch
        - group [ref=f3e48]:
          - generic "Plan details and predictions" [ref=f3e49] [cursor=pointer]
      - generic [ref=f3e50]:
        - generic [ref=f3e51]:
          - heading "Training schedule" [level=3] [ref=f3e52]
          - link "Open calendar" [ref=f3e53] [cursor=pointer]:
            - /url: calendar.html
        - generic [ref=f3e54]:
          - button "Previous week" [ref=f3e55] [cursor=pointer]
          - combobox "Training week" [ref=f3e56] [cursor=pointer]:
            - option "Week 1 - 12/10/2026"
            - option "Week 2 - 19/10/2026"
            - option "Week 3 - 26/10/2026"
            - option "Week 4 - 02/11/2026" [selected]
            - option "Week 5 - 09/11/2026"
            - option "Week 6 - 16/11/2026"
            - option "Week 7 - 23/11/2026"
            - option "Week 8 - 30/11/2026"
            - option "Week 9 - 14/12/2026"
          - button "Next week" [ref=f3e57] [cursor=pointer]
        - generic [ref=f3e59]:
          - status [ref=f3e60]: Week 4 · 02/11/2026 · 35.0 km · 0 of 4 sessions completed
          - tabpanel "Week 4 · 02/11/2026 · 35.0 km · 0 of 4 sessions completed" [ref=f3e61]:
            - button "Mon 2 Easy 6 km 6.0 km · at 6:00–6:40/km" [ref=f3e62] [cursor=pointer]:
              - generic [ref=f3e63]:
                - generic [ref=f3e64]: Mon
                - strong [ref=f3e65]: "2"
              - generic [ref=f3e66]:
                - strong [ref=f3e67]: Easy 6 km
                - generic [ref=f3e68]: 6.0 km · at 6:00–6:40/km
            - button "Wed 4 Tempo 5 km 5.0 km · at 5:00–5:30/km" [ref=f3e72] [cursor=pointer]:
              - generic [ref=f3e73]:
                - generic [ref=f3e74]: Wed
                - strong [ref=f3e75]: "4"
              - generic [ref=f3e76]:
                - strong [ref=f3e77]: Tempo 5 km
                - generic [ref=f3e78]: 5.0 km · at 5:00–5:30/km
            - button "Fri 6 Recovery 5 km 5.0 km · at 6:00–6:40/km" [ref=f3e82] [cursor=pointer]:
              - generic [ref=f3e83]:
                - generic [ref=f3e84]: Fri
                - strong [ref=f3e85]: "6"
              - generic [ref=f3e86]:
                - strong [ref=f3e87]: Recovery 5 km
                - generic [ref=f3e88]: 5.0 km · at 6:00–6:40/km
            - button "Sun 8 Long 19 km 19.0 km · at 5:00–5:30/km" [ref=f3e92] [cursor=pointer]:
              - generic [ref=f3e93]:
                - generic [ref=f3e94]: Sun
                - strong [ref=f3e95]: "8"
              - generic [ref=f3e96]:
                - strong [ref=f3e97]: Long 19 km
                - generic [ref=f3e98]: 19.0 km · at 5:00–5:30/km
      - group [ref=f3e102]:
        - generic "Manage plan" [ref=f3e103] [cursor=pointer]
```

# Test source

```ts
  417 |   const rows = page.locator(".activity-row");
  418 |   await expect(rows.first()).toBeVisible();
  419 |   const total = await rows.count();
  420 |   const name = await rows.first().locator("h3").innerText();
  421 |   const search = page.getByRole("searchbox", { name: "Search workouts" });
  422 |   await search.fill(name.toUpperCase());
  423 |   await expect(rows.first()).toContainText(name);
  424 |   await page.locator("#history-type").selectOption("run");
  425 |   await search.fill("no-such-workout");
  426 |   await expect(rows).toHaveCount(0);
  427 |   await expect(page.getByRole("heading", { name: "No matching workouts" })).toBeVisible();
  428 |   await expect(page.locator("#history-summary")).toContainText("0 activities");
  429 |   await page.locator("#history-empty-reset").click();
  430 |   await expect(search).toBeFocused();
  431 |   await expect(search).toHaveValue("");
  432 |   await expect(page.locator("#history-type")).toHaveValue("all");
  433 |   await expect(rows).toHaveCount(total);
  434 |   await expect(page.locator("#history-reset")).toBeHidden();
  435 | });
  436 | 
  437 | test("Progress opens the shared completed-run review and restores focus", async ({ page }) => {
  438 |   await page.goto("/achievements.html");
  439 |   const activity = page.locator(".activity-row").first();
  440 |   await activity.click();
  441 |   await expect(page.locator(".shared-run-review .run-laps")).toContainText("Lap 2 too fast");
  442 |   await expect(page.locator(".shared-run-review .run-splits")).toBeVisible();
  443 |   await page.getByRole("button", { name: "Close", exact: true }).click();
  444 |   await expect(page.locator("dialog[open]")).toHaveCount(0);
  445 |   await expect(activity).toBeFocused();
  446 | });
  447 | 
  448 | test("touch rescheduling retains its receipt and Undo", async ({ page }) => {
  449 |   await page.goto("/__calendar_harness.html");
  450 |   await page.locator('.agenda-row[data-event-index="4"]').click();
  451 |   await page.getByRole("button", { name: "Move to", exact: false }).click();
  452 |   await page.locator("#move-session-date").fill("2026-10-15");
  453 |   await page.getByRole("button", { name: "Move session", exact: true }).click();
  454 |   await expect(page.locator("#toast")).toContainText("Moved to");
  455 |   await page.getByRole("button", { name: "Undo", exact: true }).click();
  456 |   await expect(page.locator("dialog[open]")).toHaveCount(0);
  457 | });
  458 | 
  459 | test("No-plan accounts retain Calendar and Progress routes", async ({ page }) => {
  460 |   for (const route of ["calendar.html", "achievements.html"]) {
  461 |     await page.goto(`/${route}?noPlan=1`);
  462 |     await expect(page.locator("#nav-destinations .nav-link")).toHaveCount(4);
  463 |     await expect(page).toHaveURL(new RegExp(route));
  464 |   }
  465 | });
  466 | 
  467 | test("Reference and legacy palettes remain selectable", async ({ page }) => {
  468 |   await page.goto("/support.html");
  469 |   const colours = await page.evaluate(async () => {
  470 |     const { setPalette } = await import("/src/lib/theme.ts");
  471 |     setPalette("amethyst");
  472 |     const legacy = getComputedStyle(document.documentElement).getPropertyValue("--bg").trim();
  473 |     setPalette("reference");
  474 |     return { legacy, reference: getComputedStyle(document.documentElement).getPropertyValue("--bg").trim(), saved: localStorage.getItem("runalytics.palette") };
  475 |   });
  476 |   expect(colours.reference).toBe("#f5f5f7");
  477 |   expect(colours.legacy).not.toBe(colours.reference);
  478 |   expect(colours.saved).toBe("reference");
  479 | });
  480 | 
  481 | for (const width of [320, 390, 768, 1440]) {
  482 |   test(`all screens fit ${width}px in both themes`, async ({ page }) => {
  483 |     await page.setViewportSize({ width, height: 1000 });
  484 |     const failures = [];
  485 |     for (const route of ["index.html", "plan.html", "calendar.html", "achievements.html", "settings.html", "injury.html", "support.html"]) {
  486 |       await page.goto(`/${route}`);
  487 |       await expect(page.locator(".page-header")).toBeVisible();
  488 |       for (const theme of ["light", "dark"]) {
  489 |         await page.evaluate(async (choice) => {
  490 |           const { setTheme } = await import("/src/lib/theme.ts");
  491 |           setTheme(choice);
  492 |         }, theme);
  493 |         const checks = await page.evaluate(() => {
  494 |           const ids = [...document.querySelectorAll("[id]")].map(node => node.id);
  495 |           return {
  496 |             overflow: document.documentElement.scrollWidth > innerWidth,
  497 |             duplicateIds: ids.filter((id, index) => ids.indexOf(id) !== index),
  498 |             visibleHidden: [...document.querySelectorAll("[hidden]")].filter(node => getComputedStyle(node).display !== "none").length,
  499 |           };
  500 |         });
  501 |         if (checks.overflow || checks.duplicateIds.length || checks.visibleHidden) failures.push({ route, theme, ...checks });
  502 |         if (width === 390 || width === 1440) {
  503 |           await page.screenshot({ path: `test-results/premium-${route}-${theme}-${width}.png`, fullPage: true });
  504 |         }
  505 |       }
  506 |     }
  507 |     expect(failures).toEqual([]);
  508 |   });
  509 | }
  510 | 
  511 | test("primary screens meet automated WCAG AA checks", async ({ page }) => {
  512 |   const failures = [];
  513 |   for (const route of ["index.html", "plan.html", "calendar.html", "achievements.html", "settings.html", "injury.html", "support.html"]) {
  514 |     await page.goto(`/${route}`);
  515 |     await expect(page.locator(".page-header")).toBeVisible();
  516 |     for (const theme of ["light", "dark"]) {
> 517 |       await page.evaluate(async (choice) => {
      |                  ^ Error: page.evaluate: Execution context was destroyed, most likely because of a navigation
  518 |         const { setTheme } = await import("/src/lib/theme.ts");
  519 |         setTheme(choice);
  520 |       }, theme);
  521 |       const result = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze();
  522 |       failures.push(...result.violations.map(violation => ({ route, theme, id: violation.id, nodes: violation.nodes.map(node => ({target:node.target,summary:node.failureSummary})) })));
  523 |     }
  524 |   }
  525 |   expect(failures).toEqual([]);
  526 | });
  527 | 
  528 | for (const width of [320, 390, 768, 1440]) {
  529 |   test(`daily signal labels and values do not collide at ${width}px`, async ({ page }) => {
  530 |     await page.setViewportSize({ width, height: 900 });
  531 |     await page.goto("/");
  532 |     await expect(page.locator(".pillar")).toHaveCount(3);
  533 |     for (const pillar of await page.locator(".pillar").all()) {
  534 |       const label = await pillar.locator(".pillar-name").boundingBox();
  535 |       const value = await pillar.locator("strong").boundingBox();
  536 |       const dial = await pillar.locator(".signal-dial").boundingBox();
  537 |       expect(label.y + label.height).toBeLessThanOrEqual(value.y);
  538 |       expect(value.x + value.width).toBeLessThanOrEqual(dial.x);
  539 |     }
  540 |     await expect(page.locator(".today-workout .session-open").first()).toBeVisible();
  541 |     await expect(page.locator(".session-metrics").first()).toContainText("Distance");
  542 |   });
  543 | }
  544 | 
  545 | test("premium surfaces provide interaction motion and respect reduced motion", async ({ page }) => {
  546 |   await page.emulateMedia({ reducedMotion: "no-preference" });
  547 |   await page.goto("/plan.html");
  548 |   const session = page.locator(".plan-session").first();
  549 |   await expect(session).toBeVisible();
  550 |   expect(await session.evaluate(node => getComputedStyle(node).boxShadow)).not.toBe("none");
  551 |   expect(await session.evaluate(node => getComputedStyle(node).transitionDuration)).toContain("0.2s");
  552 |   await session.click();
  553 |   const card = page.locator("dialog[open] .workout-card");
  554 |   await expect(card).toBeVisible();
  555 |   expect(await card.evaluate(node => getComputedStyle(node).animationName)).toBe("premium-open");
  556 |   await page.getByRole("button", { name: "Close workout", exact: true }).click();
  557 |   await page.emulateMedia({ reducedMotion: "reduce" });
  558 |   await expect(session).toHaveCSS("transition-duration", "0s");
  559 |   await session.hover();
  560 |   await expect(session).toHaveCSS("transform", "none");
  561 |   await session.click();
  562 |   await expect(card).toHaveCSS("animation-name", "none");
  563 | });
  564 | 
  565 | for (const width of [390, 1440]) {
  566 |   test(`expanded analytics remain accessible and fit ${width}px`, async ({ page }) => {
  567 |     await page.setViewportSize({ width, height: 1000 });
  568 |     await page.goto("/achievements.html#performance");
  569 |     await expect(page.locator("#status-panel .band-value")).toBeVisible();
  570 |     for (const detail of await page.locator(".analysis-detail").all()) {
  571 |       await detail.locator(":scope > summary").click();
  572 |       await expect(detail).toHaveAttribute("open");
  573 |     }
  574 |     for (const theme of ["light", "dark"]) {
  575 |       await page.evaluate(async choice => {
  576 |         const { setTheme } = await import("/src/lib/theme.ts");
  577 |         setTheme(choice);
  578 |       }, theme);
  579 |       expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  580 |       const results = await new AxeBuilder({ page }).include("#performance-analysis").withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze();
  581 |       expect(results.violations.map(violation => ({id:violation.id, nodes:violation.nodes.map(node => ({target:node.target, summary:node.failureSummary}))}))).toEqual([]);
  582 |       await page.screenshot({ path: `test-results/premium-performance-${theme}-${width}.png`, fullPage: true });
  583 |     }
  584 |   });
  585 | }
  586 | 
  587 | test("reference palette keeps mobile status chrome consistent with appearance", async ({ page }) => {
  588 |   await page.goto("/support.html");
  589 |   for (const [theme, colour] of [["light", "#f5f5f7"], ["dark", "#161618"]]) {
  590 |     await page.evaluate(async choice => {
  591 |       const { setTheme, setPalette } = await import("/src/lib/theme.ts");
  592 |       setPalette("reference");
  593 |       setTheme(choice);
  594 |     }, theme);
  595 |     await expect(page.locator('meta[name="theme-color"]')).toHaveAttribute("content", colour);
  596 |   }
  597 | });
  598 | 
  599 | for (const width of [390, 1440]) {
  600 |   test(`system materials and accessibility preferences work at ${width}px`, async ({ page, context }) => {
  601 |     await page.setViewportSize({ width, height: 1000 });
  602 |     await page.goto("/");
  603 |     await expect(page.locator(".pillar")).toHaveCount(3);
  604 |     const nav = page.locator("#nav");
  605 |     await expect(page.locator("body")).toHaveCSS("font-family", /-apple-system/);
  606 |     await expect(page.locator(".pillar").first()).toHaveCSS("border-radius", "0px");
  607 |     await expect(page.locator(".pillar").first()).toHaveCSS("box-shadow", "none");
  608 |     await expect(nav).toHaveCSS("backdrop-filter", /blur\(28px\)/);
  609 |     const session = await context.newCDPSession(page);
  610 |     for (const preference of ["prefers-reduced-transparency", "prefers-contrast"]) {
  611 |       const value = preference === "prefers-contrast" ? "more" : "reduce";
  612 |       await session.send("Emulation.setEmulatedMedia", { features: [{ name: preference, value }] });
  613 |       expect(await page.evaluate(query => matchMedia(query).matches, `(${preference}: ${value})`)).toBe(true);
  614 |       await expect(nav).toHaveCSS("backdrop-filter", "none");
  615 |       await expect(nav).toHaveCSS("background-color", "rgb(255, 255, 255)");
  616 |       if (width === 390) {
  617 |         await page.locator(".nav-more").click();
```