# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> review details remain accessible at 1440px
- Location: tests/desk.spec.mjs:1003:3

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html?theme=light#journal/1/splits
Call log:
  - navigating to "http://127.0.0.1:1500/desk.html?theme=light#journal/1/splits", waiting until "load"

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
  906  |     const buttons = chart.getByRole('button');
  907  |     await expect(buttons).toHaveCount(session === 'tempo' ? 3 : 9);
  908  |     await expect(chart.locator('.lap-target-band')).toHaveCount(session === 'tempo' ? 1 : 4);
  909  |     await expect(chart.locator('.lap-target-band.on-target')).toHaveCount(session === 'tempo' ? 1 : 2);
  910  |     // The table states the same thing in words: one row per lap, with the
  911  |     // graded laps' target cells highlighted where the chart does.
  912  |     await expect(page.locator('.lap-execution-table tbody tr')).toHaveCount(session === 'tempo' ? 3 : 9);
  913  |     await expect(page.locator('.lap-execution-table .lap-row-target.on-target')).toHaveCount(session === 'tempo' ? 1 : 2);
  914  |     const geometry = await buttons.evaluateAll(columns => columns.map(column => ({
  915  |       actual:parseFloat(column.querySelector('.lap-pace-actual').style.height),
  916  |       bottom:parseFloat(column.querySelector('.lap-target-band')?.style.bottom),
  917  |       band:parseFloat(column.querySelector('.lap-target-band')?.style.height),
  918  |       bandPx:column.querySelector('.lap-target-band')?.getBoundingClientRect().height ?? 0,
  919  |       width:column.getBoundingClientRect().width,
  920  |     })));
  921  |     if (session === 'intervals') {
  922  |       // The axis is linear over every pace the chart draws (laps and band edges),
  923  |       // padded 15% of the span either side, so a target range is a readable band
  924  |       // rather than a hairline. Fastest 268 s/km, slowest 402 s/km, so the axis
  925  |       // runs 247.9 to 422.1 over a 174.2 s span.
  926  |       const pos = pace => (422.1 - pace) / 174.2 * 100;
  927  |       expect(geometry[1].actual).toBeCloseTo(pos(268), 3);
  928  |       expect(geometry[1].bottom).toBeCloseTo(pos(285), 3);
  929  |       expect(geometry[1].bottom + geometry[1].band).toBeCloseTo(pos(275), 3);
  930  |       // Each column is as wide as its lap is long: the 24% warm-up is roughly
  931  |       // twice the 12% rep and ten times a 2.5% recovery.
  932  |       expect(geometry[0].width).toBeGreaterThan(geometry[1].width * 1.7);
  933  |       expect(geometry[1].width).toBeGreaterThan(geometry[2].width * 3);
  934  |       expect(geometry[2].width).toBeGreaterThanOrEqual(20);
  935  |       expect(geometry[1].actual).toBeGreaterThan(geometry[1].bottom + geometry[1].band);
  936  |       // A 10 s/km target range has to be something the eye can land on, not the
  937  |       // 5 px line the old reciprocal scale drew.
  938  |       expect(geometry[1].bandPx).toBeGreaterThanOrEqual(12);
  939  |       expect(geometry[3].actual).toBeGreaterThan(geometry[3].bottom);
  940  |       expect(geometry[3].actual).toBeLessThan(geometry[3].bottom + geometry[3].band);
  941  |       expect(geometry[5].actual).toBeLessThan(geometry[5].bottom);
  942  |       for (const index of [0,2,4,6,8]) await expect(buttons.nth(index).locator('.lap-target-band')).toHaveCount(0);
  943  |       await buttons.nth(5).click();
  944  |       await expect(page.locator('.lap-pace-selection')).toContainText('Slower than target');
  945  |       await expect(page.locator('.lap-execution-table tr[data-selected="true"]')).toContainText('4:51/km');
  946  |     } else {
  947  |       expect(geometry[1].actual).toBeGreaterThan(geometry[1].bottom);
  948  |       expect(geometry[1].actual).toBeLessThan(geometry[1].bottom + geometry[1].band);
  949  |     }
  950  |     await buttons.nth(1).focus();
  951  |     await page.keyboard.press('Enter');
  952  |     await expect(buttons.nth(1)).toHaveAttribute('aria-pressed','true');
  953  |     await expect(page.locator('.lap-pace-selection')).toContainText('4:35 - 4:45/km');
  954  |     await expect(page.getByRole('region',{name:'Recorded laps'}).getByRole('columnheader',{name:'Time',exact:true})).toBeVisible();
  955  |     await expect(page.getByRole('region',{name:'Recorded laps'}).getByRole('columnheader',{name:'Pace',exact:true})).toBeVisible();
  956  |     await expect(page.getByRole('region',{name:'Recorded laps'}).getByRole('columnheader',{name:'Target pace',exact:true})).toBeVisible();
  957  |     const rows = await page.locator('.lap-execution-table tbody tr').evaluateAll(tableRows => tableRows.map(row => row.innerText.replace(/\s+/g, ' ').trim()));
  958  |     const paces = await buttons.evaluateAll(columns => columns.map(column => column.querySelector('.lap-pace-actual > span')?.textContent ?? 'Not recorded'));
  959  |     for (const [index, column] of geometry.entries()) {
  960  |       const row = rows[index];
  961  |       expect(row).toContain(paces[index]);
  962  |       if (!Number.isNaN(column.bottom)) {
  963  |         expect(row).toContain('4:35 - 4:45/km');
  964  |       } else {
  965  |         expect(row).toContain('Not prescribed');
  966  |       }
  967  |     }
  968  |     await page.getByRole('region',{name:'Recorded laps'}).getByRole('button').first().click();
  969  |     await expect(buttons.first()).toHaveAttribute('aria-pressed','true');
  970  |     await expect(page.locator('.lap-pace-selection')).toContainText('Warm-up');
  971  |     expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  972  |     expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations).toEqual([]);
  973  |     await page.locator('.lap-execution').screenshot({path:`test-results/desk-${session}-pace-ranges.png`});
  974  |   });
  975  | }
  976  | 
  977  | test('lap ranges handle missing measurements and never invent unprescribed targets', async ({page}) => {
  978  |   await page.addInitScript(() => {
  979  |     const invoke = window.__TAURI_INTERNALS__.invoke;
  980  |     window.__TAURI_INTERNALS__.invoke = async (command, args) => {
  981  |       const result = await invoke(command, args);
  982  |       if (command !== 'activity_review') return result;
  983  |       return {...result, prescribed:!new URLSearchParams(location.search).has('unprescribed'), laps:result.laps.map((lap,index) => index === 1 ? {...lap,avgPaceSKm:null} : index === 3 ? {...lap,bandLoSKm:300,bandHiSKm:280} : lap)};
  984  |     };
  985  |   });
  986  |   await page.goto('/desk.html#journal/1/execution');
  987  |   const missing = page.getByRole('region',{name:'Lap pace chart'}).getByRole('button').nth(1);
  988  |   await expect(missing.locator('.lap-target-band')).toHaveCount(1);
  989  |   await expect(missing.locator('.lap-pace-actual')).toHaveCount(0);
  990  |   await missing.click();
  991  |   await expect(page.locator('.lap-pace-selection')).toContainText('Not recorded');
  992  |   await expect(page.locator('.lap-pace-selection')).toContainText('Not graded');
  993  |   await expect(page.locator('.lap-pace-column').nth(3).locator('.lap-target-band')).toHaveCount(0);
  994  |   expect(await page.locator('.lap-execution').innerHTML()).not.toMatch(/NaN|Infinity/);
  995  |   await page.goto('/desk.html?unprescribed=1#journal/1/execution');
  996  |   await expect(page.getByRole('heading',{name:'Recorded laps',exact:true})).toBeVisible();
  997  |   await expect(page.locator('.lap-target-band, .lap-pace-chart')).toHaveCount(0);
  998  |   await expect(page.locator('.lap-execution-table .lap-row-target.on-target')).toHaveCount(0);
  999  |   await expect(page.getByRole('region',{name:'Recorded laps'})).toContainText('Not prescribed');
  1000 | });
  1001 | 
  1002 | for (const width of [320,1440]) {
  1003 |   test(`review details remain accessible at ${width}px`, async ({page}) => {
  1004 |     await page.setViewportSize({width,height:1000});
  1005 |     for(const theme of ['light','dark']) {
> 1006 |       await page.goto(`/desk.html?theme=${theme}#journal/1/splits`);
       |                  ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html?theme=light#journal/1/splits
  1007 |       await expect(page.getByRole('heading',{name:'Kilometre splits'})).toBeVisible();
  1008 |       expect(await page.getByRole('region',{name:'Kilometre splits'}).evaluate(region => region.scrollWidth <= region.clientWidth)).toBe(true);
  1009 |       await page.getByRole('tab',{name:'Session execution',exact:true}).click();
  1010 |       await expect(page.getByRole('heading',{name:'Session execution',exact:true})).toBeVisible();
  1011 |       expect(await page.getByRole('region',{name:'Recorded laps'}).evaluate(region => region.scrollWidth <= region.clientWidth)).toBe(true);
  1012 |       expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'activity review').toEqual([]);
  1013 |       expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  1014 |       await page.screenshot({path:`test-results/desk-activity-${theme}-${width}.png`,fullPage:true});
  1015 |       await page.goto(`/desk.html?proposal=1&theme=${theme}#health/journal`);
  1016 |       await page.getByRole('button',{name:'Review changes',exact:true}).click();
  1017 |       await expect(page.getByRole('dialog',{name:'Review plan changes'})).toBeVisible();
  1018 |       expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'proposal review').toEqual([]);
  1019 |       await page.screenshot({path:`test-results/desk-proposal-${theme}-${width}.png`,fullPage:true});
  1020 |     }
  1021 |   });
  1022 | }
  1023 | 
  1024 | test('ending a plan requires confirmation and keeps recorded history',async ({page})=>{
  1025 |   await page.goto('/plan.html');
  1026 |   await page.getByRole('button',{name:'End current plan',exact:true}).click();
  1027 |   await expect(page.getByRole('dialog')).toContainText('Other saved plans are kept.');
  1028 |   expect(await page.evaluate(()=>window.__deskCalls.filter(call=>call.command==='abort_plan'))).toEqual([]);
  1029 |   await page.getByRole('button',{name:'Keep plan',exact:true}).click();
  1030 |   await expect(page.getByRole('dialog')).toHaveCount(0);
  1031 |   await expect(page.getByRole('button',{name:'End current plan',exact:true})).toBeVisible();
  1032 |   await page.getByRole('button',{name:'End current plan',exact:true}).click();
  1033 |   await page.getByRole('dialog').getByRole('button',{name:'End current plan',exact:true}).click();
  1034 |   await expect(page.getByRole('dialog')).toHaveCount(0);
  1035 |   expect(await page.evaluate(()=>window.__deskCalls.filter(call=>call.command==='abort_plan'))).toHaveLength(1);
  1036 |   await page.getByRole('link',{name:'Activities',exact:true}).click();
  1037 |   await expect(page.locator('.activity-row')).toHaveCount(9);
  1038 | });
  1039 | 
  1040 | test("original entry routes load the independent workspace and retain deep links", async ({page}) => {
  1041 |   const loaded=[];
  1042 |   page.on('request',request=>loaded.push(request.url()));
  1043 |   for(const [url,heading,section] of [
  1044 |     ['/', 'Training', 'Week'],
  1045 |     ['/index.html', 'Training', 'Week'],
  1046 |     ['/plan.html', 'Training', 'Plan'],
  1047 |     ['/calendar.html', 'Training', 'Month'],
  1048 |     ['/achievements.html', 'Activities', null],
  1049 |     ['/achievements.html#performance', 'Health', 'Overview'],
  1050 |     ['/achievements.html#recovery', 'Health', 'Overview'],
  1051 |     ['/achievements.html#risk', 'Health', 'Overview'],
  1052 |     ['/achievements.html#milestones', 'Health', 'Milestones'],
  1053 |     ['/injury.html', 'Health', 'Pain journal'],
  1054 |     ['/settings.html', 'Settings', null],
  1055 |     ['/support.html', 'Help', null],
  1056 |   ]) {
  1057 |     await page.goto(url);
  1058 |     await expect(page.locator('#workspace h1')).toHaveText(heading);
  1059 |     if(section) await expect(page.locator('.horizon-switch a[aria-current="page"]')).toHaveText(section);
  1060 |     await expect(page.locator('#desk-root')).toBeVisible();
  1061 |   }
  1062 |   for(const [hash,heading] of [['theme','Appearance'],['zones','Training boundaries'],['tiles','Offline maps'],['publishing','Calendar and devices'],['setup','Watch connection']]) {
  1063 |     await page.goto(`/settings.html#${hash}`);
  1064 |     await expect(page.getByRole('heading',{name:heading,exact:true})).toBeVisible();
  1065 |   }
  1066 |   await page.goto('/index.html#journal/1');
  1067 |   await expect(page.getByRole('complementary',{name:'Activity review'})).toBeVisible();
  1068 |   expect(loaded.filter(url=>/\/(app|redesign)\.css|\/src\/(dashboard|plan|calendar|achievements|injury|settings|support)\.ts|\/lib\/(navigation|overlay|workout-detail)\.ts/.test(url))).toEqual([]);
  1069 | });
  1070 | 
  1071 | for (const theme of ['light','dark']) {
  1072 |   for (const width of [320,390,768,1440]) {
  1073 |     test(`visual matrix ${theme} ${width}px`, async ({page}) => {
  1074 |       test.setTimeout(90000);
  1075 |       const errors=[];
  1076 |       page.on('pageerror',error=>errors.push(error.message));
  1077 |       await page.setViewportSize({width,height:1000});
  1078 |       for (const route of ['training','training/month','training/plan','journal','health','health/journal','health/milestones','settings','help']) {
  1079 |         await page.goto(`/desk.html?theme=${theme}#${route}`);
  1080 |         await expect(page.locator('#workspace h1')).toBeVisible();
  1081 |         await expect(page.locator('.desk-loading')).toHaveCount(0);
  1082 |         await expect(page.getByRole('alert')).toHaveCount(0);
  1083 |         expect(await page.evaluate(() => document.documentElement.scrollWidth<=innerWidth),route).toBe(true);
  1084 |         expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,route).toEqual([]);
  1085 |         await page.screenshot({path:`test-results/desk-${route.replace('/','-')}-${theme}-${width}.png`,fullPage:true});
  1086 |       }
  1087 |       await page.goto(`/desk.html?theme=${theme}#training`);
  1088 |       if (width<=800) await page.locator('.mobile-date-ribbon button').first().click();
  1089 |       await page.locator(width>800 ? '.week-board .session-tile' : '.day-itinerary .session-tile').first().click();
  1090 |       await expect(page.getByRole('dialog',{name:'Session',exact:true})).toBeVisible();
  1091 |       if (width<=800) {
  1092 |         for(let index=0;index<14;index++) {await page.keyboard.press('Tab');expect(await page.evaluate(()=>Boolean(document.activeElement?.closest('.desk-inspector')))).toBe(true);}
  1093 |       }
  1094 |       expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'session inspector').toEqual([]);
  1095 |       await page.screenshot({path:`test-results/desk-session-${theme}-${width}.png`,fullPage:true});
  1096 |       await page.keyboard.press('Escape');
  1097 |       await expect(page.getByRole('dialog',{name:'Session',exact:true})).toHaveCount(0);
  1098 |       await expect(page.getByRole('navigation',{name:'Workspaces'})).toBeVisible();
  1099 |       expect(errors).toEqual([]);
  1100 |     });
  1101 |   }
  1102 | }
  1103 | 
  1104 | test("native sync completion refreshes evidence and watch progress is visible", async ({page}) => {
  1105 |   await page.goto('/desk.html');
  1106 |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='plugin:event|listen').length)).toBe(4);
```