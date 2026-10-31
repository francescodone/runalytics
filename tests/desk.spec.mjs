import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { installDeskFixture } from "./desk.fixture.mjs";

test.beforeEach(async ({ page }) => installDeskFixture(page));

test.describe("motion system", () => {
  for (const width of [390, 1440]) {
    for (const theme of ["light", "dark"]) {
      test(`complete motion-enabled workspace pass ${theme} ${width}px`, async ({ page }) => {
        test.setTimeout(90000);
        await page.setViewportSize({ width, height: 1000 });
        await page.emulateMedia({ reducedMotion: "no-preference" });
        const errors = [];
        page.on("pageerror", error => errors.push(error.message));
        for (const route of ["training", "training/month", "training/plan", "journal", "journal/map", "health", "health/pace", "health/journal", "health/milestones", "settings", "settings/appearance", "help"]) {
          await page.goto(`/desk.html?theme=${theme}#${route}`);
          await expect(page.locator("#workspace h1")).toBeVisible();
          await expect(page.locator(".desk-loading")).toHaveCount(0);
          expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), route).toBe(true);
          expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations, route).toEqual([]);
          await page.screenshot({ path: `test-results/motion-${route.replaceAll("/", "-")}-${theme}-${width}.png`, fullPage: true, animations: "disabled" });
        }
        await page.goto(`/desk.html?theme=${theme}#health/pace`);
        await page.getByRole("button", { name: "Log pain", exact: true }).click();
        await expect(page.getByRole("dialog")).toBeVisible();
        await page.screenshot({ path: `test-results/motion-sheet-${theme}-${width}.png`, animations: "disabled" });
        await page.keyboard.press("Escape");
        await expect(page.getByRole("dialog")).toHaveCount(0);
        await page.getByRole("link", { name: "Training", exact: true }).click();
        if (width < 800) await page.locator(".mobile-date-ribbon button").first().click();
        await page.locator(width < 800 ? ".day-itinerary .session-tile" : ".week-board .session-tile").first().click();
        await expect(page.locator(".desk-inspector")).toBeVisible();
        await page.screenshot({ path: `test-results/motion-inspector-${theme}-${width}.png`, animations: "disabled" });
        await page.keyboard.press("Escape");
        await expect(page.locator(".desk-inspector")).toHaveCount(0);
        expect(errors).toEqual([]);
      });
    }
  }

  test("selection continuity, tooltips and resize remain interruptible", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto("/desk.html#training");
    const navigation = page.getByRole("navigation", { name: "Workspaces" });
    for (const workspace of ["Journal", "Health", "Training", "Health"]) {
      await navigation.getByRole("link", { name: workspace, exact: true }).click();
      await expect(page.locator("#workspace h1")).toHaveText(workspace);
    }
    const active = navigation.locator("a[aria-current]");
    await expect.poll(async () => {
      const link = await active.boundingBox();
      const marker = await active.locator(".selection-marker").boundingBox();
      return Math.abs(link.x - marker.x) + Math.abs(link.width - marker.width);
    }).toBeLessThan(1);
    const help = page.getByRole("button", { name: "Help", exact: true });
    await help.hover();
    await expect(help.locator(".control-tooltip")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(help.locator(".control-tooltip")).toBeHidden();
    await page.mouse.move(100, 200);
    await expect(help.locator(".control-tooltip")).toBeHidden();
    await help.focus();
    await page.keyboard.press("Tab");
    await page.keyboard.press("Shift+Tab");
    await expect(help).toBeFocused();
    await expect(help.locator(".control-tooltip")).toBeVisible();
    await navigation.getByRole("link", { name: "Training", exact: true }).click();
    await page.locator(".week-board .session-tile").first().click();
    await expect(page.locator(".desk-inspector")).toBeVisible();
    await page.setViewportSize({ width: 390, height: 844 });
    await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe("hidden");
    await page.emulateMedia({ reducedMotion: "reduce" });
    await expect.poll(async () => Math.round((await page.locator(".desk-inspector").boundingBox()).x)).toBe(0);
    await page.keyboard.press("Escape");
    await expect(page.locator(".desk-inspector")).toHaveCount(0);
    expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
  });

  test("buttons preserve geometry through pending, success, error and keyboard retry", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.addInitScript(() => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        if (command === "save_settings") await new Promise(resolve => { window.__releaseMotionAction = resolve; });
        return invoke(command, args);
      };
    });
    await page.goto("/desk.html#settings/maps");
    const save = page.getByRole("button", { name: "Save map source", exact: true });
    await expect(save).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    const resting = await save.boundingBox();
    await save.click();
    await expect(save).toHaveAttribute("aria-busy", "true");
    await expect(save).toBeDisabled();
    expect((await save.boundingBox()).width).toBeCloseTo(resting.width, 1);
    expect((await save.boundingBox()).height).toBeCloseTo(resting.height, 1);
    await page.evaluate(() => window.__releaseMotionAction());
    await expect(save).toHaveAttribute("data-result", "success");
    await expect(save).toBeEnabled();
    await expect(page.getByRole("status")).toContainText("Map source saved");
    await page.evaluate(() => { window.__deskFail = "save_settings"; });
    await page.getByLabel("Raster PMTiles archive").fill("/synthetic/map.pmtiles");
    await page.getByLabel("Raster PMTiles archive").press("Enter");
    await expect(save).toHaveAttribute("aria-busy", "true");
    await page.evaluate(() => window.__releaseMotionAction());
    await expect(save).toHaveAttribute("data-result", "error");
    await expect(page.getByRole("alert")).toContainText("Unable to save");
    await expect(page.getByLabel("Raster PMTiles archive")).toHaveValue("/synthetic/map.pmtiles");
    expect((await save.boundingBox()).width).toBeCloseTo(resting.width, 1);
    await page.evaluate(() => { window.__deskFail = ""; });
    await save.press("Enter");
    await page.evaluate(() => window.__releaseMotionAction());
    await expect(save).toHaveAttribute("data-result", "success");
    const notice = page.locator(".desk-notice");
    await page.getByRole("button", { name: "Dismiss notification" }).evaluate(button => button.click());
    await expect(notice).toBeAttached();
    await expect(notice).toHaveCount(0);
  });

  for (const width of [390, 1440]) {
    test(`overlays retain their exit, focus and scroll ownership at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.emulateMedia({ reducedMotion: "no-preference" });
      await page.goto("/desk.html#health/pace");
      const opener = page.getByRole("button", { name: "Log pain", exact: true });
      await opener.click();
      const dialog = page.getByRole("dialog", { name: "Log pain" });
      await expect(dialog).toBeVisible();
      await expect.poll(async () => Math.round((await dialog.boundingBox()).x)).toBe(width === 390 ? 0 : 420);
      await page.getByLabel("Notes", { exact: true }).fill("Retained while the surface exits");
      await page.getByRole("button", { name: "Cancel", exact: true }).evaluate(button => button.click());
      await expect(dialog).toHaveAttribute("data-state", "closing");
      await expect(dialog).toContainText("Save pain entry");
      expect(await page.evaluate(() => document.body.style.overflow)).toBe("hidden");
      await expect(dialog).toHaveCount(0);
      await expect(opener).toBeFocused();
      expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
      await opener.click();
      await page.keyboard.press("Escape");
      await expect(dialog).toHaveCount(0);
      await expect(opener).toBeFocused();
      expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
    });
  }

  test("touch surfaces track, cancel and dismiss without hijacking form controls", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    const session = await page.context().newCDPSession(page);
    await session.send("Emulation.setTouchEmulationEnabled", { enabled: true });
    const frame = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(resolve)));
    const touch = (type, x, y) => session.send("Input.dispatchTouchEvent", { type, touchPoints: type === "touchEnd" || type === "touchCancel" ? [] : [{ x, y }] });
    await page.goto("/desk.html#health/pace");
    const opener = page.getByRole("button", { name: "Log pain", exact: true });
    await opener.click();
    const dialog = page.locator(".desk-dialog");
    const grip = page.locator(".dialog-grip");
    await expect.poll(async () => Math.round((await dialog.boundingBox()).y + (await dialog.boundingBox()).height)).toBe(844);
    const rest = await dialog.boundingBox();
    const handle = await grip.boundingBox();
    const start = { x: handle.x + handle.width / 2, y: handle.y + handle.height / 2 };
    await touch("touchStart", start.x, start.y);
    await touch("touchMove", start.x, start.y + 12);
    await frame();
    await touch("touchMove", start.x, start.y + 48);
    await frame();
    expect((await dialog.boundingBox()).y - rest.y).toBeGreaterThan(20);
    await touch("touchCancel", start.x, start.y + 48);
    await expect(dialog).toHaveCount(1);
    await expect.poll(async () => Math.round((await dialog.boundingBox()).y)).toBe(Math.round(rest.y));
    await page.getByLabel("Notes", { exact: true }).fill("Touch input still works");
    await touch("touchStart", start.x, start.y);
    await touch("touchMove", start.x, start.y + 16);
    await frame();
    await touch("touchMove", start.x, start.y + 150);
    await frame();
    expect((await dialog.boundingBox()).y - rest.y).toBeGreaterThan(60);
    await touch("touchEnd", start.x, start.y + 150);
    await expect(dialog).toHaveCount(0);
    await expect(opener).toBeFocused();
    await page.goto("/desk.html#training");
    await page.locator(".mobile-date-ribbon button").first().click();
    const tile = page.locator(".day-itinerary .session-tile").first();
    await tile.click();
    const inspector = page.locator(".desk-inspector");
    await expect.poll(async () => Math.round((await inspector.boundingBox()).x)).toBe(0);
    const bar = await page.locator(".inspector-bar > span").boundingBox();
    const origin = { x: bar.x + bar.width / 2, y: bar.y + bar.height / 2 };
    await touch("touchStart", origin.x, origin.y);
    await touch("touchMove", origin.x + 16, origin.y);
    await frame();
    await touch("touchMove", origin.x + 140, origin.y);
    await frame();
    expect((await inspector.boundingBox()).x).toBeGreaterThan(40);
    await touch("touchEnd", origin.x + 140, origin.y);
    await expect(inspector).toHaveCount(0);
    await expect(tile).toBeFocused();
    expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
    await session.detach();
  });

  test("reduced motion keeps feedback and keyboard interaction without spatial animation", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/desk.html#health/pace");
    await page.getByRole("button", { name: "Log pain", exact: true }).click();
    const dialog = page.getByRole("dialog");
    const geometry = await dialog.boundingBox();
    expect(Math.round(geometry.x)).toBe(0);
    expect(Math.round(geometry.y + geometry.height)).toBe(844);
    expect(await dialog.evaluate(element => getComputedStyle(element).transform)).toBe("none");
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await page.getByRole("button", { name: "Long", exact: true }).focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("button", { name: "Long", exact: true })).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByRole("region", { name: "Selected pace session", exact: true })).toContainText("Race-pace long run");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  });
});

for (const [width, pointCount, span] of [[390, 2, .001], [1440, 1001, .5]]) {
  test(`activity route draws with ease-in-out in three seconds at ${width}px with ${pointCount} GPS points`, async ({page}) => {
    await page.setViewportSize({width, height:1000});
    await page.emulateMedia({reducedMotion:'no-preference'});
    const tile = await page.evaluate(() => {
      const canvas = document.createElement('canvas');
      canvas.width = canvas.height = 256;
      const context = canvas.getContext('2d');
      context.fillStyle = '#e5e7eb';
      context.fillRect(0, 0, 256, 256);
      return canvas.toDataURL().split(',')[1];
    });
    await page.route('https://tile.openstreetmap.org/**', route => route.fulfill({status:200, contentType:'image/png', body:Buffer.from(tile, 'base64')}));
    await page.route('**/src/desk/map.tsx*', async route => {
      const response = await route.fetch();
      await route.fulfill({response, body:`${await response.text()}
        const originalSetPaint = maplibregl.Map.prototype.setPaintProperty;
        maplibregl.Map.prototype.setPaintProperty = function(layer, property, value, ...options) {
          const result = originalSetPaint.call(this, layer, property, value, ...options);
          if (layer === 'route' && property === 'line-gradient') {
            window.__routeTestMap = this;
            (window.__routeFrames ??= []).push({time:performance.now(), progress:value[0] === 'step' ? value[3] : 1});
          }
          return result;
        };
      `});
    });
    await page.addInitScript(({pointCount, span}) => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        const result = await invoke(command, args);
        return command === 'activity_review' ? {...result, track:Array.from({length:pointCount}, (_, index) => ({lat:38.72, lon:-9.14 + span * index / (pointCount - 1)}))} : result;
      };
    }, {pointCount, span});
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.clock.install();
    await page.goto('/desk.html#journal/1');
    await expect.poll(() => page.evaluate(() => window.__routeFrames?.length ?? 0)).toBeGreaterThan(1);
    await page.clock.pauseAt(await page.evaluate(() => Date.now() + 100));
    const elapsed = await page.evaluate(() => performance.now() - window.__routeFrames[0].time);
    expect(elapsed).toBeLessThan(1500);
    await page.clock.runFor(1500 - elapsed);
    const progress = () => page.evaluate(() => window.__routeFrames?.at(-1)?.progress);
    expect(await progress()).toBeCloseTo(.5, 1);
    expect(await page.evaluate(() => window.__routeTestMap.getPaintProperty('route-outline','line-gradient')[3])).toBeCloseTo(.5, 1);
    const pixels = () => page.locator('.maplibregl-canvas').evaluate(canvas => {
      const context = canvas.getContext('webgl2');
      const bytes = new Uint8Array(context.drawingBufferWidth * context.drawingBufferHeight * 4);
      context.readPixels(0, 0, context.drawingBufferWidth, context.drawingBufferHeight, context.RGBA, context.UNSIGNED_BYTE, bytes);
      const counts = {start:0, end:0};
      for (let index = 0; index < bytes.length; index += 4) {
        if (bytes[index + 3] && bytes[index + 1] > bytes[index] + 30 && bytes[index + 1] > bytes[index + 2] + 15) {
          counts[(index / 4) % context.drawingBufferWidth < context.drawingBufferWidth / 2 ? 'start' : 'end']++;
        }
      }
      return counts;
    });
    await expect.poll(async () => (await pixels()).start).toBeGreaterThan(50);
    const halfway = await pixels();
    expect(halfway.end).toBeLessThan(halfway.start / 4);
    await page.screenshot({path:`test-results/desk-route-half-${width}.png`});
    await page.clock.runFor(1400);
    expect(await progress()).toBeLessThan(1);
    await page.clock.runFor(150);
    expect(await progress()).toBe(1);
    const frames = await page.evaluate(() => window.__routeFrames);
    expect(frames[0].progress).toBe(0);
    expect(frames.at(-1).time - frames[0].time).toBeGreaterThanOrEqual(3000);
    expect(frames.at(-1).time - frames[0].time).toBeLessThan(3050);
    expect(frames.find(frame => frame.time - frames[0].time >= 750).progress).toBeCloseTo(.15625, 1);
    expect(frames.find(frame => frame.time - frames[0].time >= 2250).progress).toBeCloseTo(.84375, 1);
    expect(frames.every((frame, index) => index === 0 || frame.progress >= frames[index - 1].progress)).toBe(true);
    await expect.poll(async () => (await pixels()).end).toBeGreaterThan(halfway.start / 2);
    await page.screenshot({path:`test-results/desk-route-complete-${width}.png`});
    await page.getByRole('button',{name:'Back',exact:true}).click();
    await page.clock.runFor(250);
    await expect(page.locator('.maplibregl-canvas')).toHaveCount(0);
    await page.evaluate(() => {window.__routeFrames = [];});
    await page.clock.resume();
    await page.locator('.activity-row').first().click();
    await expect.poll(() => page.evaluate(() => window.__routeFrames.length)).toBeGreaterThan(1);
    expect(await progress()).toBeLessThan(.5);
    await page.getByRole('button',{name:'Back',exact:true}).click();
    await expect(page.locator('.maplibregl-canvas')).toHaveCount(0);
    const stopped = await page.evaluate(() => window.__routeFrames.length);
    await page.clock.runFor(3100);
    expect(await page.evaluate(() => window.__routeFrames.length)).toBe(stopped);
    await page.emulateMedia({reducedMotion:'reduce'});
    await page.evaluate(() => {window.__routeFrames = [];});
    await page.locator('.activity-row').first().click();
    await expect.poll(progress).toBe(1);
    expect(await page.evaluate(() => window.__routeFrames.map(frame => frame.progress))).toEqual([1]);
    await expect(page.locator('.route-figure figcaption')).toHaveCount(0);
    expect(errors).toEqual([]);
  });
}

test('graded performance proposals preview race estimates and only change them after approval', async ({page}) => {
  await page.goto('/desk.html?pacingProposal=1#health/pace');
  await page.getByRole('button',{name:'Review changes',exact:true}).click();
  const preview = page.getByRole('region',{name:'Proposed race estimates'});
  await expect(preview).toBeVisible();
  const times = await preview.locator('tbody tr').evaluateAll(rows => rows.map(row => [...row.querySelectorAll('td')].map(cell => Number(cell.dataset.seconds))));
  expect(times).toHaveLength(4);
  expect(times.every(([before,after]) => after < before && after > before*.95)).toBe(true);
  expect(await page.evaluate(async () => {
    const {trainingRaceTime}=await import('/src/desk/race-estimates.tsx');
    return [5,10,21.0975,42.195].map(distance=>trainingRaceTime(285,0,distance));
  })).toEqual([1348,2810,6201,12928]);
  await expect(page.getByRole('region',{name:'Proposed session changes'})).toContainText('10.3 km');
  await expect(page.getByRole('region',{name:'Proposed session changes'})).toContainText('4:30 - 4:40/km');
  expect(await page.evaluate(()=>window.__deskCalls.filter(call=>call.command==='apply_proposal'))).toHaveLength(0);
  await page.evaluate(()=>{window.__deskFail='apply_proposal';});
  await page.getByRole('button',{name:'Apply changes',exact:true}).click();
  await expect(page.getByRole('dialog').getByRole('alert')).toBeVisible();
  await expect(preview).toBeVisible();
  await page.evaluate(()=>{window.__deskFail='';});
  await page.getByRole('button',{name:'Apply changes',exact:true}).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.getByRole('link',{name:'Overview',exact:true}).click();
  const estimates=page.getByRole('region',{name:'Training race estimates'});
  await expect(estimates).toBeVisible();
  expect(await estimates.locator('tbody td').evaluateAll(cells=>cells.map(cell=>Number(cell.dataset.seconds)))).toEqual(times.map(values=>values[1]));
});

for (const width of [320,1440]) {
  test(`percentage charts and tinted glass respect accessibility preferences at ${width}px`, async ({page}) => {
    await page.setViewportSize({width,height:1000});
    for (const theme of ['light','dark']) {
      await page.goto(`/desk.html?theme=${theme}&palette=amethyst#health`);
      await expect(page.locator('.trend-chart[data-percent]')).toHaveCount(5);
      await page.evaluate(()=>document.fonts.ready);
      expect(await page.evaluate(()=>document.fonts.check('16px "Desk Sans"') && document.fonts.check('24px "Desk Serif"'))).toBe(true);
      const charts=await page.locator('.trend-chart[data-percent]').evaluateAll(figures=>figures.map(figure=>({
        label:figure.querySelector('figcaption span').textContent,
        latest:figure.querySelector('figcaption strong').textContent,
        values:[...figure.querySelectorAll('circle')].map(circle=>Number(circle.getAttribute('cy'))),
        gradient:figure.querySelector('linearGradient')?.id,
        stop:figure.querySelector('stop:last-child')?.getAttribute('stop-opacity'),
        ticks:[...figure.querySelectorAll('svg text')].map(text=>text.textContent),
      })));
      expect(charts.every(chart=>chart.latest.endsWith('%') && chart.values.every(value=>value>=20 && value<=128))).toBe(true);
      expect(charts.every(chart=>chart.stop==='0' && chart.ticks.includes('100%') && chart.ticks.includes('0%'))).toBe(true);
      expect(new Set(charts.map(chart=>chart.gradient)).size).toBe(5);
      const load=charts.find(chart=>chart.label==='Training load (normalized)');
      const raw=await page.evaluate(async()=>window.__TAURI_INTERNALS__.invoke('dashboard',{range:'7d'}));
      const latest=raw.loadHistory.at(-1).ratio;
      expect(load.latest).toBe(`${Math.round(Math.min(100,Math.max(0,latest/raw.thresholds.ratioOptimizedHi*100)))}%`);
      const readiness=charts.find(chart=>chart.label==='Readiness');
      expect(readiness.latest).toBe(`${raw.scores.globalScore}%`);
      const amethyst=await page.locator('body').evaluate(element=>getComputedStyle(element).backgroundImage);
      await page.goto(`/desk.html?theme=${theme}&palette=ember#health`);
      await expect(page.locator('.trend-chart[data-percent]')).toHaveCount(5);
      expect(await page.locator('body').evaluate(element=>getComputedStyle(element).backgroundImage)).not.toBe(amethyst);
      const glass=await page.locator('.desk-toolbar').evaluate(element=>getComputedStyle(element).backdropFilter);
      expect(glass).toContain('blur');
      expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
      expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations).toEqual([]);
      await page.screenshot({path:`test-results/desk-glass-health-${theme}-${width}.png`,fullPage:true});
    }
    await page.emulateMedia({reducedMotion:'reduce',contrast:'more'});
    expect(await page.locator('.desk-toolbar').evaluate(element=>getComputedStyle(element).backdropFilter)).toBe('none');
    expect(await page.locator('.desk-button').first().evaluate(element=>getComputedStyle(element).transitionDuration)).toBe('0s');
  });
}

