# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> activity route draws with ease-in-out in three seconds at 390px with 2 GPS points
- Location: tests/desk.spec.mjs:640:3

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
  589 |     await page.goto("/desk.html#journal/1");
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
> 676 |     await page.goto('/desk.html#journal/1');
      |                ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#journal/1
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
  690 |       paint.fillRect(0, 0, 1, 1);
  691 |       const accent = paint.getImageData(0, 0, 1, 1).data;
  692 |       const context = canvas.getContext('webgl2');
  693 |       const bytes = new Uint8Array(context.drawingBufferWidth * context.drawingBufferHeight * 4);
  694 |       context.readPixels(0, 0, context.drawingBufferWidth, context.drawingBufferHeight, context.RGBA, context.UNSIGNED_BYTE, bytes);
  695 |       const counts = {start:0, end:0};
  696 |       for (let index = 0; index < bytes.length; index += 4) {
  697 |         if (bytes[index + 3] > 240 && [0, 1, 2].every(channel => Math.abs(bytes[index + channel] - accent[channel]) < 20)) {
  698 |           counts[(index / 4) % context.drawingBufferWidth < context.drawingBufferWidth / 2 ? 'start' : 'end']++;
  699 |         }
  700 |       }
  701 |       return counts;
  702 |     });
  703 |     await expect.poll(async () => (await pixels()).start).toBeGreaterThan(50);
  704 |     const halfway = await pixels();
  705 |     expect(halfway.end).toBeLessThan(halfway.start / 4);
  706 |     await page.screenshot({path:`test-results/desk-route-half-${width}.png`});
  707 |     await page.clock.runFor(1400);
  708 |     expect(await progress()).toBeLessThan(1);
  709 |     await page.clock.runFor(150);
  710 |     expect(await progress()).toBe(1);
  711 |     const frames = await page.evaluate(() => window.__routeFrames);
  712 |     expect(frames[0].progress).toBe(0);
  713 |     expect(frames.at(-1).time - frames[0].time).toBeGreaterThanOrEqual(3000);
  714 |     expect(frames.at(-1).time - frames[0].time).toBeLessThan(3050);
  715 |     expect(frames.find(frame => frame.time - frames[0].time >= 750).progress).toBeCloseTo(.15625, 1);
  716 |     expect(frames.find(frame => frame.time - frames[0].time >= 2250).progress).toBeCloseTo(.84375, 1);
  717 |     expect(frames.every((frame, index) => index === 0 || frame.progress >= frames[index - 1].progress)).toBe(true);
  718 |     await expect.poll(async () => (await pixels()).end).toBeGreaterThan(halfway.start / 2);
  719 |     await page.screenshot({path:`test-results/desk-route-complete-${width}.png`});
  720 |     await page.getByRole('button',{name:'Back',exact:true}).click();
  721 |     await page.clock.runFor(250);
  722 |     await expect(page.locator('.maplibregl-canvas')).toHaveCount(0);
  723 |     await page.evaluate(() => {window.__routeFrames = [];});
  724 |     await page.clock.resume();
  725 |     await page.locator('.activity-row').first().click();
  726 |     await expect.poll(() => page.evaluate(() => window.__routeFrames.length)).toBeGreaterThan(1);
  727 |     expect(await progress()).toBeLessThan(.5);
  728 |     await page.getByRole('button',{name:'Back',exact:true}).click();
  729 |     await expect(page.locator('.maplibregl-canvas')).toHaveCount(0);
  730 |     const stopped = await page.evaluate(() => window.__routeFrames.length);
  731 |     await page.clock.runFor(3100);
  732 |     expect(await page.evaluate(() => window.__routeFrames.length)).toBe(stopped);
  733 |     await page.emulateMedia({reducedMotion:'reduce'});
  734 |     await page.evaluate(() => {window.__routeFrames = [];});
  735 |     await page.locator('.activity-row').first().click();
  736 |     await expect.poll(progress).toBe(1);
  737 |     expect(await page.evaluate(() => window.__routeFrames.map(frame => frame.progress))).toEqual([1]);
  738 |     await expect(page.locator('.route-figure figcaption')).toHaveCount(0);
  739 |     expect(errors).toEqual([]);
  740 |   });
  741 | }
  742 | 
  743 | test('graded performance proposals preview race estimates and only change them after approval', async ({page}) => {
  744 |   await page.goto('/desk.html?pacingProposal=1#health/pace');
  745 |   await page.getByRole('button',{name:'Review changes',exact:true}).click();
  746 |   const preview = page.getByRole('region',{name:'Proposed race estimates'});
  747 |   await expect(preview).toBeVisible();
  748 |   const times = await preview.locator('tbody tr').evaluateAll(rows => rows.map(row => [...row.querySelectorAll('td')].map(cell => Number(cell.dataset.seconds))));
  749 |   expect(times).toHaveLength(4);
  750 |   expect(times.every(([before,after]) => after < before && after > before*.95)).toBe(true);
  751 |   expect(await page.evaluate(async () => {
  752 |     const {trainingRaceTime}=await import('/src/desk/race-estimates.tsx');
  753 |     return [5,10,21.0975,42.195].map(distance=>trainingRaceTime(285,0,distance));
  754 |   })).toEqual([1348,2810,6201,12928]);
  755 |   await expect(page.getByRole('region',{name:'Proposed session changes'})).toContainText('10.3 km');
  756 |   await expect(page.getByRole('region',{name:'Proposed session changes'})).toContainText('4:30 - 4:40/km');
  757 |   expect(await page.evaluate(()=>window.__deskCalls.filter(call=>call.command==='apply_proposal'))).toHaveLength(0);
  758 |   await page.evaluate(()=>{window.__deskFail='apply_proposal';});
  759 |   await page.getByRole('button',{name:'Apply changes',exact:true}).click();
  760 |   await expect(page.getByRole('dialog').getByRole('alert')).toBeVisible();
  761 |   await expect(preview).toBeVisible();
  762 |   await page.evaluate(()=>{window.__deskFail='';});
  763 |   await page.getByRole('button',{name:'Apply changes',exact:true}).click();
  764 |   await expect(page.getByRole('dialog')).toHaveCount(0);
  765 |   await page.getByRole('link',{name:'Overview',exact:true}).click();
  766 |   await page.getByRole('button',{name:'Show health details',exact:true}).click();
  767 |   const estimates=page.getByRole('region',{name:'Training race estimates'});
  768 |   await expect(estimates).toBeVisible();
  769 |   expect(await estimates.locator('tbody td').evaluateAll(cells=>cells.map(cell=>Number(cell.dataset.seconds)))).toEqual(times.map(values=>values[1]));
  770 | });
  771 | 
  772 | for (const width of [320,1440]) {
  773 |   test(`percentage charts and tinted glass respect accessibility preferences at ${width}px`, async ({page}) => {
  774 |     await page.setViewportSize({width,height:1000});
  775 |     for (const theme of ['light','dark']) {
  776 |       await page.goto(`/desk.html?theme=${theme}&palette=amethyst#health`);
```