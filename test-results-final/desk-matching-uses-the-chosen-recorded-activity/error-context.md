# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> matching uses the chosen recorded activity
- Location: tests/desk.spec.mjs:1039:1

# Error details

```
Test timeout of 30000ms exceeded.
```

```
Error: locator.click: Test timeout of 30000ms exceeded.
Call log:
  - waiting for locator('.week-board .session-tile.movable').first()

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
      - link "Activities" [ref=e12] [cursor=pointer]:
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
          - heading "26/10/2026 - 01/11/2026" [level=2] [ref=e46]
        - button "Schedule race or test" [ref=e48] [cursor=pointer]
      - generic [ref=e51]:
        - navigation "Training horizon" [ref=e52]:
          - link "Week" [ref=e53] [cursor=pointer]:
            - /url: "#training/week"
          - link "Month" [ref=e54] [cursor=pointer]:
            - /url: "#training/month"
          - link "Plan" [ref=e55] [cursor=pointer]:
            - /url: "#training/plan"
        - generic [ref=e56]:
          - button "Previous week" [ref=e57] [cursor=pointer]
          - button "Today" [ref=e62] [cursor=pointer]
          - button "Next week" [ref=e64] [cursor=pointer]
      - generic [ref=e69]:
        - generic [ref=e70]: 5/5 sessions done
        - generic [ref=e71]: 41.0 km / 0.0 km done
        - button "Reset week" [ref=e72] [cursor=pointer]
      - generic [ref=e77]:
        - region "Mon 26/10/2026" [ref=e78]:
          - generic [ref=e79]:
            - generic [ref=e80]: Mon
            - strong [ref=e81]: "26"
          - generic [ref=e82]:
            - button "Easy Completed Easy 6 km 6.0 km" [ref=e83] [cursor=pointer]:
              - generic [ref=e84]:
                - generic [ref=e85]: Easy
                - img "Completed" [ref=e86]
              - strong [ref=e88]: Easy 6 km
              - generic [ref=e89]: 6.0 km
            - button "Recorded Completed Recorded run 6.0 km" [ref=e90] [cursor=pointer]:
              - generic [ref=e91]:
                - generic [ref=e92]: Recorded
                - img "Completed" [ref=e93]
              - strong [ref=e95]: Recorded run
              - generic [ref=e96]: 6.0 km
        - region "Tue 27/10/2026" [ref=e97]:
          - generic [ref=e98]:
            - generic [ref=e99]: Tue
            - strong [ref=e100]: "27"
          - paragraph [ref=e102]: Rest day
        - region "Wed 28/10/2026" [ref=e103]:
          - generic [ref=e104]:
            - generic [ref=e105]: Wed
            - strong [ref=e106]: "28"
          - button "Tempo Completed Tempo 5 km 5.0 km" [ref=e108] [cursor=pointer]:
            - generic [ref=e109]:
              - generic [ref=e110]: Tempo
              - img "Completed" [ref=e111]
            - strong [ref=e113]: Tempo 5 km
            - generic [ref=e114]: 5.0 km
        - region "Thu 29/10/2026" [ref=e115]:
          - generic [ref=e116]:
            - generic [ref=e117]: Thu
            - strong [ref=e118]: "29"
          - paragraph [ref=e120]: Rest day
        - region "Fri 30/10/2026" [ref=e121]:
          - generic [ref=e122]:
            - generic [ref=e123]: Fri
            - strong [ref=e124]: "30"
          - button "Easy Completed Recovery 5 km 5.0 km" [ref=e126] [cursor=pointer]:
            - generic [ref=e127]:
              - generic [ref=e128]: Easy
              - img "Completed" [ref=e129]
            - strong [ref=e131]: Recovery 5 km
            - generic [ref=e132]: 5.0 km
        - region "Sat 31/10/2026" [ref=e133]:
          - generic [ref=e134]:
            - generic [ref=e135]: Sat
            - strong [ref=e136]: "31"
          - paragraph [ref=e138]: Rest day
        - region "Sun 01/11/2026" [ref=e139]:
          - generic [ref=e140]:
            - generic [ref=e141]: Sun
            - strong [ref=e142]: "1"
            - generic [ref=e143]: Today
          - button "Long run Completed Long 19 km 19.0 km" [ref=e145] [cursor=pointer]:
            - generic [ref=e146]:
              - generic [ref=e147]: Long run
              - img "Completed" [ref=e148]
            - strong [ref=e150]: Long 19 km
            - generic [ref=e151]: 19.0 km
```