for (const width of [320, 1440]) {
  test(`pace insights compare normalized rep ranges and support inspection at ${width}px`, async ({page}) => {
    await page.setViewportSize({width,height:1000});
    for (const theme of ['light','dark']) {
      await page.goto(`/desk.html?theme=${theme}#health/pace`);
      await expect(page.getByRole('heading',{name:'Pace on point',exact:true})).toBeVisible();
      const points = page.locator('.pace-point-session');
      await expect(points).toHaveCount(5);
      await expect(page.locator('.pace-summary')).toHaveText('2/4 sessions within target');
      await expect(page.getByText('Next speed workout in 3 days', {exact:true})).toBeVisible();
      const geometry = await points.evaluateAll(groups => groups.map(group => ({
        top:Number(group.querySelector('.pace-point-range').getAttribute('y1')),
        bottom:Number(group.querySelector('.pace-point-range').getAttribute('y2')),
        circles:group.querySelectorAll('circle').length,
      })));
      expect(geometry[0].top).toBeLessThan(geometry[0].bottom);
      expect(geometry[2].circles).toBe(1);
      expect(geometry[2].top).toBeCloseTo(140, 3);
      const band = await page.locator('.pace-point-band').evaluate(element => ({top:Number(element.getAttribute('y')),height:Number(element.getAttribute('height'))}));
      expect(geometry[1].top).toBeGreaterThan(band.top);
      expect(geometry[1].bottom).toBeLessThan(band.top + band.height);
      expect(geometry[0].top).toBeLessThan(band.top);
      expect(geometry[0].bottom).toBeGreaterThan(band.top + band.height);
      expect(await page.locator('.pace-insights-figure svg').evaluate(svg => {
        const band = svg.querySelector('.pace-point-band');
        const left = Number(band.getAttribute('x'));
        const right = left + Number(band.getAttribute('width'));
        return [...svg.querySelectorAll('circle')].every(point => Number(point.getAttribute('cx')) >= left && Number(point.getAttribute('cx')) <= right);
      })).toBe(true);
      const picker = page.getByRole('group',{name:'Inspect pace session'});
      await picker.getByRole('button').nth(2).focus();
      await page.keyboard.press('Enter');
      await expect(page.getByRole('region',{name:'Selected pace session'})).toContainText('Continuous tempo');
      await expect(page.getByRole('region',{name:'Pace insight repetitions'}).locator('tbody tr')).toHaveCount(1);
      await picker.getByRole('button').first().click();
      await expect(page.getByRole('region',{name:'Selected pace session'})).toContainText('Outdated: excluded from current pace status');
      await page.getByRole('button',{name:'Long',exact:true}).click();
      await expect(points).toHaveCount(1);
      await expect(page.getByRole('region',{name:'Selected pace session'})).toContainText('Race-pace long run');
      await expect(page.getByText('Next long run in 5 days',{exact:true})).toBeVisible();
      await page.getByRole('button',{name:'Speed',exact:true}).click();
      await expect(points).toHaveCount(5);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations).toEqual([]);
      await page.locator('.pace-insights').screenshot({path:`test-results/desk-pace-insights-${theme}-${width}.png`});
    }
    await page.getByRole('button',{name:'Open activity review',exact:true}).click();
    await expect(page).toHaveURL(/#journal\/1$/);
    await expect(page.getByRole('heading',{name:'Session execution',exact:true})).toBeVisible();
  });
}

