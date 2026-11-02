# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> map theme overrides preserve contrasting routes and circular finish at 390px
- Location: tests/desk.spec.mjs:568:3

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#journal/1
Call log:
  - navigating to "http://127.0.0.1:1500/desk.html#journal/1", waiting until "load"

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
  571 |       const canvas = document.createElement("canvas");
  572 |       canvas.width = canvas.height = 256;
  573 |       const context = canvas.getContext("2d");
  574 |       context.fillStyle = "#e5e7eb";
  575 |       context.fillRect(0, 0, 256, 256);
  576 |       return canvas.toDataURL().split(",")[1];
  577 |     });
  578 |     await page.route("https://tile.openstreetmap.org/**", route => route.fulfill({ contentType: "image/png", body: Buffer.from(tile, "base64") }));
  579 |     await page.route("**/src/desk/map.tsx*", async route => {
  580 |       const response = await route.fetch();
  581 |       await route.fulfill({ response, body: `${await response.text()}
  582 |         const originalPaint = maplibregl.Map.prototype.setPaintProperty;
  583 |         maplibregl.Map.prototype.setPaintProperty = function(...args) {
  584 |           window.__themeMap = this;
  585 |           return originalPaint.apply(this, args);
  586 |         };
  587 |       ` });
  588 |     });
