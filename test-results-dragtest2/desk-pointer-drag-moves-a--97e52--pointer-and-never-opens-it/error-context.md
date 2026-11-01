# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> pointer drag moves a session to the lane under the pointer and never opens it
- Location: tests/desk.spec.mjs:931:1

# Error details

```
ReferenceError: from is not defined
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
      - link "Journal" [ref=e12] [cursor=pointer]:
        - /url: "#journal"
      - link "Health" [ref=e19] [cursor=pointer]:
        - /url: "#health"
    - generic [ref=e23]:
      - button "Sync activities" [ref=e24] [cursor=pointer]
      - button "Help" [ref=e29] [cursor=pointer]
      - button "Settings" [ref=e34] [cursor=pointer]
  - main [ref=e39]:
    - generic [ref=e40]:
      - heading "Training" [level=1] [ref=e41]
      - generic [ref=e42]: COROS connected
    - generic [ref=e43]:
      - generic [ref=e44]:
        - generic [ref=e45]:
          - text: Race preparation / 21K
          - heading "02/11/2026 - 08/11/2026" [level=2] [ref=e46]
        - generic [ref=e47]:
          - button "Schedule race or test" [ref=e48] [cursor=pointer]
          - button "Update watch" [ref=e51] [cursor=pointer]
      - generic [ref=e57]:
        - navigation "Training horizon" [ref=e58]:
          - link "Week" [ref=e59] [cursor=pointer]:
            - /url: "#training/week"
          - link "Month" [ref=e60] [cursor=pointer]:
            - /url: "#training/month"
          - link "Plan" [ref=e61] [cursor=pointer]:
            - /url: "#training/plan"
        - generic [ref=e62]:
          - button "Previous week" [ref=e63] [cursor=pointer]
          - button "Today" [ref=e68] [cursor=pointer]
          - button "Next week" [ref=e70] [cursor=pointer]
      - generic [ref=e78]:
        - generic [ref=e79]:
          - region "Mon 02/11/2026" [ref=e80]:
            - generic [ref=e81]:
              - generic [ref=e82]: Mon
              - strong [ref=e83]: "2"
            - paragraph [ref=e85]: Rest day
          - region "Tue 03/11/2026" [ref=e86]:
            - generic [ref=e87]:
              - generic [ref=e88]: Tue
              - strong [ref=e89]: "3"
            - button "Easy Easy 6 km 6.0 km Rescheduled" [ref=e91] [cursor=pointer]:
              - generic [ref=e92]: Easy
              - strong [ref=e93]: Easy 6 km
              - generic [ref=e94]: 6.0 km
              - generic [ref=e95]: Rescheduled
          - region "Wed 04/11/2026" [ref=e96]:
            - generic [ref=e97]:
              - generic [ref=e98]: Wed
              - strong [ref=e99]: "4"
            - button "Tempo Tempo 5 km 5.0 km" [ref=e101] [cursor=pointer]:
              - generic [ref=e102]: Tempo
              - strong [ref=e103]: Tempo 5 km
              - generic [ref=e104]: 5.0 km
          - region "Thu 05/11/2026" [ref=e105]:
            - generic [ref=e106]:
              - generic [ref=e107]: Thu
              - strong [ref=e108]: "5"
            - paragraph [ref=e110]: Rest day
          - region "Fri 06/11/2026" [ref=e111]:
            - generic [ref=e112]:
              - generic [ref=e113]: Fri
              - strong [ref=e114]: "6"
            - button "Easy Recovery 5 km 5.0 km" [ref=e116] [cursor=pointer]:
              - generic [ref=e117]: Easy
              - strong [ref=e118]: Recovery 5 km
              - generic [ref=e119]: 5.0 km
          - region "Sat 07/11/2026" [ref=e120]:
            - generic [ref=e121]:
              - generic [ref=e122]: Sat
              - strong [ref=e123]: "7"
            - paragraph [ref=e125]: Rest day
          - region "Sun 08/11/2026" [ref=e126]:
            - generic [ref=e127]:
              - generic [ref=e128]: Sun
              - strong [ref=e129]: "8"
            - button "Long run Long 20 km 20.0 km" [ref=e131] [cursor=pointer]:
              - generic [ref=e132]: Long run
              - strong [ref=e133]: Long 20 km
              - generic [ref=e134]: 20.0 km
        - generic [ref=e135]:
          - generic [ref=e136]: 4 sessions in this week
          - generic [ref=e137]: 36.0 km
  - status [ref=e138]:
    - generic [ref=e139]: Moved to 03/11/2026.
    - button "Undo" [ref=e140] [cursor=pointer]
    - button "Dismiss notification" [ref=e141] [cursor=pointer]
```