test('pace insights handle no plan, empty history and retry without changing the plan', async ({page}) => {
  await page.goto('/desk.html?noPlan=1#health/pace');
  await expect(page.getByText('Create a training plan and link completed runs to see pace insights.')).toBeVisible();
  await expect(page.locator('.pace-point-session')).toHaveCount(0);
  await page.addInitScript(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    let fail = true;
    window.__TAURI_INTERNALS__.invoke = async (command,args) => {
      if(command === 'pace_insights') {
        if(fail) { fail=false;throw new Error('Synthetic insight failure'); }
        return {speed:[],long:[],nextSpeed:null,nextLong:null};
      }
      return invoke(command,args);
    };
  });
  await page.goto('/desk.html#health/pace');
  await expect(page.getByText(/Synthetic insight failure/)).toBeVisible();
  await page.getByRole('button',{name:'Try again',exact:true}).click();
  await expect(page.getByText(/Completed tempo and interval sessions/)).toBeVisible();
  await page.getByRole('button',{name:'Long',exact:true}).click();
  await expect(page.getByText(/Conversational runs are not graded/)).toBeVisible();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => ['apply_proposal','save_settings','create_plan'].includes(call.command)))).toEqual([]);
});

for (const session of ['intervals', 'tempo']) {
  test(`${session} activity shows measured lap bars against prescribed pace ranges`, async ({page}) => {
    await page.setViewportSize({width:390, height:1000});
    if (session === 'tempo') await page.addInitScript(() => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        const result = await invoke(command, args);
        if (command !== 'activity_review') return result;
        return {...result, laps:[result.laps[0], {...result.laps[3], index:2, label:'Continuous tempo'}, {...result.laps.at(-1), index:3}]};
      };
    });
    await page.goto('/desk.html#journal/1');
    const chart = page.getByRole('region',{name:'Lap pace chart'});
    const buttons = chart.getByRole('button');
    await expect(buttons).toHaveCount(session === 'tempo' ? 3 : 9);
    await expect(chart.locator('.lap-target-band')).toHaveCount(session === 'tempo' ? 1 : 4);
    await expect(page.locator('.lap-pace-range')).toHaveCount(session === 'tempo' ? 1 : 4);
    await expect(chart.locator('.lap-target-band.on-target')).toHaveCount(session === 'tempo' ? 1 : 2);
    await expect(page.locator('.lap-pace-range.on-target')).toHaveCount(session === 'tempo' ? 1 : 2);
    const geometry = await buttons.evaluateAll(columns => columns.map(column => ({
      actual:parseFloat(column.querySelector('.lap-pace-actual').style.height),
      bottom:parseFloat(column.querySelector('.lap-target-band')?.style.bottom),
      band:parseFloat(column.querySelector('.lap-target-band')?.style.height),
    })));
    if (session === 'intervals') {
      expect(geometry[1].actual).toBeCloseTo(88, 3);
      expect(geometry[1].bottom).toBeCloseTo(88 * 268 / 285, 3);
      expect(geometry[1].bottom + geometry[1].band).toBeCloseTo(88 * 268 / 275, 3);
      expect(geometry[1].actual).toBeGreaterThan(geometry[1].bottom + geometry[1].band);
      expect(geometry[3].actual).toBeGreaterThan(geometry[3].bottom);
      expect(geometry[3].actual).toBeLessThan(geometry[3].bottom + geometry[3].band);
      expect(geometry[5].actual).toBeLessThan(geometry[5].bottom);
      for (const index of [0,2,4,6,8]) await expect(buttons.nth(index).locator('.lap-target-band')).toHaveCount(0);
      await buttons.nth(5).click();
      await expect(page.locator('.lap-pace-selection')).toContainText('Slower than target');
      await expect(page.locator('.lap-execution-table tr[data-selected="true"]')).toContainText('4:51/km');
    } else {
      expect(geometry[1].actual).toBeGreaterThan(geometry[1].bottom);
      expect(geometry[1].actual).toBeLessThan(geometry[1].bottom + geometry[1].band);
    }
    await buttons.nth(1).focus();
    await page.keyboard.press('Enter');
    await expect(buttons.nth(1)).toHaveAttribute('aria-pressed','true');
    await expect(page.locator('.lap-pace-selection')).toContainText('4:35 - 4:45/km');
    await expect(page.getByRole('region',{name:'Recorded laps'}).getByRole('columnheader',{name:'Time',exact:true})).toBeVisible();
    const rowGeometry = await page.locator('.lap-execution-table tbody tr').evaluateAll(rows => rows.map(row => ({
      actual:parseFloat(row.querySelector('.lap-pace-fill').style.width),
      start:parseFloat(row.querySelector('.lap-pace-range')?.style.left),
      range:parseFloat(row.querySelector('.lap-pace-range')?.style.width),
    })));
    for (const [index, column] of geometry.entries()) {
      expect(rowGeometry[index].actual).toBeCloseTo(column.actual, 3);
      if (!Number.isNaN(column.bottom)) {
        expect(rowGeometry[index].start).toBeCloseTo(column.bottom, 3);
        expect(rowGeometry[index].range).toBeCloseTo(column.band, 3);
      }
    }
    await page.getByRole('region',{name:'Recorded laps'}).getByRole('button').first().click();
    await expect(buttons.first()).toHaveAttribute('aria-pressed','true');
    await expect(page.locator('.lap-pace-selection')).toContainText('Warm-up');
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations).toEqual([]);
    await page.locator('.lap-execution').screenshot({path:`test-results/desk-${session}-pace-ranges.png`});
  });
}

