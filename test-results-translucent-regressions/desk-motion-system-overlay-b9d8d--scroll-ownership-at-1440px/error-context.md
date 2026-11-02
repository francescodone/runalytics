# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> motion system >> overlays retain their exit, focus and scroll ownership at 1440px
- Location: tests/desk.spec.mjs:467:5

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#health/pace
Call log:
  - navigating to "http://127.0.0.1:1500/desk.html#health/pace", waiting until "load"

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
  370 |         await expect(page.getByRole("dialog")).toHaveCount(0);
  371 |         await page.getByRole("link", { name: "Training", exact: true }).click();
  372 |         if (width < 800) await page.locator(".mobile-date-ribbon button").first().click();
  373 |         await page.locator(width < 800 ? ".day-itinerary .session-tile" : ".week-board .session-tile").first().click();
  374 |         await expect(page.locator(".desk-inspector")).toBeVisible();
  375 |         await page.screenshot({ path: `test-results/motion-inspector-${theme}-${width}.png`, animations: "disabled" });
  376 |         await page.keyboard.press("Escape");
  377 |         await expect(page.locator(".desk-inspector")).toHaveCount(0);
  378 |         expect(errors).toEqual([]);
  379 |       });
  380 |     }
  381 |   }
  382 | 
  383 |   test("selection continuity, tooltips and resize remain interruptible", async ({ page }) => {
  384 |     await page.setViewportSize({ width: 1440, height: 900 });
  385 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  386 |     await page.goto("/desk.html#training");
  387 |     const navigation = page.getByRole("navigation", { name: "Workspaces" });
  388 |     for (const workspace of ["Activities", "Health", "Training", "Health"]) {
  389 |       await navigation.getByRole("link", { name: workspace, exact: true }).click();
  390 |       await expect(page.locator("#workspace h1")).toHaveText(workspace);
  391 |     }
  392 |     const active = navigation.locator("a[aria-current]");
  393 |     await expect.poll(async () => {
  394 |       const link = await active.boundingBox();
  395 |       const marker = await active.locator(".selection-marker").boundingBox();
  396 |       const border = await active.evaluate(element => parseFloat(getComputedStyle(element).borderLeftWidth) * (parseFloat(getComputedStyle(element).scale) || 1));
  397 |       return Math.abs(link.x + border - marker.x) + Math.abs(link.width - 2 * border - marker.width);
  398 |     }).toBeLessThan(1);
  399 |     const help = page.getByRole("button", { name: "Help", exact: true });
  400 |     await help.hover();
  401 |     await expect(help.locator(".control-tooltip")).toBeVisible();
  402 |     await page.keyboard.press("Escape");
  403 |     await expect(help.locator(".control-tooltip")).toBeHidden();
  404 |     await page.mouse.move(100, 200);
  405 |     await expect(help.locator(".control-tooltip")).toBeHidden();
  406 |     await help.focus();
  407 |     await page.keyboard.press("Tab");
  408 |     await page.keyboard.press("Shift+Tab");
  409 |     await expect(help).toBeFocused();
  410 |     await expect(help.locator(".control-tooltip")).toBeVisible();
  411 |     await navigation.getByRole("link", { name: "Training", exact: true }).click();
  412 |     await page.locator(".week-board .session-tile").first().click();
  413 |     await expect(page.locator(".desk-inspector")).toBeVisible();
  414 |     await page.setViewportSize({ width: 390, height: 844 });
  415 |     await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe("hidden");
  416 |     await page.emulateMedia({ reducedMotion: "reduce" });
  417 |     await expect.poll(async () => Math.round((await page.locator(".desk-inspector").boundingBox()).x)).toBe(0);
  418 |     await page.keyboard.press("Escape");
  419 |     await expect(page.locator(".desk-inspector")).toHaveCount(0);
  420 |     expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
  421 |   });
  422 | 
  423 |   test("buttons preserve geometry through pending, success, error and keyboard retry", async ({ page }) => {
  424 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  425 |     await page.addInitScript(() => {
  426 |       const invoke = window.__TAURI_INTERNALS__.invoke;
  427 |       window.__TAURI_INTERNALS__.invoke = async (command, args) => {
  428 |         if (command === "save_settings") await new Promise(resolve => { window.__releaseMotionAction = resolve; });
  429 |         return invoke(command, args);
  430 |       };
  431 |     });
  432 |     await page.goto("/desk.html#settings/maps");
  433 |     const save = page.getByRole("button", { name: "Save map source", exact: true });
  434 |     await expect(save).toBeVisible();
  435 |     await page.evaluate(() => document.fonts.ready);
  436 |     const layout = () => save.evaluate(element => ({ width: element.offsetWidth, height: element.offsetHeight }));
  437 |     const resting = await layout();
  438 |     await save.click();
  439 |     await expect(save).toHaveAttribute("aria-busy", "true");
  440 |     await expect(save).toBeDisabled();
  441 |     expect(await layout()).toEqual(resting);
  442 |     await page.evaluate(() => window.__releaseMotionAction());
  443 |     await expect(save).toHaveAttribute("data-result", "success");
  444 |     await expect(save).toBeEnabled();
  445 |     expect(await layout()).toEqual(resting);
  446 |     await expect(page.getByRole("status")).toContainText("Map source saved");
  447 |     await page.evaluate(() => { window.__deskFail = "save_settings"; });
  448 |     await page.getByLabel("Raster PMTiles archive").fill("/synthetic/map.pmtiles");
  449 |     await page.getByLabel("Raster PMTiles archive").press("Enter");
  450 |     await expect(save).toHaveAttribute("aria-busy", "true");
  451 |     await page.evaluate(() => window.__releaseMotionAction());
  452 |     await expect(save).toHaveAttribute("data-result", "error");
  453 |     await expect(page.getByRole("alert")).toContainText("Unable to save");
  454 |     await expect(page.getByLabel("Raster PMTiles archive")).toHaveValue("/synthetic/map.pmtiles");
  455 |     expect(await layout()).toEqual(resting);
  456 |     await page.evaluate(() => { window.__deskFail = ""; });
  457 |     await save.press("Enter");
  458 |     await page.evaluate(() => window.__releaseMotionAction());
  459 |     await expect(save).toHaveAttribute("data-result", "success");
  460 |     const notice = page.locator(".desk-notice");
  461 |     await page.getByRole("button", { name: "Dismiss notification" }).evaluate(button => button.click());
  462 |     await expect(notice).toBeAttached();
  463 |     await expect(notice).toHaveCount(0);
  464 |   });
  465 | 
  466 |   for (const width of [390, 1440]) {
  467 |     test(`overlays retain their exit, focus and scroll ownership at ${width}px`, async ({ page }) => {
  468 |       await page.setViewportSize({ width, height: 900 });
  469 |       await page.emulateMedia({ reducedMotion: "no-preference" });
> 470 |       await page.goto("/desk.html#health/pace");
      |                  ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#health/pace
  471 |       const opener = page.getByRole("button", { name: "Log pain", exact: true });
  472 |       await opener.click();
  473 |       const dialog = page.getByRole("dialog", { name: "Log pain" });
  474 |       await expect(dialog).toBeVisible();
  475 |       await expect.poll(async () => Math.round((await dialog.boundingBox()).x)).toBe(width === 390 ? 0 : 420);
  476 |       await page.getByLabel("Notes", { exact: true }).fill("Retained while the surface exits");
  477 |       await page.getByRole("button", { name: "Cancel", exact: true }).evaluate(button => button.click());
  478 |       await expect(dialog).toHaveAttribute("data-state", "closing");
  479 |       await expect(dialog).toContainText("Save pain entry");
  480 |       expect(await page.evaluate(() => document.body.style.overflow)).toBe("hidden");
  481 |       await expect(dialog).toHaveCount(0);
  482 |       await expect(opener).toBeFocused();
  483 |       expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
  484 |       await opener.click();
  485 |       await page.keyboard.press("Escape");
  486 |       await expect(dialog).toHaveCount(0);
  487 |       await expect(opener).toBeFocused();
  488 |       expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
  489 |     });
  490 |   }
  491 | 
  492 |   test("touch surfaces track, cancel and dismiss without hijacking form controls", async ({ page }) => {
  493 |     await page.setViewportSize({ width: 390, height: 844 });
  494 |     await page.emulateMedia({ reducedMotion: "no-preference" });
  495 |     const session = await page.context().newCDPSession(page);
  496 |     await session.send("Emulation.setTouchEmulationEnabled", { enabled: true });
  497 |     const frame = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(resolve)));
  498 |     const touch = (type, x, y) => session.send("Input.dispatchTouchEvent", { type, touchPoints: type === "touchEnd" || type === "touchCancel" ? [] : [{ x, y }] });
  499 |     await page.goto("/desk.html#health/pace");
  500 |     const opener = page.getByRole("button", { name: "Log pain", exact: true });
  501 |     await opener.click();
  502 |     const dialog = page.locator(".desk-dialog");
  503 |     const grip = page.locator(".dialog-grip");
  504 |     await expect.poll(async () => Math.round((await dialog.boundingBox()).y + (await dialog.boundingBox()).height)).toBe(844);
  505 |     const rest = await dialog.boundingBox();
  506 |     const handle = await grip.boundingBox();
  507 |     const start = { x: handle.x + handle.width / 2, y: handle.y + handle.height / 2 };
  508 |     await touch("touchStart", start.x, start.y);
  509 |     await touch("touchMove", start.x, start.y + 12);
  510 |     await frame();
  511 |     await touch("touchMove", start.x, start.y + 48);
  512 |     await frame();
  513 |     expect((await dialog.boundingBox()).y - rest.y).toBeGreaterThan(20);
  514 |     await touch("touchCancel", start.x, start.y + 48);
  515 |     await expect(dialog).toHaveCount(1);
  516 |     await expect.poll(async () => Math.round((await dialog.boundingBox()).y)).toBe(Math.round(rest.y));
  517 |     await page.getByLabel("Notes", { exact: true }).fill("Touch input still works");
  518 |     await touch("touchStart", start.x, start.y);
  519 |     await touch("touchMove", start.x, start.y + 16);
  520 |     await frame();
  521 |     await touch("touchMove", start.x, start.y + 150);
  522 |     await frame();
  523 |     expect((await dialog.boundingBox()).y - rest.y).toBeGreaterThan(60);
  524 |     await touch("touchEnd", start.x, start.y + 150);
  525 |     await expect(dialog).toHaveCount(0);
  526 |     await expect(opener).toBeFocused();
  527 |     await page.goto("/desk.html#training");
  528 |     await page.locator(".mobile-date-ribbon button").first().click();
  529 |     const tile = page.locator(".day-itinerary .session-tile").first();
  530 |     await tile.click();
  531 |     const inspector = page.locator(".desk-inspector");
  532 |     await expect.poll(async () => Math.round((await inspector.boundingBox()).x)).toBe(0);
  533 |     const bar = await page.locator(".inspector-bar > span").boundingBox();
  534 |     const origin = { x: bar.x + bar.width / 2, y: bar.y + bar.height / 2 };
  535 |     await touch("touchStart", origin.x, origin.y);
  536 |     await touch("touchMove", origin.x + 16, origin.y);
  537 |     await frame();
  538 |     await touch("touchMove", origin.x + 140, origin.y);
  539 |     await frame();
  540 |     expect((await inspector.boundingBox()).x).toBeGreaterThan(40);
  541 |     await touch("touchEnd", origin.x + 140, origin.y);
  542 |     await expect(inspector).toHaveCount(0);
  543 |     await expect(tile).toBeFocused();
  544 |     expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
  545 |     await session.detach();
  546 |   });
  547 | 
  548 |   test("reduced motion keeps feedback and keyboard interaction without spatial animation", async ({ page }) => {
  549 |     await page.emulateMedia({ reducedMotion: "reduce" });
  550 |     await page.goto("/desk.html#health/pace");
  551 |     await page.getByRole("button", { name: "Log pain", exact: true }).click();
  552 |     const dialog = page.getByRole("dialog");
  553 |     const geometry = await dialog.boundingBox();
  554 |     expect(Math.round(geometry.x)).toBe(0);
  555 |     expect(Math.round(geometry.y + geometry.height)).toBe(844);
  556 |     expect(await dialog.evaluate(element => getComputedStyle(element).transform)).toBe("none");
  557 |     await page.keyboard.press("Escape");
  558 |     await expect(dialog).toHaveCount(0);
  559 |     await page.getByRole("button", { name: "Long", exact: true }).focus();
  560 |     await page.keyboard.press("Enter");
  561 |     await expect(page.getByRole("button", { name: "Long", exact: true })).toHaveAttribute("aria-pressed", "true");
  562 |     await expect(page.getByRole("region", { name: "Selected pace session", exact: true })).toContainText("Race-pace long run");
  563 |     expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  564 |   });
  565 | });
  566 | 
  567 | for (const width of [390, 1440]) {
  568 |   test(`map theme overrides preserve contrasting routes and circular finish at ${width}px`, async ({ page }) => {
  569 |     await page.setViewportSize({ width, height: 1000 });
  570 |     const tile = await page.evaluate(() => {
```