# Test source

```ts
  851  |   const loaded=[];
  852  |   page.on('request',request=>loaded.push(request.url()));
  853  |   for(const [url,heading,section] of [
  854  |     ['/', 'Training', 'Week'],
  855  |     ['/index.html', 'Training', 'Week'],
  856  |     ['/plan.html', 'Training', 'Plan'],
  857  |     ['/calendar.html', 'Training', 'Month'],
  858  |     ['/achievements.html', 'Journal', 'Activities'],
  859  |     ['/achievements.html#performance', 'Health', 'Overview'],
  860  |     ['/achievements.html#recovery', 'Health', 'Overview'],
  861  |     ['/achievements.html#risk', 'Health', 'Overview'],
  862  |     ['/achievements.html#milestones', 'Health', 'Milestones'],
  863  |     ['/achievements.html#routes', 'Journal', 'Training map'],
  864  |     ['/injury.html', 'Health', 'Pain journal'],
  865  |     ['/settings.html', 'Settings', null],
  866  |     ['/support.html', 'Help', null],
  867  |   ]) {
  868  |     await page.goto(url);
  869  |     await expect(page.locator('#workspace h1')).toHaveText(heading);
  870  |     if(section) await expect(page.locator('.horizon-switch a[aria-current="page"]')).toHaveText(section);
  871  |     await expect(page.locator('#desk-root')).toBeVisible();
  872  |   }
  873  |   for(const [hash,heading] of [['theme','Appearance'],['zones','Training boundaries'],['tiles','Offline maps'],['publishing','Calendar and devices'],['setup','Watch connection']]) {
  874  |     await page.goto(`/settings.html#${hash}`);
  875  |     await expect(page.getByRole('heading',{name:heading,exact:true})).toBeVisible();
  876  |   }
  877  |   await page.goto('/index.html#journal/1');
  878  |   await expect(page.getByRole('complementary',{name:'Activity review'})).toBeVisible();
  879  |   expect(loaded.filter(url=>/\/(app|redesign)\.css|\/src\/(dashboard|plan|calendar|achievements|injury|settings|support)\.ts|\/lib\/(navigation|overlay|workout-detail)\.ts/.test(url))).toEqual([]);
  880  | });
  881  | 
  882  | for (const theme of ['light','dark']) {
  883  |   for (const width of [320,390,768,1440]) {
  884  |     test(`visual matrix ${theme} ${width}px`, async ({page}) => {
  885  |       test.setTimeout(90000);
  886  |       const errors=[];
  887  |       page.on('pageerror',error=>errors.push(error.message));
  888  |       await page.setViewportSize({width,height:1000});
  889  |       for (const route of ['training','training/month','training/plan','journal','journal/map','health','health/journal','health/milestones','settings','help']) {
  890  |         await page.goto(`/desk.html?theme=${theme}#${route}`);
  891  |         await expect(page.locator('#workspace h1')).toBeVisible();
  892  |         await expect(page.locator('.desk-loading')).toHaveCount(0);
  893  |         await expect(page.getByRole('alert')).toHaveCount(0);
  894  |         expect(await page.evaluate(() => document.documentElement.scrollWidth<=innerWidth),route).toBe(true);
  895  |         expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,route).toEqual([]);
  896  |         await page.screenshot({path:`test-results/desk-${route.replace('/','-')}-${theme}-${width}.png`,fullPage:true});
  897  |       }
  898  |       await page.goto(`/desk.html?theme=${theme}#training`);
  899  |       await page.locator(width>800 ? '.week-board .session-tile.movable' : '.day-itinerary .session-tile.movable').first().click();
  900  |       await expect(page.getByRole('complementary',{name:'Session',exact:true})).toBeVisible();
  901  |       if (width<=800) {
  902  |         for(let index=0;index<14;index++) {await page.keyboard.press('Tab');expect(await page.evaluate(()=>Boolean(document.activeElement?.closest('.desk-inspector')))).toBe(true);}
  903  |       }
  904  |       expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'session inspector').toEqual([]);
  905  |       await page.screenshot({path:`test-results/desk-session-${theme}-${width}.png`,fullPage:true});
  906  |       await page.keyboard.press('Escape');
  907  |       await expect(page.getByRole('complementary',{name:'Session',exact:true})).toHaveCount(0);
  908  |       await expect(page.getByRole('navigation',{name:'Workspaces'})).toBeVisible();
  909  |       expect(errors).toEqual([]);
  910  |     });
  911  |   }
  912  | }
  913  | 
  914  | test("native sync completion refreshes evidence and watch progress is visible", async ({page}) => {
  915  |   await page.goto('/desk.html');
  916  |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='plugin:event|listen').length)).toBe(3);
  917  |   await page.evaluate(() => window.__deskEmit('sync://started', null));
  918  |   await expect(page.getByRole('button',{name:'Syncing activities'})).toBeDisabled();
  919  |   const before = await page.evaluate(() => window.__deskCalls.filter(call=>call.command==='current_plan').length);
  920  |   await page.evaluate(() => window.__deskEmit('sync://completed',{ok:true}));
  921  |   await expect(page.getByRole('button',{name:'Sync activities'})).toBeEnabled();
  922  |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='current_plan').length)).toBeGreaterThan(before);
  923  |   await page.evaluate(() => {window.__deskHoldPush=true;});
  924  |   await page.getByRole('button',{name:/Send to watch|Update watch/}).click();
  925  |   await page.evaluate(() => window.__deskEmit('plan://sync',{op:'push',percent:45,stage:'Publishing sessions'}));
  926  |   await expect(page.getByRole('progressbar',{name:'Watch publishing progress'})).toHaveAttribute('value','45');
  927  |   await page.evaluate(() => window.__deskResolvePush());
  928  |   await expect(page.getByRole('status')).toContainText('Plan sent to COROS');
  929  | });
  930  | 
  931  | test("pointer drag moves a session to the lane under the pointer and never opens it", async ({page}) => {
  932  |   await page.setViewportSize({width:1440,height:1000});
  933  |   await page.goto('/desk.html');
  934  |   await page.getByRole('button',{name:'Next week',exact:true}).click();
  935  |   const tile = page.locator('.week-board .session-tile.movable').first();
  936  |   const lanes = page.locator('.week-board .day-lane');
  937  |   const fromIndex = await lanes.evaluateAll(nodes => nodes.findIndex(node => node.querySelector('.session-tile.movable')));
  938  |   const target = lanes.nth(fromIndex + 1);
  939  |   const a = await tile.boundingBox();
  940  |   const b = await target.boundingBox();
  941  |   await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
  942  |   await page.mouse.down();
  943  |   await page.mouse.move(a.x + a.width / 2 + 30, a.y + a.height / 2, {steps: 4});
  944  |   await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2, {steps: 8});
  945  |   await expect(target).toHaveAttribute('data-drop-target', '');
  946  |   await page.mouse.up();
  947  |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='move_workout').map(call=>call.args.date))).toHaveLength(1);
  948  |   await expect(page.getByRole('status')).toContainText('Moved to');
  949  |   // The click the browser fires after the release must not open the inspector.
  950  |   await expect(page.getByRole('complementary',{name:'Session',exact:true})).toHaveCount(0);