test('lap ranges handle missing measurements and never invent unprescribed targets', async ({page}) => {
  await page.addInitScript(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, args) => {
      const result = await invoke(command, args);
      if (command !== 'activity_review') return result;
      return {...result, prescribed:!new URLSearchParams(location.search).has('unprescribed'), laps:result.laps.map((lap,index) => index === 1 ? {...lap,avgPaceSKm:null} : index === 3 ? {...lap,bandLoSKm:300,bandHiSKm:280} : lap)};
    };
  });
  await page.goto('/desk.html#journal/1');
  const missing = page.getByRole('region',{name:'Lap pace chart'}).getByRole('button').nth(1);
  await expect(missing.locator('.lap-target-band')).toHaveCount(1);
  await expect(missing.locator('.lap-pace-actual')).toHaveCount(0);
  await missing.click();
  await expect(page.locator('.lap-pace-selection')).toContainText('Not recorded');
  await expect(page.locator('.lap-pace-selection')).toContainText('Not graded');
  await expect(page.locator('.lap-pace-column').nth(3).locator('.lap-target-band')).toHaveCount(0);
  expect(await page.locator('.lap-execution').innerHTML()).not.toMatch(/NaN|Infinity/);
  await page.goto('/desk.html?unprescribed=1#journal/1');
  await expect(page.getByRole('heading',{name:'Recorded laps',exact:true})).toBeVisible();
  await expect(page.locator('.lap-target-band, .lap-pace-range, .lap-pace-chart')).toHaveCount(0);
  await expect(page.getByRole('region',{name:'Recorded laps'})).toContainText('Not prescribed');
});