# Test source

```ts
  942  |   expect(await page.evaluate(() => window.__deskCalls.filter(call=>call.command==='push_plan_to_provider').length)).toBe(0);
  943  | });
  944  | 
  945  | test("pointer drag moves a session to the lane under the pointer and never opens it", async ({page}) => {
  946  |   await page.setViewportSize({width:1440,height:1000});
  947  |   await page.goto('/desk.html');
  948  |   await page.getByRole('button',{name:'Next week',exact:true}).click();
  949  |   const tile = page.locator('.week-board .session-tile.movable').first();
  950  |   const lanes = page.locator('.week-board .day-lane');
  951  |   const fromIndex = await lanes.evaluateAll(nodes => nodes.findIndex(node => node.querySelector('.session-tile.movable')));
  952  |   const target = lanes.nth(fromIndex + 1);
  953  |   const a = await tile.boundingBox();
  954  |   const b = await target.boundingBox();
  955  |   await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
  956  |   await page.mouse.down();
  957  |   await page.mouse.move(a.x + a.width / 2 + 30, a.y + a.height / 2, {steps: 4});
  958  |   await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2, {steps: 8});
  959  |   await expect(target).toHaveAttribute('data-drop-target', '');
  960  |   await page.mouse.up();
  961  |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='move_workout').map(call=>call.args.date))).toHaveLength(1);
  962  |   await expect(page.getByRole('status')).toContainText('Moved to');
  963  |   // The click the browser fires after the release must not open the inspector.
  964  |   await expect(page.getByRole('complementary',{name:'Session',exact:true})).toHaveCount(0);
  965  | });
  966  | 
  967  | test("month view marks finished sessions like the week board does", async ({page}) => {
  968  |   await page.setViewportSize({width:1440,height:1000});
  969  |   await page.goto('/desk.html#training/month');
  970  |   const tiles = page.locator('.month-sessions .session-tile');
  971  |   await expect(tiles.first()).toBeVisible();
  972  |   // The fixture links finished workouts to an activity, so their tiles carry
  973  |   // the same Completed mark as on the week board; the kind label is what the
  974  |   // month cell gives up for space, never the mark.
  975  |   const done = page.locator('.month-sessions .session-tile [aria-label="Completed"]');
  976  |   expect(await done.count()).toBeGreaterThan(0);
  977  |   await expect(done.first()).toBeVisible();
  978  |   await expect(page.locator('.month-sessions .session-kind').first()).toBeHidden();
  979  |   // Today stays highlighted in the month grid: the number carries
  980  |   // aria-current="date" and wears the filled disc, not plain ink.
  981  |   const todayCell = page.locator('.month-number[aria-current="date"]');
  982  |   await expect(todayCell).toHaveCount(1);
  983  |   await expect.poll(async () => (await todayCell.evaluate(el => getComputedStyle(el).backgroundColor)) !== 'rgba(0, 0, 0, 0)').toBe(true);
  984  | });
  985  | 
  986  | test("schedule mutations retain intent and Undo restores the original date", async ({page}) => {
  987  |   await page.setViewportSize({width:1440,height:1000});
  988  |   await page.goto('/desk.html');
  989  |   await page.getByRole('button',{name:'Next week',exact:true}).click();
  990  |   await page.locator('.week-board .session-tile.movable').first().click();
  991  |   const original = await page.getByLabel('Run on',{exact:true}).inputValue();
  992  |   const next = new Date(`${original}T12:00:00`); next.setDate(next.getDate()+1);
  993  |   const moved = `${next.getFullYear()}-${String(next.getMonth()+1).padStart(2,'0')}-${String(next.getDate()).padStart(2,'0')}`;
  994  |   await page.getByLabel('Run on',{exact:true}).fill(moved);
  995  |   await page.getByRole('button',{name:'Move session',exact:true}).click();
  996  |   await expect(page.getByRole('status')).toContainText('Moved to');
  997  |   await page.getByRole('button',{name:'Undo',exact:true}).click();
  998  |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='move_workout').map(call=>call.args.date))).toEqual([moved,original]);
  999  |   await page.locator('.week-board .session-tile.movable').first().click();
  1000 |   await page.getByRole('button',{name:'Skip session',exact:true}).click();
  1001 |   await expect(page.getByRole('status')).toContainText('Session skipped');
  1002 |   await page.getByRole('button',{name:'Undo',exact:true}).click();
  1003 |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='set_workout_skipped').map(call=>call.args.skipped))).toEqual([true,false]);
  1004 | });
  1005 | 
  1006 | test("week summary shows run mileage against the plan's planned mileage", async ({page}) => {
  1007 |   await page.setViewportSize({width:1440,height:1000});
  1008 |   await page.goto('/desk.html');
  1009 |   const strip = page.locator('.schedule-summary');
  1010 |   await expect(strip).toBeVisible();
  1011 |   // The km reading carries the plan's denominator: "X km / Y km done", so a
  1012 |   // week at 38 run of 30 planned reads as 38/30 rather than a bare 38.
  1013 |   await expect(strip.locator('span').nth(1)).toContainText("/");
  1014 |   await expect(strip.locator('span').nth(1)).toContainText("done");
  1015 | });
  1016 | 
  1017 | test("reset week undoes a scheduled benchmark and a moved session", async ({page}) => {
  1018 |   await page.setViewportSize({width:1440,height:1000});
  1019 |   await page.goto('/desk.html');
  1020 |   await page.getByRole('button',{name:'Next week',exact:true}).click();
  1021 |   // Schedule a benchmark over a planned session, the way the user did.
  1022 |   await page.getByRole('button',{name:'Schedule race or test',exact:true}).click();
  1023 |   const dialog = page.getByRole('dialog',{name:'Schedule race or test',exact:true});
  1024 |   await dialog.getByRole('button',{name:'Schedule benchmark',exact:true}).click();
  1025 |   await expect(dialog).toHaveCount(0);
  1026 |   await expect(page.locator('.week-board')).toContainText('Benchmark: parkrun');
  1027 |   // The reset offer appears on the summary strip for the week horizon.
  1028 |   await page.getByRole('button',{name:'Reset week',exact:true}).click();
  1029 |   const reset = page.getByRole('dialog',{name:'Reset this week',exact:true});
  1030 |   await expect(reset).toBeVisible();
  1031 |   await expect(reset.locator('.reset-list li').filter({hasText:'Benchmark'})).toBeVisible();
  1032 |   await reset.getByRole('button',{name:/^Reset \d+$/}).click();
  1033 |   await expect(reset).toHaveCount(0);
  1034 |   await expect(page.getByRole('status')).toContainText('back on the plan');
  1035 |   await expect(page.locator('.week-board')).not.toContainText('Benchmark: parkrun');
  1036 |   expect(await page.evaluate(() => window.__deskCalls.some(call=>call.command==='reset_week'))).toBe(true);
  1037 | });
  1038 | 
  1039 | test("matching uses the chosen recorded activity", async ({page}) => {
  1040 |   await page.setViewportSize({width:1440,height:1000});
  1041 |   await page.goto('/desk.html?unmatched=1');
> 1042 |   await page.locator('.week-board .session-tile.movable').first().click();
       |                                                                   ^ Error: locator.click: Test timeout of 30000ms exceeded.
  1043 |   await page.getByRole('combobox',{name:'Recorded activity'}).selectOption('99');
  1044 |   await page.getByRole('button',{name:'Match run',exact:true}).click();
  1045 |   await expect(page.getByRole('status')).toContainText('Recorded run matched');
  1046 |   expect(await page.evaluate(() => window.__deskCalls.find(call=>call.command==='match_calendar_activity').args.activityId)).toBe(99);
  1047 | });
  1048 | 
  1049 | for (const action of ['Apply','Dismiss']) {
  1050 |   test(`${action} proposal requires review and preserves a failed decision`, async ({page}) => {
  1051 |     await page.goto('/desk.html?proposal=1#health/journal');
  1052 |     await page.getByRole('button',{name:'Review changes',exact:true}).click();
  1053 |     await expect(page.getByRole('dialog')).toContainText('Easy run');
  1054 |     expect(await page.evaluate(() => window.__deskCalls.some(call=>['apply_proposal','dismiss_proposal'].includes(call.command)))).toBe(false);
  1055 |     if (action==='Dismiss') await page.getByRole('textbox',{name:'Reason for dismissing (optional)'}).fill('Prefer to review with my clinician.');
  1056 |     await page.evaluate(command => {window.__deskFail=command;},`${action.toLowerCase()}_proposal`);
  1057 |     await page.getByRole('button',{name:`${action} changes`,exact:true}).click();
  1058 |     await expect(page.getByRole('dialog').getByRole('alert')).toContainText('Unable to save');
  1059 |     await page.evaluate(() => {window.__deskFail='';});
  1060 |     await page.getByRole('button',{name:`${action} changes`,exact:true}).click();
  1061 |     await expect(page.getByRole('dialog')).toHaveCount(0);
  1062 |     await expect(page.getByRole('button',{name:'Review changes',exact:true})).toHaveCount(0);
  1063 |   });
  1064 | }
  1065 | 
  1066 | for (const width of [390,1440]) {
  1067 |   test(`recorded route map renders geographic pixels and responds to zoom at ${width}px`, async ({page}) => {
  1068 |     await page.setViewportSize({width,height:1000});
  1069 |     await page.goto('/desk.html#journal/1');
  1070 |     await expect(page.getByRole('region',{name:'Recorded GPS route'})).toBeVisible();
  1071 |     const pixels = () => page.locator('.maplibregl-canvas').evaluate(canvas => {
  1072 |       const context=canvas.getContext('webgl2');
  1073 |       const bytes=new Uint8Array(context.drawingBufferWidth*context.drawingBufferHeight*4);
  1074 |       context.readPixels(0,0,context.drawingBufferWidth,context.drawingBufferHeight,context.RGBA,context.UNSIGNED_BYTE,bytes);
  1075 |       let colored=0, fingerprint=0;
  1076 |       for(let index=0;index<bytes.length;index+=64) { if(bytes[index+3]>0 && bytes[index+1]>bytes[index]+10 && bytes[index+1]>bytes[index+2]+5) colored++; fingerprint=(fingerprint+bytes[index]*(index+1))%1000000007; }
  1077 |       return {colored,fingerprint};
  1078 |     });
  1079 |     await expect.poll(async () => (await pixels()).colored).toBeGreaterThan(10);
  1080 |     const before=await pixels();
  1081 |     await page.getByRole('button',{name:'Zoom in',exact:true}).click();
  1082 |     await expect.poll(async () => (await pixels()).fingerprint).not.toBe(before.fingerprint);
  1083 |     await page.getByRole('button',{name:'Fit route',exact:true}).click();
  1084 |     expect(await page.evaluate(() => document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  1085 |     await page.screenshot({path:`test-results/desk-map-${width}.png`,fullPage:true});
  1086 |   });
  1087 | }
  1088 | 
  1089 | test("independent shell loads only new presentation", async ({ page }) => {
  1090 |   const loaded = [];
  1091 |   page.on("request", request => loaded.push(request.url()));
  1092 |   await page.goto("/desk.html");
  1093 |   await expect(page.getByRole("heading", { name: "Training", exact: true })).toBeVisible();
  1094 |   await expect(page.getByRole("navigation", { name: "Workspaces" }).getByRole("link")).toHaveCount(3);
  1095 |   await expect(page.locator(".day-itinerary")).toBeVisible();
  1096 |   expect(loaded.filter(url => /\/(app|redesign)\.css|\/src\/(dashboard|plan|calendar|achievements|injury|settings|support)\.ts|\/lib\/(navigation|overlay|workout-detail)\.ts/.test(url))).toEqual([]);
  1097 | });
  1098 | 
  1099 | test("mobile date selection opens a full-screen session and returns to its opener", async ({ page }) => {
  1100 |   await page.setViewportSize({width:390, height:844});
  1101 |   await page.emulateMedia({reducedMotion:"no-preference"});
  1102 |   await page.goto("/desk.html#health");
  1103 |   await page.getByRole("navigation", {name:"Workspaces"}).getByRole("link", {name:"Training"}).click();
  1104 |   await page.locator(".mobile-date-ribbon button").first().click();
  1105 |   const session = page.locator(".day-itinerary .session-tile").first();
  1106 |   await session.click();
  1107 |   await expect(page.getByRole("complementary", { name: "Session", exact: true })).toBeVisible();
  1108 |   await expect.poll(async () => {
  1109 |     const bounds = await page.locator(".desk-inspector").boundingBox();
  1110 |     return bounds && Object.fromEntries(Object.entries(bounds).map(([key,value]) => [key,Math.round(value)]));
  1111 |   }).toEqual({x:0,y:0,width:390,height:844});
  1112 |   await expect(page.locator(".prescription")).toContainText("Repeat 4 times:");
  1113 |   await page.getByRole("button", { name: "Back", exact: true }).click();
  1114 |   await expect(session).toBeFocused();
  1115 | });
  1116 | 
  1117 | for (const width of [390, 1440]) {
  1118 |   test(`benchmark scheduling submits parkrun and custom test contracts at ${width}px`, async ({ page }) => {
  1119 |     await page.setViewportSize({ width, height: 900 });
  1120 |     await page.goto("/desk.html#training/week");
  1121 |     const open = page.getByRole("button", { name: "Schedule race or test", exact: true });
  1122 |     await open.click();
  1123 |     const dialog = page.getByRole("dialog", { name: "Schedule race or test", exact: true });
  1124 |     await expect(dialog.getByLabel("Distance (km)")).toHaveValue("5");
  1125 |     await expect(dialog.getByLabel("Distance (km)")).toBeDisabled();
  1126 |     const date = await dialog.getByLabel("Date", { exact: true }).inputValue();
  1127 |     expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  1128 |     await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
  1129 |     await expect(dialog).toHaveCount(0);
  1130 |     expect(await page.evaluate(() => window.__deskCalls.find(call => call.command === "schedule_benchmark").args)).toEqual({ date, name: "parkrun", distanceM: 5000 });
  1131 |     await expect(page.locator(".day-itinerary")).toContainText("Benchmark: parkrun");
  1132 |     await open.click();
  1133 |     await dialog.getByRole("combobox", { name: "Event", exact: true }).selectOption("trial");
  1134 |     await dialog.getByLabel("Event name", { exact: true }).fill("3 km fitness test");
  1135 |     await dialog.getByLabel("Distance (km)").fill("3");
  1136 |     await page.evaluate(() => { window.__deskFail = "schedule_benchmark"; });
  1137 |     await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
  1138 |     await expect(dialog.getByRole("alert")).toContainText("Unable to save right now");
  1139 |     await expect(dialog.getByLabel("Event name", { exact: true })).toHaveValue("3 km fitness test");
  1140 |     await page.screenshot({ path: `test-results/benchmark-${width}.png`, animations: "disabled" });
  1141 |     await page.evaluate(() => { window.__deskFail = ""; });
  1142 |     await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
```