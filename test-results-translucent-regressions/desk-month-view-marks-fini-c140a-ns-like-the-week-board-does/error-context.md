# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> month view marks finished sessions like the week board does
- Location: tests/desk.spec.mjs:1153:1

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#training/month
Call log:
  - navigating to "http://127.0.0.1:1500/desk.html#training/month", waiting until "load"

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
  1107 |   await page.evaluate(() => window.__deskEmit('sync://started', null));
  1108 |   await expect(page.getByRole('button',{name:'Syncing activities'})).toBeDisabled();
  1109 |   const before = await page.evaluate(() => window.__deskCalls.filter(call=>call.command==='current_plan').length);
  1110 |   await page.evaluate(() => window.__deskEmit('sync://completed',{ok:true}));
  1111 |   await expect(page.getByRole('button',{name:'Sync activities'})).toBeEnabled();
  1112 |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='current_plan').length)).toBeGreaterThan(before);
  1113 |   // A plan that has never reached the watch is pushed by itself: the page
  1114 |   // opens on ?noPush=1&holdPush=1, the fixture holds the push open, and the
  1115 |   // backend's plan://sync stages must drive the progress bar unattended.
  1116 |   await page.goto('/desk.html?noPush=1&holdPush=1');
  1117 |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='push_plan_to_provider').length)).toBe(1);
  1118 |   await page.evaluate(() => window.__deskEmit('plan://sync',{op:'push',percent:45,stage:'Publishing sessions'}));
  1119 |   await expect(page.getByRole('progressbar',{name:'Watch publishing progress'})).toHaveAttribute('value','45');
  1120 |   await page.evaluate(() => window.__deskResolvePush());
  1121 |   await expect(page.getByRole('status')).toContainText('Plan sent to COROS');
  1122 | });
  1123 | 
  1124 | test("a plan already on the watch is never re-pushed by the page", async ({page}) => {
  1125 |   await page.goto('/desk.html');
  1126 |   await expect(page.locator('#workspace h1')).toBeVisible();
  1127 |   await page.waitForTimeout(500);
  1128 |   expect(await page.evaluate(() => window.__deskCalls.filter(call=>call.command==='push_plan_to_provider').length)).toBe(0);
  1129 | });
  1130 | 
  1131 | test("pointer drag moves a session to the lane under the pointer and never opens it", async ({page}) => {
  1132 |   await page.setViewportSize({width:1440,height:1000});
  1133 |   await page.goto('/desk.html');
  1134 |   await page.getByRole('button',{name:'Next week',exact:true}).click();
  1135 |   const tile = page.locator('.week-board .session-tile.movable').first();
  1136 |   const lanes = page.locator('.week-board .day-lane');
  1137 |   const fromIndex = await lanes.evaluateAll(nodes => nodes.findIndex(node => node.querySelector('.session-tile.movable')));
  1138 |   const target = lanes.nth(fromIndex + 1);
  1139 |   const a = await tile.boundingBox();
  1140 |   const b = await target.boundingBox();
  1141 |   await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
  1142 |   await page.mouse.down();
  1143 |   await page.mouse.move(a.x + a.width / 2 + 30, a.y + a.height / 2, {steps: 4});
  1144 |   await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2, {steps: 8});
  1145 |   await expect(target).toHaveAttribute('data-drop-target', '');
  1146 |   await page.mouse.up();
  1147 |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='move_workout').map(call=>call.args.date))).toHaveLength(1);
  1148 |   await expect(page.getByRole('status')).toContainText('Moved to');
  1149 |   // The click the browser fires after the release must not open the inspector.
  1150 |   await expect(page.getByRole('dialog',{name:'Session',exact:true})).toHaveCount(0);
  1151 | });
  1152 | 
  1153 | test("month view marks finished sessions like the week board does", async ({page}) => {
  1154 |   await page.setViewportSize({width:1440,height:1000});
> 1155 |   await page.goto('/desk.html#training/month');
       |              ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#training/month
  1156 |   const tiles = page.locator('.month-sessions .session-tile');
  1157 |   await expect(tiles.first()).toBeVisible();
  1158 |   const done = page.locator('.month-sessions .session-tile [aria-label="Completed"]');
  1159 |   expect(await done.count()).toBeGreaterThan(0);
  1160 |   await expect(done.first()).toBeVisible();
  1161 |   await expect(page.locator('.month-sessions .session-kind').first()).toBeVisible();
  1162 |   await expect(page.locator('button.month-number')).toHaveCount(0);
  1163 |   await expect(page.locator('.month-day').first()).toHaveCSS('border-radius', '0px');
  1164 |   await expect(page.locator('.month-day').last()).toHaveCSS('border-bottom-width', '1px');
  1165 |   await expect(page.locator('.month-day').last()).toHaveCSS('border-right-width', '1px');
  1166 |   // Today stays highlighted in the month grid: the number carries
  1167 |   // aria-current="date" and wears the filled disc, not plain ink.
  1168 |   const todayCell = page.locator('.month-number[aria-current="date"]');
  1169 |   await expect(todayCell).toHaveCount(1);
  1170 |   await expect.poll(async () => (await todayCell.evaluate(el => getComputedStyle(el).backgroundColor)) !== 'rgba(0, 0, 0, 0)').toBe(true);
  1171 | });
  1172 | 
  1173 | test("schedule mutations retain intent and Undo restores the original date", async ({page}) => {
  1174 |   await page.setViewportSize({width:1440,height:1000});
  1175 |   await page.goto('/desk.html');
  1176 |   await page.getByRole('button',{name:'Next week',exact:true}).click();
  1177 |   await page.locator('.week-board .session-tile.movable').first().click();
  1178 |   const original = await page.getByLabel('Run on',{exact:true}).inputValue();
  1179 |   const next = new Date(`${original}T12:00:00`); next.setDate(next.getDate()+1);
  1180 |   const moved = `${next.getFullYear()}-${String(next.getMonth()+1).padStart(2,'0')}-${String(next.getDate()).padStart(2,'0')}`;
  1181 |   await page.getByLabel('Run on',{exact:true}).fill(moved);
  1182 |   await page.getByRole('button',{name:'Move session',exact:true}).click();
  1183 |   await expect(page.getByRole('status')).toContainText('Moved to');
  1184 |   await page.getByRole('button',{name:'Undo',exact:true}).click();
  1185 |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='move_workout').map(call=>call.args.date))).toEqual([moved,original]);
  1186 |   await page.locator('.week-board .session-tile.movable').first().click();
  1187 |   await page.getByRole('button',{name:'Skip session',exact:true}).click();
  1188 |   await expect(page.getByRole('status')).toContainText('Session skipped');
  1189 |   await page.getByRole('button',{name:'Undo',exact:true}).click();
  1190 |   await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='set_workout_skipped').map(call=>call.args.skipped))).toEqual([true,false]);
  1191 | });
  1192 | 
  1193 | test("week summary shows run mileage against the plan's planned mileage", async ({page}) => {
  1194 |   await page.setViewportSize({width:1440,height:1000});
  1195 |   await page.goto('/desk.html');
  1196 |   const strip = page.locator('.schedule-summary');
  1197 |   await expect(strip).toBeVisible();
  1198 |   // The km reading carries the plan's denominator: "X km / Y km done", so a
  1199 |   // week at 38 run of 30 planned reads as 38/30 rather than a bare 38.
  1200 |   await expect(strip.locator('span').nth(1)).toContainText("/");
  1201 |   await expect(strip.locator('span').nth(1)).toContainText("done");
  1202 | });
  1203 | 
  1204 | test("reset week undoes a scheduled benchmark and a moved session", async ({page}) => {
  1205 |   await page.setViewportSize({width:1440,height:1000});
  1206 |   await page.goto('/desk.html');
  1207 |   await page.getByRole('button',{name:'Next week',exact:true}).click();
  1208 |   // Schedule a benchmark over a planned session, the way the user did.
  1209 |   await page.getByRole('button',{name:'Schedule race or test',exact:true}).click();
  1210 |   const dialog = page.getByRole('dialog',{name:'Schedule race or test',exact:true});
  1211 |   await dialog.getByRole('button',{name:'Schedule benchmark',exact:true}).click();
  1212 |   await expect(dialog).toHaveCount(0);
  1213 |   await expect(page.locator('.week-board')).toContainText('Benchmark: parkrun');
  1214 |   // The reset offer appears on the summary strip for the week horizon.
  1215 |   await page.getByRole('button',{name:'Reset week',exact:true}).click();
  1216 |   const reset = page.getByRole('dialog',{name:'Reset this week',exact:true});
  1217 |   await expect(reset).toBeVisible();
  1218 |   await expect(reset.locator('.reset-list li').filter({hasText:'Benchmark'})).toBeVisible();
  1219 |   await reset.getByRole('button',{name:/^Reset \d+$/}).click();
  1220 |   await expect(reset).toHaveCount(0);
  1221 |   await expect(page.getByRole('status')).toContainText('back on the plan');
  1222 |   await expect(page.locator('.week-board')).not.toContainText('Benchmark: parkrun');
  1223 |   expect(await page.evaluate(() => window.__deskCalls.some(call=>call.command==='reset_week'))).toBe(true);
  1224 | });
  1225 | 
  1226 | test("matching uses the chosen recorded activity", async ({page}) => {
  1227 |   await page.setViewportSize({width:1440,height:1000});
  1228 |   await page.goto('/desk.html?unmatched=1');
  1229 |   await page.locator('.week-board .session-tile.movable').first().click();
  1230 |   await page.getByRole('combobox',{name:'Recorded activity'}).selectOption('99');
  1231 |   await page.getByRole('button',{name:'Match run',exact:true}).click();
  1232 |   await expect(page.getByRole('status')).toContainText('Recorded run matched');
  1233 |   expect(await page.evaluate(() => window.__deskCalls.find(call=>call.command==='match_calendar_activity').args.activityId)).toBe(99);
  1234 | });
  1235 | 
  1236 | for (const action of ['Apply','Dismiss']) {
  1237 |   test(`${action} proposal requires review and preserves a failed decision`, async ({page}) => {
  1238 |     await page.goto('/desk.html?proposal=1#health/journal');
  1239 |     await page.getByRole('button',{name:'Review changes',exact:true}).click();
  1240 |     await expect(page.getByRole('dialog')).toContainText('Easy run');
  1241 |     expect(await page.evaluate(() => window.__deskCalls.some(call=>['apply_proposal','dismiss_proposal'].includes(call.command)))).toBe(false);
  1242 |     if (action==='Dismiss') await page.getByRole('textbox',{name:'Reason for dismissing (optional)'}).fill('Prefer to review with my clinician.');
  1243 |     await page.evaluate(command => {window.__deskFail=command;},`${action.toLowerCase()}_proposal`);
  1244 |     await page.getByRole('button',{name:`${action} changes`,exact:true}).click();
  1245 |     await expect(page.getByRole('dialog').getByRole('alert')).toContainText('Unable to save');
  1246 |     await page.evaluate(() => {window.__deskFail='';});
  1247 |     await page.getByRole('button',{name:`${action} changes`,exact:true}).click();
  1248 |     await expect(page.getByRole('dialog')).toHaveCount(0);
  1249 |     await expect(page.getByRole('button',{name:'Review changes',exact:true})).toHaveCount(0);
  1250 |   });
  1251 | }
  1252 | 
  1253 | for (const width of [390,1440]) {
  1254 |   test(`recorded route map renders geographic pixels and responds to zoom at ${width}px`, async ({page}) => {
  1255 |     await page.setViewportSize({width,height:1000});
```