for (const width of [320,1440]) {
  test(`review details remain accessible at ${width}px`, async ({page}) => {
    await page.setViewportSize({width,height:1000});
    for(const theme of ['light','dark']) {
      await page.goto(`/desk.html?theme=${theme}#journal/1`);
      await expect(page.getByRole('heading',{name:'Kilometre splits'})).toBeVisible();
      expect(await page.getByRole('region',{name:'Recorded laps'}).evaluate(region => region.scrollWidth <= region.clientWidth)).toBe(true);
      expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'activity review').toEqual([]);
      expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
      await page.screenshot({path:`test-results/desk-activity-${theme}-${width}.png`,fullPage:true});
      await page.goto(`/desk.html?proposal=1&theme=${theme}#health/journal`);
      await page.getByRole('button',{name:'Review changes',exact:true}).click();
      await expect(page.getByRole('dialog',{name:'Review plan changes'})).toBeVisible();
      expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'proposal review').toEqual([]);
      await page.screenshot({path:`test-results/desk-proposal-${theme}-${width}.png`,fullPage:true});
    }
  });
}

test('ending a plan requires confirmation and keeps recorded history',async ({page})=>{
  await page.goto('/plan.html');
  await page.getByRole('button',{name:'End plan',exact:true}).click();
  expect(await page.evaluate(()=>window.__deskCalls.filter(call=>call.command==='abort_plan'))).toEqual([]);
  await page.getByRole('button',{name:'Keep plan',exact:true}).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByRole('button',{name:'End plan',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'End plan',exact:true}).click();
  await page.getByRole('dialog').getByRole('button',{name:'End plan',exact:true}).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect(await page.evaluate(()=>window.__deskCalls.filter(call=>call.command==='abort_plan'))).toHaveLength(1);
  await page.getByRole('link',{name:'Journal',exact:true}).click();
  await expect(page.locator('.activity-row')).toHaveCount(9);
});