> 951  |   await from.waitFor();
       |   ^ ReferenceError: from is not defined
  952  | });
  953  | 
  954  | test("schedule mutations retain intent and Undo restores the original date", async ({page}) => {
  955  |   await page.setViewportSize({width:1440,height:1000});
  956  |   await page.goto('/desk.html');
  957  |   await page.getByRole('button',{name:'Next week',exact:true}).click();
  958  |   await page.locator('.week-board .session-tile.movable').first().click();
  959  |   const original = await page.getByLabel('Run on',{exact:true}).inputValue();
  960  |   const next = new Date(`${original}T12:00:00`); next.setDate(next.getDate()+1);
  961  |   const moved = `${next.getFullYear()}-${String(next.getMonth()+1).padStart(2,'0')}-${String(next.getDate()).padStart(2,'0')}`;
  962  |   await page.getByLabel('Run on',{exact:true}).fill(moved);
  963  |   await page.getByRole('button',{name:'Move session',exact:true}).click();
  964  |   await expect(page.getByRole('status')).toContainText('Moved to');
  965  |   await page.getByRole('button',{name:'Undo',exact:true}).click();
  966  |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='move_workout').map(call=>call.args.date))).toEqual([moved,original]);
  967  |   await page.locator('.week-board .session-tile.movable').first().click();
  968  |   await page.getByRole('button',{name:'Skip session',exact:true}).click();
  969  |   await expect(page.getByRole('status')).toContainText('Session skipped');
  970  |   await page.getByRole('button',{name:'Undo',exact:true}).click();
  971  |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='set_workout_skipped').map(call=>call.args.skipped))).toEqual([true,false]);
  972  | });
  973  | 
  974  | test("matching uses the chosen recorded activity", async ({page}) => {
  975  |   await page.setViewportSize({width:1440,height:1000});
  976  |   await page.goto('/desk.html?unmatched=1');
  977  |   await page.locator('.week-board .session-tile.movable').first().click();
  978  |   await page.getByRole('combobox',{name:'Recorded activity'}).selectOption('99');
  979  |   await page.getByRole('button',{name:'Match run',exact:true}).click();
  980  |   await expect(page.getByRole('status')).toContainText('Recorded run matched');
  981  |   expect(await page.evaluate(() => window.__deskCalls.find(call=>call.command==='match_calendar_activity').args.activityId)).toBe(99);
  982  | });
  983  | 
  984  | for (const action of ['Apply','Dismiss']) {
  985  |   test(`${action} proposal requires review and preserves a failed decision`, async ({page}) => {
  986  |     await page.goto('/desk.html?proposal=1#health/journal');
  987  |     await page.getByRole('button',{name:'Review changes',exact:true}).click();
  988  |     await expect(page.getByRole('dialog')).toContainText('Easy run');
  989  |     expect(await page.evaluate(() => window.__deskCalls.some(call=>['apply_proposal','dismiss_proposal'].includes(call.command)))).toBe(false);
  990  |     if (action==='Dismiss') await page.getByRole('textbox',{name:'Reason for dismissing (optional)'}).fill('Prefer to review with my clinician.');
  991  |     await page.evaluate(command => {window.__deskFail=command;},`${action.toLowerCase()}_proposal`);
  992  |     await page.getByRole('button',{name:`${action} changes`,exact:true}).click();
  993  |     await expect(page.getByRole('dialog').getByRole('alert')).toContainText('Unable to save');
  994  |     await page.evaluate(() => {window.__deskFail='';});
  995  |     await page.getByRole('button',{name:`${action} changes`,exact:true}).click();
  996  |     await expect(page.getByRole('dialog')).toHaveCount(0);
  997  |     await expect(page.getByRole('button',{name:'Review changes',exact:true})).toHaveCount(0);
  998  |   });
  999  | }
  1000 | 
  1001 | for (const width of [390,1440]) {
  1002 |   test(`training map renders geographic pixels and responds to zoom at ${width}px`, async ({page}) => {
  1003 |     await page.setViewportSize({width,height:1000});
  1004 |     await page.goto('/desk.html#journal/map');
  1005 |     await expect(page.getByRole('region',{name:'Training route heatmap'})).toBeVisible();
  1006 |     const pixels = () => page.locator('.maplibregl-canvas').evaluate(canvas => {
  1007 |       const context=canvas.getContext('webgl2');
  1008 |       const bytes=new Uint8Array(context.drawingBufferWidth*context.drawingBufferHeight*4);
  1009 |       context.readPixels(0,0,context.drawingBufferWidth,context.drawingBufferHeight,context.RGBA,context.UNSIGNED_BYTE,bytes);
  1010 |       let colored=0, fingerprint=0;
  1011 |       for(let index=0;index<bytes.length;index+=64) { if(bytes[index+3]>0 && bytes[index+1]>bytes[index]+10 && bytes[index+1]>bytes[index+2]+5) colored++; fingerprint=(fingerprint+bytes[index]*(index+1))%1000000007; }
  1012 |       return {colored,fingerprint};
  1013 |     });
  1014 |     await expect.poll(async () => (await pixels()).colored).toBeGreaterThan(10);
  1015 |     const before=await pixels();
  1016 |     await page.getByRole('button',{name:'Zoom in',exact:true}).click();
  1017 |     await expect.poll(async () => (await pixels()).fingerprint).not.toBe(before.fingerprint);
  1018 |     await page.getByRole('button',{name:'Fit training area',exact:true}).click();
  1019 |     expect(await page.evaluate(() => document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  1020 |     await page.screenshot({path:`test-results/desk-map-${width}.png`,fullPage:true});
  1021 |   });
  1022 | }
  1023 | 
  1024 | test("independent shell loads only new presentation", async ({ page }) => {
  1025 |   const loaded = [];
  1026 |   page.on("request", request => loaded.push(request.url()));
  1027 |   await page.goto("/desk.html");
  1028 |   await expect(page.getByRole("heading", { name: "Training", exact: true })).toBeVisible();
  1029 |   await expect(page.getByRole("navigation", { name: "Workspaces" }).getByRole("link")).toHaveCount(3);
  1030 |   await expect(page.locator(".day-itinerary")).toBeVisible();
  1031 |   expect(loaded.filter(url => /\/(app|redesign)\.css|\/src\/(dashboard|plan|calendar|achievements|injury|settings|support)\.ts|\/lib\/(navigation|overlay|workout-detail)\.ts/.test(url))).toEqual([]);
  1032 | });
  1033 | 
  1034 | test("mobile date selection opens a full-screen session and returns to its opener", async ({ page }) => {
  1035 |   await page.setViewportSize({width:390, height:844});
  1036 |   await page.emulateMedia({reducedMotion:"no-preference"});
  1037 |   await page.goto("/desk.html#health");
  1038 |   await page.getByRole("navigation", {name:"Workspaces"}).getByRole("link", {name:"Training"}).click();
  1039 |   await page.locator(".mobile-date-ribbon button").first().click();
  1040 |   const session = page.locator(".day-itinerary .session-tile").first();
  1041 |   await session.click();
  1042 |   await expect(page.getByRole("complementary", { name: "Session", exact: true })).toBeVisible();
  1043 |   await expect.poll(async () => {
  1044 |     const bounds = await page.locator(".desk-inspector").boundingBox();
  1045 |     return bounds && Object.fromEntries(Object.entries(bounds).map(([key,value]) => [key,Math.round(value)]));
  1046 |   }).toEqual({x:0,y:0,width:390,height:844});
  1047 |   await expect(page.locator(".prescription")).toContainText("Repeat 4 times:");
  1048 |   await page.getByRole("button", { name: "Back", exact: true }).click();
  1049 |   await expect(session).toBeFocused();
  1050 | });
  1051 | 
```