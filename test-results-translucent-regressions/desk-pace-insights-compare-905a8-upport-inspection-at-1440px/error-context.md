# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> pace insights compare normalized rep ranges and support inspection at 1440px
- Location: tests/desk.spec.mjs:814:3

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html?theme=light#health/pace
Call log:
  - navigating to "http://127.0.0.1:1500/desk.html?theme=light#health/pace", waiting until "load"

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
> 817 |       await page.goto(`/desk.html?theme=${theme}#health/pace`);
      |                  ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html?theme=light#health/pace
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
  845 |       await expect(page.getByRole('region',{name:'Selected pace session'})).toContainText('Continuous tempo');
  846 |       await expect(page.getByRole('region',{name:'Pace insight repetitions'}).locator('tbody tr')).toHaveCount(1);
  847 |       await picker.getByRole('button').first().click();
  848 |       await expect(page.getByRole('region',{name:'Selected pace session'})).toContainText('Outdated: excluded from current pace status');
  849 |       await page.getByRole('button',{name:'Long',exact:true}).click();
  850 |       await expect(points).toHaveCount(1);
  851 |       await expect(page.getByRole('region',{name:'Selected pace session'})).toContainText('Race-pace long run');
  852 |       await expect(page.getByText('Next long run in 5 days',{exact:true})).toBeVisible();
  853 |       await page.getByRole('button',{name:'Speed',exact:true}).click();
  854 |       await expect(points).toHaveCount(5);
  855 |       expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  856 |       expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations).toEqual([]);
  857 |       await page.locator('.pace-insights').screenshot({path:`test-results/desk-pace-insights-${theme}-${width}.png`});
  858 |     }
  859 |     await page.getByRole('button',{name:'Open activity review',exact:true}).click();
  860 |     await expect(page).toHaveURL(/#journal\/1$/);
  861 |     // The review opens on Overview; the execution chart is one tab away.
  862 |     await expect(page.getByRole('heading',{name:'Session execution',exact:true})).toHaveCount(0);
  863 |     await page.getByRole('tab',{name:'Session execution',exact:true}).click();
  864 |     await expect(page).toHaveURL(/#journal\/1\/execution$/);
  865 |     await expect(page.getByRole('heading',{name:'Session execution',exact:true})).toBeVisible();
  866 |   });
  867 | }
  868 | 
  869 | test('pace insights handle no plan, empty history and retry without changing the plan', async ({page}) => {
  870 |   await page.goto('/desk.html?noPlan=1#health/pace');
  871 |   await expect(page.getByText('Create a training plan and link completed runs to see pace insights.')).toBeVisible();
  872 |   await expect(page.locator('.pace-point-session')).toHaveCount(0);
  873 |   await page.addInitScript(() => {
  874 |     const invoke = window.__TAURI_INTERNALS__.invoke;
  875 |     let fail = true;
  876 |     window.__TAURI_INTERNALS__.invoke = async (command,args) => {
  877 |       if(command === 'pace_insights') {
  878 |         if(fail) { fail=false;throw new Error('Synthetic insight failure'); }
  879 |         return {speed:[],long:[],nextSpeed:null,nextLong:null};
  880 |       }
  881 |       return invoke(command,args);
  882 |     };
  883 |   });
  884 |   await page.goto('/desk.html#health/pace');
  885 |   await expect(page.getByText(/Synthetic insight failure/)).toBeVisible();
  886 |   await page.getByRole('button',{name:'Try again',exact:true}).click();
  887 |   await expect(page.getByText(/Completed tempo and interval sessions/)).toBeVisible();
  888 |   await page.getByRole('button',{name:'Long',exact:true}).click();
  889 |   await expect(page.getByText(/Conversational runs are not graded/)).toBeVisible();
  890 |   expect(await page.evaluate(() => window.__deskCalls.filter(call => ['apply_proposal','save_settings','create_plan'].includes(call.command)))).toEqual([]);
  891 | });
  892 | 
  893 | for (const session of ['intervals', 'tempo']) {
  894 |   test(`${session} activity shows measured lap bars against prescribed pace ranges`, async ({page}) => {
  895 |     await page.setViewportSize({width:390, height:1000});
  896 |     if (session === 'tempo') await page.addInitScript(() => {
  897 |       const invoke = window.__TAURI_INTERNALS__.invoke;
  898 |       window.__TAURI_INTERNALS__.invoke = async (command, args) => {
  899 |         const result = await invoke(command, args);
  900 |         if (command !== 'activity_review') return result;
  901 |         return {...result, laps:[result.laps[0], {...result.laps[3], index:2, label:'Continuous tempo'}, {...result.laps.at(-1), index:3}]};
  902 |       };
  903 |     });
  904 |     await page.goto('/desk.html#journal/1/execution');
  905 |     const chart = page.getByRole('region',{name:'Lap pace chart'});
  906 |     const buttons = chart.getByRole('button');
  907 |     await expect(buttons).toHaveCount(session === 'tempo' ? 3 : 9);
  908 |     await expect(chart.locator('.lap-target-band')).toHaveCount(session === 'tempo' ? 1 : 4);
  909 |     await expect(chart.locator('.lap-target-band.on-target')).toHaveCount(session === 'tempo' ? 1 : 2);
  910 |     // The table states the same thing in words: one row per lap, with the
  911 |     // graded laps' target cells highlighted where the chart does.
  912 |     await expect(page.locator('.lap-execution-table tbody tr')).toHaveCount(session === 'tempo' ? 3 : 9);
  913 |     await expect(page.locator('.lap-execution-table .lap-row-target.on-target')).toHaveCount(session === 'tempo' ? 1 : 2);
  914 |     const geometry = await buttons.evaluateAll(columns => columns.map(column => ({
  915 |       actual:parseFloat(column.querySelector('.lap-pace-actual').style.height),
  916 |       bottom:parseFloat(column.querySelector('.lap-target-band')?.style.bottom),
  917 |       band:parseFloat(column.querySelector('.lap-target-band')?.style.height),
```