test("original entry routes load the independent workspace and retain deep links", async ({page}) => {
  const loaded=[];
  page.on('request',request=>loaded.push(request.url()));
  for(const [url,heading,section] of [
    ['/', 'Training', 'Week'],
    ['/index.html', 'Training', 'Week'],
    ['/plan.html', 'Training', 'Plan'],
    ['/calendar.html', 'Training', 'Month'],
    ['/achievements.html', 'Journal', 'Activities'],
    ['/achievements.html#performance', 'Health', 'Overview'],
    ['/achievements.html#recovery', 'Health', 'Overview'],
    ['/achievements.html#risk', 'Health', 'Overview'],
    ['/achievements.html#milestones', 'Health', 'Milestones'],
    ['/achievements.html#routes', 'Journal', 'Training map'],
    ['/injury.html', 'Health', 'Pain journal'],
    ['/settings.html', 'Settings', null],
    ['/support.html', 'Help', null],
  ]) {
    await page.goto(url);
    await expect(page.locator('#workspace h1')).toHaveText(heading);
    if(section) await expect(page.locator('.horizon-switch a[aria-current="page"]')).toHaveText(section);
    await expect(page.locator('#desk-root')).toBeVisible();
  }
  for(const [hash,heading] of [['theme','Appearance'],['zones','Training boundaries'],['tiles','Offline maps'],['publishing','Calendar and devices'],['setup','Watch connection']]) {
    await page.goto(`/settings.html#${hash}`);
    await expect(page.getByRole('heading',{name:heading,exact:true})).toBeVisible();
  }
  await page.goto('/index.html#journal/1');
  await expect(page.getByRole('complementary',{name:'Activity review'})).toBeVisible();
  expect(loaded.filter(url=>/\/(app|redesign)\.css|\/src\/(dashboard|plan|calendar|achievements|injury|settings|support)\.ts|\/lib\/(navigation|overlay|workout-detail)\.ts/.test(url))).toEqual([]);
});

for (const theme of ['light','dark']) {
  for (const width of [320,390,768,1440]) {
    test(`visual matrix ${theme} ${width}px`, async ({page}) => {
      test.setTimeout(90000);
      const errors=[];
      page.on('pageerror',error=>errors.push(error.message));
      await page.setViewportSize({width,height:1000});
      for (const route of ['training','training/month','training/plan','journal','journal/map','health','health/journal','health/milestones','settings','help']) {
        await page.goto(`/desk.html?theme=${theme}#${route}`);
        await expect(page.locator('#workspace h1')).toBeVisible();
        await expect(page.locator('.desk-loading')).toHaveCount(0);
        await expect(page.getByRole('alert')).toHaveCount(0);
        expect(await page.evaluate(() => document.documentElement.scrollWidth<=innerWidth),route).toBe(true);
        expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,route).toEqual([]);
        await page.screenshot({path:`test-results/desk-${route.replace('/','-')}-${theme}-${width}.png`,fullPage:true});
      }
      await page.goto(`/desk.html?theme=${theme}#training`);
      await page.locator(width>800 ? '.week-board .session-tile[draggable="true"]' : '.day-itinerary .session-tile[draggable="true"]').first().click();
      await expect(page.getByRole('complementary',{name:'Session',exact:true})).toBeVisible();
      if (width<=800) {
        for(let index=0;index<14;index++) {await page.keyboard.press('Tab');expect(await page.evaluate(()=>Boolean(document.activeElement?.closest('.desk-inspector')))).toBe(true);}
      }
      expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'session inspector').toEqual([]);
      await page.screenshot({path:`test-results/desk-session-${theme}-${width}.png`,fullPage:true});
      await page.keyboard.press('Escape');
      await expect(page.getByRole('complementary',{name:'Session',exact:true})).toHaveCount(0);
      await expect(page.getByRole('navigation',{name:'Workspaces'})).toBeVisible();
      expect(errors).toEqual([]);
    });
  }
}

test("native sync completion refreshes evidence and watch progress is visible", async ({page}) => {
  await page.goto('/desk.html');
  await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='plugin:event|listen').length)).toBe(3);
  await page.evaluate(() => window.__deskEmit('sync://started', null));
  await expect(page.getByRole('button',{name:'Syncing activities'})).toBeDisabled();
  const before = await page.evaluate(() => window.__deskCalls.filter(call=>call.command==='current_plan').length);
  await page.evaluate(() => window.__deskEmit('sync://completed',{ok:true}));
  await expect(page.getByRole('button',{name:'Sync activities'})).toBeEnabled();
  await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='current_plan').length)).toBeGreaterThan(before);
  await page.evaluate(() => {window.__deskHoldPush=true;});
  await page.getByRole('button',{name:/Send to watch|Update watch/}).click();
  await page.evaluate(() => window.__deskEmit('plan://sync',{op:'push',percent:45,stage:'Publishing sessions'}));
  await expect(page.getByRole('progressbar',{name:'Watch publishing progress'})).toHaveAttribute('value','45');
  await page.evaluate(() => window.__deskResolvePush());
  await expect(page.getByRole('status')).toContainText('Plan sent to COROS');
});

test("schedule mutations retain intent and Undo restores the original date", async ({page}) => {
  await page.setViewportSize({width:1440,height:1000});
  await page.goto('/desk.html');
  await page.getByRole('button',{name:'Next week',exact:true}).click();
  await page.locator('.week-board .session-tile[draggable="true"]').first().click();
  const original = await page.getByLabel('Run on',{exact:true}).inputValue();
  const next = new Date(`${original}T12:00:00`); next.setDate(next.getDate()+1);
  const moved = `${next.getFullYear()}-${String(next.getMonth()+1).padStart(2,'0')}-${String(next.getDate()).padStart(2,'0')}`;
  await page.getByLabel('Run on',{exact:true}).fill(moved);
  await page.getByRole('button',{name:'Move session',exact:true}).click();
  await expect(page.getByRole('status')).toContainText('Moved to');
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='move_workout').map(call=>call.args.date))).toEqual([moved,original]);
  await page.locator('.week-board .session-tile[draggable="true"]').first().click();
  await page.getByRole('button',{name:'Skip session',exact:true}).click();
  await expect(page.getByRole('status')).toContainText('Session skipped');
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='set_workout_skipped').map(call=>call.args.skipped))).toEqual([true,false]);
});

test("reset previews changes and matching uses the chosen recorded activity", async ({page}) => {
  await page.setViewportSize({width:1440,height:1000});
  await page.goto('/desk.html?unmatched=1');
  await page.getByRole('button',{name:'Next week',exact:true}).click();
  await page.locator('.week-board .session-tile[draggable="true"]').first().click();
  const original = await page.getByLabel('Run on',{exact:true}).inputValue();
  const next = new Date(`${original}T12:00:00`);next.setDate(next.getDate()+1);
  await page.getByLabel('Run on',{exact:true}).fill(`${next.getFullYear()}-${String(next.getMonth()+1).padStart(2,'0')}-${String(next.getDate()).padStart(2,'0')}`);
  await page.getByRole('button',{name:'Move session',exact:true}).click();
  await page.getByRole('button',{name:'Reset this week'}).click();
  await expect(page.getByRole('dialog')).toContainText('to');
  expect(await page.evaluate(() => window.__deskCalls.some(call=>call.command==='reset_week'))).toBe(false);
  await page.getByRole('button',{name:'Restore week',exact:true}).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.locator('.week-board .session-tile[draggable="true"]').first().click();
  await page.getByRole('combobox',{name:'Recorded activity'}).selectOption('99');
  await page.getByRole('button',{name:'Match run',exact:true}).click();
  await expect(page.getByRole('status')).toContainText('Recorded run matched');
  expect(await page.evaluate(() => window.__deskCalls.find(call=>call.command==='match_calendar_activity').args.activityId)).toBe(99);
});

