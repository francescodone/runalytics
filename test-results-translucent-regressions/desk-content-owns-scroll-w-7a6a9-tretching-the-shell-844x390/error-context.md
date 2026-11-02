# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> content owns scroll without stretching the shell 844x390
- Location: tests/desk.spec.mjs:1308:3

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#health
Call log:
  - navigating to "http://127.0.0.1:1500/desk.html#health", waiting until "load"

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
  1256 |     await page.goto('/desk.html#journal/1');
  1257 |     await expect(page.getByRole('region',{name:'Recorded GPS route'})).toBeVisible();
  1258 |     const pixels = () => page.locator('.maplibregl-canvas').evaluate(canvas => {
  1259 |       const context=canvas.getContext('webgl2');
  1260 |       const bytes=new Uint8Array(context.drawingBufferWidth*context.drawingBufferHeight*4);
  1261 |       context.readPixels(0,0,context.drawingBufferWidth,context.drawingBufferHeight,context.RGBA,context.UNSIGNED_BYTE,bytes);
  1262 |       const probe = document.createElement("canvas");
  1263 |       probe.width = probe.height = 1;
  1264 |       const paint = probe.getContext("2d");
  1265 |       paint.fillStyle = getComputedStyle(document.documentElement).getPropertyValue("--desk-action").trim();
  1266 |       paint.fillRect(0, 0, 1, 1);
  1267 |       const accent = paint.getImageData(0, 0, 1, 1).data;
  1268 |       let colored=0, fingerprint=0;
  1269 |       for(let index=0;index<bytes.length;index+=64) { if(bytes[index+3]>0 && Math.abs(bytes[index] - accent[0]) < 30 && Math.abs(bytes[index+1] - accent[1]) < 30 && Math.abs(bytes[index+2] - accent[2]) < 30) colored++; fingerprint=(fingerprint+bytes[index]*(index+1))%1000000007; }
  1270 |       return {colored,fingerprint};
  1271 |     });
  1272 |     await expect.poll(async () => (await pixels()).colored).toBeGreaterThan(10);
  1273 |     const before=await pixels();
  1274 |     await page.getByRole('button',{name:'Zoom in',exact:true}).click();
  1275 |     await expect.poll(async () => (await pixels()).fingerprint).not.toBe(before.fingerprint);
  1276 |     await page.getByRole('button',{name:'Fit route',exact:true}).click();
  1277 |     expect(await page.evaluate(() => document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  1278 |     await page.screenshot({path:`test-results/desk-map-${width}.png`,fullPage:true});
  1279 |   });
  1280 | }
  1281 | 
  1282 | for (const width of [390, 1440]) {
  1283 |   for (const reducedMotion of ["no-preference", "reduce"]) {
  1284 |     test(`plan centers the current week on every visit ${width}px ${reducedMotion}`, async ({ page }) => {
  1285 |       await page.setViewportSize({ width, height: 844 });
  1286 |       await page.emulateMedia({ reducedMotion });
  1287 |       await page.goto("/desk.html#training");
  1288 |       for (let visit = 0; visit < 2; visit++) {
  1289 |         await page.getByRole("link", { name: "Plan", exact: true }).click();
  1290 |         const list = page.getByRole("region", { name: "Plan weeks", exact: true });
  1291 |         const current = list.locator(".is-current");
  1292 |         await expect(current).toBeInViewport({ ratio: .9 });
  1293 |         await expect.poll(() => list.evaluate(element => {
  1294 |           const row = element.querySelector(".is-current");
  1295 |           const target = Math.min(element.scrollHeight - element.clientHeight, Math.max(0, row.offsetTop - (element.clientHeight - row.offsetHeight) / 2));
  1296 |           return Math.abs(element.scrollTop - target);
  1297 |         })).toBeLessThan(2);
  1298 |         expect(await page.evaluate(() => document.documentElement.scrollHeight <= innerHeight && scrollY === 0)).toBe(true);
  1299 |         await list.evaluate(element => { element.scrollTop = element.scrollHeight; });
  1300 |         await page.getByRole("link", { name: "Week", exact: true }).click();
  1301 |         await expect(list).toHaveCount(0);
  1302 |       }
  1303 |     });
  1304 |   }
  1305 | }
  1306 | 
  1307 | for (const viewport of [{ width: 320, height: 640 }, { width: 844, height: 390 }, { width: 1440, height: 900 }]) {
  1308 |   test(`content owns scroll without stretching the shell ${viewport.width}x${viewport.height}`, async ({ page }) => {
  1309 |     await page.setViewportSize(viewport);
> 1310 |     await page.goto("/desk.html#health");
       |                ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#health
  1311 |     const workspace = page.locator("#workspace");
  1312 |     await page.getByRole("button", { name: "Show health details", exact: true }).click();
  1313 |     const navigation = page.getByRole("navigation", { name: "Workspaces" });
  1314 |     const before = await navigation.boundingBox();
  1315 |     await workspace.hover({ position: { x: 20, y: 100 } });
  1316 |     await page.mouse.wheel(0, 800);
  1317 |     await expect.poll(() => workspace.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
  1318 |     expect(await navigation.boundingBox()).toEqual(before);
  1319 |     expect(await page.evaluate(() => ({ height: document.documentElement.scrollHeight, width: document.documentElement.scrollWidth, scroll: scrollY }))).toEqual({ height: viewport.height, width: viewport.width, scroll: 0 });
  1320 |     await page.goto("/desk.html#journal");
  1321 |     const library = page.getByRole("region", { name: "Activity history" });
  1322 |     await expect(page.locator(".activity-row")).toHaveCount(9);
  1323 |     await expect.poll(() => library.evaluate(element => element.scrollHeight > element.clientHeight)).toBe(true);
  1324 |     expect(await library.evaluate(element => element.clientHeight <= document.querySelector("#workspace").clientHeight)).toBe(true);
  1325 |     await library.focus();
  1326 |     await page.keyboard.press("End");
  1327 |     await expect.poll(() => library.evaluate(element => Math.abs(element.scrollHeight - element.clientHeight - element.scrollTop))).toBeLessThan(2);
  1328 |     expect(await page.evaluate(() => scrollY)).toBe(0);
  1329 |     await page.goto("/desk.html#training");
  1330 |     if (viewport.width <= 800) await page.locator(".mobile-date-ribbon button").first().click();
  1331 |     const tiles = page.locator(viewport.width <= 800 ? ".day-itinerary .session-tile" : ".week-board .session-tile");
  1332 |     await expect(tiles.first()).toBeVisible();
  1333 |     expect(await tiles.evaluateAll(elements => elements.every(element => {
  1334 |       const styles = getComputedStyle(element);
  1335 |       const children = [...element.children].filter(child => getComputedStyle(child).position !== "absolute");
  1336 |       const content = children.reduce((height, child) => height + child.getBoundingClientRect().height, 0);
  1337 |       const needed = content + Math.max(0, children.length - 1) * parseFloat(styles.rowGap) + parseFloat(styles.paddingTop) + parseFloat(styles.paddingBottom);
  1338 |       return element.clientHeight <= Math.max(44, needed) + 2;
  1339 |     }))).toBe(true);
  1340 |   });
  1341 | }
  1342 | 
  1343 | test("floating session details blur and lock their background without changing inline activity review", async ({ page }) => {
  1344 |   await page.setViewportSize({ width: 1440, height: 900 });
  1345 |   await page.emulateMedia({ reducedMotion: "no-preference" });
  1346 |   await page.goto("/desk.html#training");
  1347 |   const opener = page.locator(".week-board .session-tile").first();
  1348 |   await opener.click();
  1349 |   const inspector = page.getByRole("dialog", { name: "Session", exact: true });
  1350 |   const backdrop = page.locator(".inspector-backdrop");
  1351 |   await expect(inspector).toBeVisible();
  1352 |   await expect(backdrop).toHaveCSS("backdrop-filter", "blur(12px)");
  1353 |   await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "hidden");
  1354 |   expect(await page.locator(".desk-appbar").evaluate(element => element.inert)).toBe(true);
  1355 |   await inspector.getByRole("button", { name: "Back", exact: true }).focus();
  1356 |   await page.keyboard.press("Shift+Tab");
  1357 |   expect(await inspector.evaluate(element => element.contains(document.activeElement))).toBe(true);
  1358 |   await backdrop.click({ position: { x: 20, y: 200 } });
  1359 |   await expect(inspector).toHaveCount(0);
  1360 |   await expect(backdrop).toHaveCount(0);
  1361 |   await expect(opener).toBeFocused();
  1362 |   await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "auto");
  1363 |   expect(await page.locator(".desk-appbar").evaluate(element => element.inert)).toBe(false);
  1364 |   await page.goto("/desk.html#journal/1");
  1365 |   await expect(page.getByRole("complementary", { name: "Activity review" })).toBeVisible();
  1366 |   await expect(backdrop).toHaveCount(0);
  1367 |   await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "hidden");
  1368 |   await expect(page.getByRole("complementary", { name: "Activity review" })).toHaveCSS("overflow-y", "auto");
  1369 | });
  1370 | 
  1371 | test("nested information dialogs retain review drafts and outer scroll ownership", async ({ page }) => {
  1372 |   await page.goto("/desk.html?pacingProposal=1#health/pace");
  1373 |   await page.getByRole("button", { name: "Review changes", exact: true }).click();
  1374 |   const review = page.getByRole("dialog", { name: "Review plan changes", exact: true });
  1375 |   const reason = review.getByRole("textbox", { name: "Reason for dismissing (optional)" });
  1376 |   await reason.fill("Review after my next workout");
  1377 |   const basis = review.getByRole("button", { name: "Estimate basis", exact: true });
  1378 |   await basis.click();
  1379 |   await expect(page.locator("dialog[open]")).toHaveCount(2);
  1380 |   await page.keyboard.press("Escape");
  1381 |   await expect(page.getByRole("dialog", { name: "Estimate basis", exact: true })).toHaveCount(0);
  1382 |   await expect(reason).toHaveValue("Review after my next workout");
  1383 |   await expect(basis).toBeFocused();
  1384 |   await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "hidden");
  1385 |   await review.getByRole("button", { name: "Dismiss changes", exact: true }).click();
  1386 |   await expect(review).toHaveCount(0);
  1387 |   await page.getByRole("button", { name: "Adjustment history", exact: true }).click();
  1388 |   const history = page.getByRole("dialog", { name: "Adjustment history", exact: true });
  1389 |   const entry = history.getByRole("button", { name: /Sharpen quality pace/ });
  1390 |   await entry.click();
  1391 |   await expect(review).toBeVisible();
  1392 |   await page.keyboard.press("Escape");
  1393 |   await expect(review).toHaveCount(0);
  1394 |   await expect(entry).toBeFocused();
  1395 |   await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "hidden");
  1396 |   await page.keyboard.press("Escape");
  1397 |   await expect(history).toHaveCount(0);
  1398 |   await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "auto");
  1399 | });
  1400 | 
  1401 | test("independent shell loads only new presentation", async ({ page }) => {
  1402 |   const loaded = [];
  1403 |   page.on("request", request => loaded.push(request.url()));
  1404 |   await page.goto("/desk.html");
  1405 |   await expect(page.getByRole("heading", { name: "Training", exact: true })).toBeVisible();
  1406 |   await expect(page.getByRole("navigation", { name: "Workspaces" }).getByRole("link")).toHaveCount(3);
  1407 |   await expect(page.locator(".day-itinerary")).toBeVisible();
  1408 |   expect(loaded.filter(url => /\/(app|redesign)\.css|\/src\/(dashboard|plan|calendar|achievements|injury|settings|support)\.ts|\/lib\/(navigation|overlay|workout-detail)\.ts/.test(url))).toEqual([]);
  1409 | });
  1410 | 
```