> 589 |     await page.goto("/desk.html#journal/1");
      |                ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#journal/1
  590 |     const finish = page.getByRole("img", { name: "Route finish", exact: true });
  591 |     await expect(finish).toBeVisible();
  592 |     await expect(finish).toHaveCSS("border-radius", "50%");
  593 |     await expect(finish).toHaveCSS("width", "24px");
  594 |     await expect(finish).toHaveCSS("height", "24px");
  595 |     const start = page.getByRole("img", { name: "Route start", exact: true });
  596 |     await expect(start).toHaveCSS("width", "24px");
  597 |     await expect(start).toHaveCSS("height", "24px");
  598 |     await expect(finish.locator("svg")).toHaveCSS("clip-path", "circle(50%)");
  599 |     await expect(finish.locator(".flag-pole")).toHaveCount(0);
  600 |     await expect(finish.locator(".flag-check")).toHaveCount(1);
  601 |     await page.evaluate(() => { window.__originalThemeMap = window.__themeMap; });
  602 |     const camera = await page.evaluate(() => ({ center: window.__themeMap.getCenter().toArray(), zoom: window.__themeMap.getZoom() }));
  603 |     for (const theme of ["light", "dark"]) {
  604 |       for (const palette of ["reference", "amethyst", "glacier", "evergreen", "sakura", "ember"]) {
  605 |         await page.evaluate(({ theme, palette }) => {
  606 |           document.documentElement.dataset.deskTheme = theme;
  607 |           document.documentElement.dataset.deskPalette = palette;
  608 |         }, { theme, palette });
  609 |         await expect.poll(() => page.evaluate(() => window.__themeMap.getPaintProperty("base", "raster-brightness-max"))).toBe(theme === "dark" ? .32 : 1);
  610 |         const colors = await page.evaluate(() => {
  611 |           const instance = window.__themeMap;
  612 |           const route = instance.getPaintProperty("route", "line-color");
  613 |           const outline = instance.getPaintProperty("route-outline", "line-color");
  614 |           const luminance = hex => {
  615 |             const full = hex.length === 4 ? `#${[...hex.slice(1)].map(channel => channel + channel).join("")}` : hex;
  616 |             return [1, 3, 5].map(offset => parseInt(full.slice(offset, offset + 2), 16) / 255).map(channel => channel <= .04045 ? channel / 12.92 : ((channel + .055) / 1.055) ** 2.4).reduce((total, channel, index) => total + channel * [.2126, .7152, .0722][index], 0);
  617 |           };
  618 |           const values = [luminance(route), luminance(outline)].sort((first, second) => second - first);
  619 |           return { ratio: (values[0] + .05) / (values[1] + .05), route, action: getComputedStyle(document.documentElement).getPropertyValue("--desk-action").trim(), gradient: instance.getPaintProperty("route", "line-gradient"), same: instance === window.__originalThemeMap, center: instance.getCenter().toArray(), zoom: instance.getZoom() };
  620 |         });
  621 |         expect(colors.route).toBe(colors.action);
  622 |         expect(colors.gradient).toEqual(["to-color", colors.action]);
  623 |         expect(colors.ratio, `${theme}/${palette}`).toBeGreaterThan(3);
  624 |         expect(colors.same).toBe(true);
  625 |         expect(colors.center).toEqual(camera.center);
  626 |         expect(colors.zoom).toBe(camera.zoom);
  627 |       }
  628 |       await expect.poll(() => page.locator(".maplibregl-canvas").evaluate(canvas => {
  629 |         const context = canvas.getContext("webgl2");
  630 |         const pixel = new Uint8Array(4);
  631 |         context.readPixels(10, 10, 1, 1, context.RGBA, context.UNSIGNED_BYTE, pixel);
  632 |         return pixel[3] === 255 && (document.documentElement.dataset.deskTheme === "dark" ? pixel[0] < 110 : pixel[0] > 180);
  633 |       })).toBe(true);
  634 |       await page.screenshot({ path: `test-results/map-theme-${theme}-${width}.png` });
  635 |     }
  636 |   });
  637 | }
  638 | 
  639 | for (const [width, pointCount, span] of [[390, 2, .001], [1440, 1001, .5]]) {
  640 |   test(`activity route draws with ease-in-out in three seconds at ${width}px with ${pointCount} GPS points`, async ({page}) => {
  641 |     await page.setViewportSize({width, height:1000});
  642 |     await page.emulateMedia({reducedMotion:'no-preference'});
  643 |     const tile = await page.evaluate(() => {
  644 |       const canvas = document.createElement('canvas');
  645 |       canvas.width = canvas.height = 256;
  646 |       const context = canvas.getContext('2d');
  647 |       context.fillStyle = '#e5e7eb';
  648 |       context.fillRect(0, 0, 256, 256);
  649 |       return canvas.toDataURL().split(',')[1];
  650 |     });
  651 |     await page.route('https://tile.openstreetmap.org/**', route => route.fulfill({status:200, contentType:'image/png', body:Buffer.from(tile, 'base64')}));
  652 |     await page.route('**/src/desk/map.tsx*', async route => {
  653 |       const response = await route.fetch();
  654 |       await route.fulfill({response, body:`${await response.text()}
  655 |         const originalSetPaint = maplibregl.Map.prototype.setPaintProperty;
  656 |         maplibregl.Map.prototype.setPaintProperty = function(layer, property, value, ...options) {
  657 |           const result = originalSetPaint.call(this, layer, property, value, ...options);
  658 |           if (layer === 'route' && property === 'line-gradient') {
  659 |             window.__routeTestMap = this;
  660 |             (window.__routeFrames ??= []).push({time:performance.now(), progress:value[0] === 'step' ? value[3] : 1});
  661 |           }
  662 |           return result;
  663 |         };
  664 |       `});
  665 |     });
  666 |     await page.addInitScript(({pointCount, span}) => {
  667 |       const invoke = window.__TAURI_INTERNALS__.invoke;
  668 |       window.__TAURI_INTERNALS__.invoke = async (command, args) => {
  669 |         const result = await invoke(command, args);
  670 |         return command === 'activity_review' ? {...result, track:Array.from({length:pointCount}, (_, index) => ({lat:38.72, lon:-9.14 + span * index / (pointCount - 1)}))} : result;
  671 |       };
  672 |     }, {pointCount, span});
  673 |     const errors = [];
  674 |     page.on('pageerror', error => errors.push(error.message));
  675 |     await page.clock.install();
  676 |     await page.goto('/desk.html#journal/1');
  677 |     await expect.poll(() => page.evaluate(() => window.__routeFrames?.length ?? 0)).toBeGreaterThan(1);
  678 |     await page.clock.pauseAt(await page.evaluate(() => Date.now() + 100));
  679 |     const elapsed = await page.evaluate(() => performance.now() - window.__routeFrames[0].time);
  680 |     expect(elapsed).toBeLessThan(1500);
  681 |     await page.clock.runFor(1500 - elapsed);
  682 |     const progress = () => page.evaluate(() => window.__routeFrames?.at(-1)?.progress);
  683 |     expect(await progress()).toBeCloseTo(.5, 1);
  684 |     expect(await page.evaluate(() => window.__routeTestMap.getPaintProperty('route-outline','line-gradient')[3])).toBeCloseTo(.5, 1);
  685 |     const pixels = () => page.locator('.maplibregl-canvas').evaluate(canvas => {
  686 |       const probe = document.createElement('canvas');
  687 |       probe.width = probe.height = 1;
  688 |       const paint = probe.getContext('2d');
  689 |       paint.fillStyle = getComputedStyle(document.documentElement).getPropertyValue('--desk-action').trim();
```