for (const action of ['Apply','Dismiss']) {
  test(`${action} proposal requires review and preserves a failed decision`, async ({page}) => {
    await page.goto('/desk.html?proposal=1#health/journal');
    await page.getByRole('button',{name:'Review changes',exact:true}).click();
    await expect(page.getByRole('dialog')).toContainText('Easy run');
    expect(await page.evaluate(() => window.__deskCalls.some(call=>['apply_proposal','dismiss_proposal'].includes(call.command)))).toBe(false);
    if (action==='Dismiss') await page.getByRole('textbox',{name:'Reason for dismissing (optional)'}).fill('Prefer to review with my clinician.');
    await page.evaluate(command => {window.__deskFail=command;},`${action.toLowerCase()}_proposal`);
    await page.getByRole('button',{name:`${action} changes`,exact:true}).click();
    await expect(page.getByRole('dialog').getByRole('alert')).toContainText('Unable to save');
    await page.evaluate(() => {window.__deskFail='';});
    await page.getByRole('button',{name:`${action} changes`,exact:true}).click();
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(page.getByRole('button',{name:'Review changes',exact:true})).toHaveCount(0);
  });
}

for (const width of [390,1440]) {
  test(`training map renders geographic pixels and responds to zoom at ${width}px`, async ({page}) => {
    await page.setViewportSize({width,height:1000});
    await page.goto('/desk.html#journal/map');
    await expect(page.getByRole('region',{name:'Training route heatmap'})).toBeVisible();
    const pixels = () => page.locator('.maplibregl-canvas').evaluate(canvas => {
      const context=canvas.getContext('webgl2');
      const bytes=new Uint8Array(context.drawingBufferWidth*context.drawingBufferHeight*4);
      context.readPixels(0,0,context.drawingBufferWidth,context.drawingBufferHeight,context.RGBA,context.UNSIGNED_BYTE,bytes);
      let colored=0, fingerprint=0;
      for(let index=0;index<bytes.length;index+=64) { if(bytes[index+3]>0 && bytes[index+1]>bytes[index]+10 && bytes[index+1]>bytes[index+2]+5) colored++; fingerprint=(fingerprint+bytes[index]*(index+1))%1000000007; }
      return {colored,fingerprint};
    });
    await expect.poll(async () => (await pixels()).colored).toBeGreaterThan(10);
    const before=await pixels();
    await page.getByRole('button',{name:'Zoom in',exact:true}).click();
    await expect.poll(async () => (await pixels()).fingerprint).not.toBe(before.fingerprint);
    await page.getByRole('button',{name:'Fit training area',exact:true}).click();
    expect(await page.evaluate(() => document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    await page.screenshot({path:`test-results/desk-map-${width}.png`,fullPage:true});
  });
}

test("independent shell loads only new presentation", async ({ page }) => {
  const loaded = [];
  page.on("request", request => loaded.push(request.url()));
  await page.goto("/desk.html");
  await expect(page.getByRole("heading", { name: "Training", exact: true })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Workspaces" }).getByRole("link")).toHaveCount(3);
  await expect(page.locator(".day-itinerary")).toBeVisible();
  expect(loaded.filter(url => /\/(app|redesign)\.css|\/src\/(dashboard|plan|calendar|achievements|injury|settings|support)\.ts|\/lib\/(navigation|overlay|workout-detail)\.ts/.test(url))).toEqual([]);
});

test("mobile date selection opens a full-screen session and returns to its opener", async ({ page }) => {
  await page.setViewportSize({width:390, height:844});
  await page.emulateMedia({reducedMotion:"no-preference"});
  await page.goto("/desk.html#health");
  await page.getByRole("navigation", {name:"Workspaces"}).getByRole("link", {name:"Training"}).click();
  await page.locator(".mobile-date-ribbon button").first().click();
  const session = page.locator(".day-itinerary .session-tile").first();
  await session.click();
  await expect(page.getByRole("complementary", { name: "Session", exact: true })).toBeVisible();
  await expect.poll(async () => {
    const bounds = await page.locator(".desk-inspector").boundingBox();
    return bounds && Object.fromEntries(Object.entries(bounds).map(([key,value]) => [key,Math.round(value)]));
  }).toEqual({x:0,y:0,width:390,height:844});
  await expect(page.locator(".prescription")).toContainText("Repeat 4 times:");
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect(session).toBeFocused();
});

test("plan creation submits the established contract", async ({ page }) => {
  await page.goto("/desk.html?noPlan=1");
  await page.getByRole("button", { name: "Create plan", exact: true }).click();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByLabel("Threshold pace (min/km)")).toHaveValue("4:40");
  await page.getByLabel("Weekly distance (km)").fill("40");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await page.getByRole("button", { name: "Build plan", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  const request = await page.evaluate(() => window.__deskCalls.find(call => call.command === "create_plan").args.request);
  expect(request).toMatchObject({kind:"race",distance:"21K",thresholdSecKm:280,weeklyFrequency:4,currentWeeklyKm:40});
  await expect(page.locator(".plan-basis")).toContainText("40.0 km/week");
});

test("failed plan creation keeps the reviewed draft", async ({ page }) => {
  await page.goto("/desk.html?noPlan=1");
  await page.getByRole("button", { name: "Create plan", exact: true }).click();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await page.getByLabel("Weekly distance (km)").fill("37");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await page.evaluate(() => { window.__deskFail = "create_plan"; });
  await page.getByRole("button", { name: "Build plan", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Unable to save right now");
  await expect(page.locator(".composer-summary")).toContainText("37 km/week");
  await expect(page.getByRole("button", { name: "Build plan", exact: true })).toBeEnabled();
});

for (const width of [320, 390, 768, 1440]) {
  test(`training layout and accessibility at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.goto("/desk.html");
    await expect(page.locator(width > 800 ? ".week-board" : ".day-itinerary")).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
    await page.screenshot({path:`test-results/desk-training-${width}.png`,fullPage:true});
  });
}

test("Journal combines filters and opens recorded splits", async ({ page }) => {
  await page.goto("/desk.html#journal");
  await expect(page.locator(".activity-row")).toHaveCount(9);
  await page.getByRole("combobox", { name: "Sport", exact: true }).selectOption("Run");
  await expect(page.locator(".activity-row")).toHaveCount(6);
  await page.getByLabel("Search activities").fill("not-a-run");
  await expect(page.getByRole("heading", { name: "No matching activities" })).toBeVisible();
  await page.getByRole("button", { name: "Reset filters" }).click();
  await expect(page.locator(".activity-row")).toHaveCount(9);
  await page.locator(".activity-row").first().click();
  await expect(page.getByRole("complementary", { name: "Activity review" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Kilometre splits" })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "vs previous km" })).toBeVisible();
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect(page.locator(".activity-row").first()).toBeFocused();
});

test("pain logging retains failure drafts and episode conversations survive refresh", async ({ page }) => {
  await page.goto("/desk.html#health/journal");
  await page.getByRole("button", { name: "Log pain", exact: true }).first().click();
  await page.getByRole("combobox", { name: "Body region", exact: true }).selectOption("knee.left");
  await page.getByRole("textbox", { name: "Notes", exact: true }).fill("Synthetic test: soreness after running");
  await page.evaluate(() => { window.__deskFail = "save_pain_entry"; });
  await page.getByRole("button", { name: "Save pain entry" }).click();
  await expect(page.getByRole("alert")).toContainText("Unable to save right now");
  await expect(page.getByRole("textbox", { name: "Notes", exact: true })).toHaveValue("Synthetic test: soreness after running");
  await page.evaluate(() => { window.__deskFail = ""; });
  await page.getByRole("button", { name: "Save pain entry" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator(".pain-episode")).toContainText("Left knee");
  await page.getByRole("button", { name: "Discuss episode" }).click();
  await page.getByRole("textbox", { name: "Your question" }).fill("Should I take a rest day?");
  await expect(page.getByRole("button", { name: "Send question" })).toBeDisabled();
  await page.getByRole("checkbox", { name: /I agree to send/ }).check();
  await page.getByRole("button", { name: "Send question" }).click();
  await expect(page.locator(".chat-message.assistant")).toContainText("reducing the session");
  await expect(page.getByRole("textbox", { name: "Your question" })).toHaveValue("");
  await page.getByRole("button", { name: "Mark resolved" }).click();
  await expect(page.locator(".pain-episode")).toHaveCount(0);
  await page.getByRole("checkbox", { name: "Include resolved" }).check();
  await page.getByRole("button", { name: "Reopen", exact: true }).click();
  await expect(page.locator(".pain-episode")).toContainText("Active");
});

for (const route of ["journal", "health", "health/milestones"]) {
  test(`${route} loads backend evidence and meets accessibility checks`, async ({ page }) => {
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(`/desk.html#${route}`);
    await expect(page.locator(".desk-loading")).toHaveCount(0);
    await expect(page.locator(".error-notice")).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
    expect(errors).toEqual([]);
  });
}

test("settings directory searches tasks and preserves unrelated settings", async ({ page }) => {
  await page.goto("/desk.html#settings");
  await page.getByRole("searchbox", { name: "Search settings" }).fill("colour");
  await expect(page.locator(".settings-task-list a")).toHaveCount(1);
  await page.getByRole("link", { name: /Appearance/ }).click();
  await page.getByRole("radio", { name: "Dark", exact: true }).check();
  await expect(page.locator("html")).toHaveAttribute("data-desk-theme", "dark");
  const settings = await page.evaluate(() => window.__deskCalls.find(call => call.command === "save_settings").args.settings);
  expect(settings.llmEndpoint).toBe("https://example.test/v1");
  expect(settings.thresholds.buildWeeks).toBe(3);
  await page.getByRole("radio", { name: "Glacier", exact: true }).check();
  await expect(page.locator("html")).toHaveAttribute("data-desk-palette", "glacier");
});

test("calendar settings never publish on read and retain blank credentials and sync metadata", async ({ page }) => {
  await page.goto("/desk.html#settings/calendar");
  await expect(page.getByRole("button", { name: "Save calendar settings" })).toBeVisible();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "publish_calendar"))).toEqual([]);
  await page.getByRole("checkbox", { name: "Publish calendar remotely" }).check();
  await page.getByRole("button", { name: "Save calendar settings" }).click();
  await expect(page.getByRole("status").last()).toContainText("Calendar settings saved");
  const payload = await page.evaluate(() => window.__deskCalls.find(call => call.command === "save_calendar_publishing").args);
  expect(payload.password).toBeNull();
  expect(payload.config.lastPublished).toBe("2026-10-29T12:00:00Z");
  await page.getByRole("checkbox", { name: "Sync this plan across devices" }).uncheck();
  const sync = await page.evaluate(() => window.__deskCalls.find(call => call.command === "plan_sync_configure").args);
  expect(sync).toEqual({config:{enabled:false},token:null});
  await expect(page.getByText("synthetic-account-channel", {exact:true})).toBeVisible();
  await expect(page.getByRole("button", { name: "Import plan", exact:true })).toBeDisabled();
});

test("plan import only overwrites after explicit confirmation", async ({ page }) => {
  await page.goto("/desk.html?syncConflict=1#settings/calendar");
  await page.getByRole("button", { name: "Import plan", exact: true }).click();
  await expect(page.getByRole("dialog", {name:"Replace the plan on this device?"})).toBeVisible();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_pull").map(call => call.args.force))).toEqual([false]);
  await page.getByRole("button", { name:"Keep local plan" }).click();
  await page.getByRole("button", { name:"Import plan", exact:true }).click();
  await page.getByRole("button", { name:"Replace local plan", exact:true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.getByRole("status").last()).toContainText("Imported 33 sessions");
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_pull").map(call => call.args.force))).toEqual([false,false,true]);
});

test("training boundaries keep fractional units and re-read saved values", async ({ page }) => {
  await page.goto("/desk.html#settings/thresholds");
  await page.getByRole("spinbutton", { name:"Recovery week load (%)" }).fill("65");
  await page.getByRole("button", { name:"Save boundaries" }).click();
  await expect(page.getByRole("status")).toContainText("Training boundaries saved");
  const settings = await page.evaluate(() => window.__deskCalls.find(call => call.command === "save_settings").args.settings);
  expect(settings.thresholds.downWeekFactor).toBe(.65);
  expect(settings.llmModel).toBe("test-model");
  await expect(page.getByRole("spinbutton", { name:"Recovery week load (%)" })).toHaveValue("65");
});

test("removing an assistant clears its key without deleting pain data", async ({ page }) => {
  await page.goto("/desk.html#settings/assistant");
  await page.getByRole("button", { name:"Remove assistant", exact:true }).click();
  await page.getByRole("dialog").getByRole("button", {name:"Remove assistant",exact:true}).click();
  await expect(page.getByRole("button", {name:"Add assistant",exact:true})).toBeVisible();
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "clear_llm_api_key"))).toBe(true);
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "delete_pain_entry" || call.command === "clear_pain_messages"))).toBe(false);
  await expect(page.getByRole("button", {name:"Add assistant",exact:true})).toBeDisabled();
});

for (const width of [320, 1440]) {
  test(`Settings and Help are accessible at ${width}px`, async ({ page }) => {
    await page.setViewportSize({width,height:900});
    for (const section of ["", "/watch", "/appearance", "/thresholds", "/calendar", "/assistant", "/maps", "/about"]) {
      await page.goto(`/desk.html#settings${section}`);
      await expect(page.locator(".desk-loading")).toHaveCount(0);
      await expect(page.getByRole("heading", {name:"Settings",exact:true})).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), section).toBe(true);
      expect((await new AxeBuilder({page}).withTags(["wcag2a","wcag2aa","wcag21aa"]).analyze()).violations, section).toEqual([]);
    }
    await page.goto("/desk.html#help");
    await page.getByRole("searchbox", {name:"Search help"}).fill("missing");
    await expect(page.locator(".help-articles details")).toHaveCount(1);
    await page.getByText("A recent workout is missing", {exact:true}).click();
    await page.getByRole("link", {name:"Check watch connection"}).click();
    await expect(page.getByRole("heading", {name:"Watch connection",exact:true})).toBeVisible();
  });
}