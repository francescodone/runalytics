# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> graded performance proposals preview race estimates and only change them after approval
- Location: tests/desk.spec.mjs:743:1

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html?pacingProposal=1#health/pace
Call log:
  - navigating to "http://127.0.0.1:1500/desk.html?pacingProposal=1#health/pace", waiting until "load"

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
> 744 |   await page.goto('/desk.html?pacingProposal=1#health/pace');
      |              ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html?pacingProposal=1#health/pace
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
  777 |       await expect(page.locator('.trend-chart[data-percent]')).toHaveCount(5);
  778 |       await page.evaluate(()=>document.fonts.ready);
  779 |       expect(await page.evaluate(()=>[...document.fonts].some(font=>font.family==='Outfit' && font.status==='loaded'))).toBe(true);
  780 |       const charts=await page.locator('.trend-chart[data-percent]').evaluateAll(figures=>figures.map(figure=>({
  781 |         label:figure.querySelector('figcaption span').textContent,
  782 |         latest:figure.querySelector('figcaption strong').textContent,
  783 |         values:[...figure.querySelectorAll('circle')].map(circle=>Number(circle.getAttribute('cy'))),
  784 |         gradient:figure.querySelector('linearGradient')?.id,
  785 |         stop:figure.querySelector('stop:last-child')?.getAttribute('stop-opacity'),
  786 |         ticks:[...figure.querySelectorAll('svg text')].map(text=>text.textContent),
  787 |       })));
  788 |       expect(charts.every(chart=>chart.latest.endsWith('%') && chart.values.every(value=>value>=20 && value<=128))).toBe(true);
  789 |       expect(charts.every(chart=>chart.stop==='0' && chart.ticks.includes('100%') && chart.ticks.includes('0%'))).toBe(true);
  790 |       expect(new Set(charts.map(chart=>chart.gradient)).size).toBe(5);
  791 |       const load=charts.find(chart=>chart.label==='Training load (normalized)');
  792 |       const raw=await page.evaluate(async()=>window.__TAURI_INTERNALS__.invoke('dashboard',{range:'7d'}));
  793 |       const latest=raw.loadHistory.at(-1).ratio;
  794 |       expect(load.latest).toBe(`${Math.round(Math.min(100,Math.max(0,latest/raw.thresholds.ratioOptimizedHi*100)))}%`);
  795 |       const readiness=charts.find(chart=>chart.label==='Readiness');
  796 |       expect(readiness.latest).toBe(`${raw.scores.globalScore}%`);
  797 |       const amethyst=await page.locator('body').evaluate(element=>getComputedStyle(element).backgroundImage);
  798 |       await page.goto(`/desk.html?theme=${theme}&palette=ember#health`);
  799 |       await expect(page.locator('.trend-chart[data-percent]')).toHaveCount(5);
  800 |       expect(await page.locator('body').evaluate(element=>getComputedStyle(element).backgroundImage)).not.toBe(amethyst);
  801 |       const glass=await page.locator('.desk-toolbar').evaluate(element=>getComputedStyle(element).backdropFilter);
  802 |       expect(glass).toContain('blur');
  803 |       expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  804 |       expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations).toEqual([]);
  805 |       await page.screenshot({path:`test-results/desk-glass-health-${theme}-${width}.png`,fullPage:true});
  806 |     }
  807 |     await page.emulateMedia({reducedMotion:'reduce',contrast:'more'});
  808 |     expect(await page.locator('.desk-toolbar').evaluate(element=>getComputedStyle(element).backdropFilter)).toBe('none');
  809 |     expect(await page.locator('.desk-button').first().evaluate(element=>getComputedStyle(element).transitionDuration)).toBe('0s');
  810 |   });
  811 | }
  812 | 
  813 | for (const width of [320, 1440]) {
  814 |   test(`pace insights compare normalized rep ranges and support inspection at ${width}px`, async ({page}) => {
  815 |     await page.setViewportSize({width,height:1000});
  816 |     for (const theme of ['light','dark']) {
  817 |       await page.goto(`/desk.html?theme=${theme}#health/pace`);
  818 |       await expect(page.getByRole('heading',{name:'Pace on point',exact:true})).toBeVisible();
  819 |       const points = page.locator('.pace-point-session');
  820 |       await expect(points).toHaveCount(5);
  821 |       await expect(page.locator('.pace-summary')).toHaveText('2/4 sessions within target');
  822 |       await expect(page.getByText('Next speed workout in 3 days', {exact:true})).toBeVisible();
  823 |       const geometry = await points.evaluateAll(groups => groups.map(group => ({
  824 |         top:Number(group.querySelector('.pace-point-range').getAttribute('y1')),
  825 |         bottom:Number(group.querySelector('.pace-point-range').getAttribute('y2')),
  826 |         circles:group.querySelectorAll('circle').length,
  827 |       })));
  828 |       expect(geometry[0].top).toBeLessThan(geometry[0].bottom);
  829 |       expect(geometry[2].circles).toBe(1);
  830 |       expect(geometry[2].top).toBeCloseTo(140, 3);
  831 |       const band = await page.locator('.pace-point-band').evaluate(element => ({top:Number(element.getAttribute('y')),height:Number(element.getAttribute('height'))}));
  832 |       expect(geometry[1].top).toBeGreaterThan(band.top);
  833 |       expect(geometry[1].bottom).toBeLessThan(band.top + band.height);
  834 |       expect(geometry[0].top).toBeLessThan(band.top);
  835 |       expect(geometry[0].bottom).toBeGreaterThan(band.top + band.height);
  836 |       expect(await page.locator('.pace-insights-figure svg').evaluate(svg => {
  837 |         const band = svg.querySelector('.pace-point-band');
  838 |         const left = Number(band.getAttribute('x'));
  839 |         const right = left + Number(band.getAttribute('width'));
  840 |         return [...svg.querySelectorAll('circle')].every(point => Number(point.getAttribute('cx')) >= left && Number(point.getAttribute('cx')) <= right);
  841 |       })).toBe(true);
  842 |       const picker = page.getByRole('group',{name:'Inspect pace session'});
  843 |       await picker.getByRole('button').nth(2).focus();
  844 |       await page.keyboard.press('Enter');
```