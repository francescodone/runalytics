# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> independent shell loads only new presentation
- Location: tests/desk.spec.mjs:1401:1

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html
Call log:
  - navigating to "http://127.0.0.1:1500/desk.html", waiting until "load"

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
  1304 |   }
  1305 | }
  1306 | 
  1307 | for (const viewport of [{ width: 320, height: 640 }, { width: 844, height: 390 }, { width: 1440, height: 900 }]) {
  1308 |   test(`content owns scroll without stretching the shell ${viewport.width}x${viewport.height}`, async ({ page }) => {
  1309 |     await page.setViewportSize(viewport);
  1310 |     await page.goto("/desk.html#health");
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
> 1404 |   await page.goto("/desk.html");
       |              ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html
  1405 |   await expect(page.getByRole("heading", { name: "Training", exact: true })).toBeVisible();
  1406 |   await expect(page.getByRole("navigation", { name: "Workspaces" }).getByRole("link")).toHaveCount(3);
  1407 |   await expect(page.locator(".day-itinerary")).toBeVisible();
  1408 |   expect(loaded.filter(url => /\/(app|redesign)\.css|\/src\/(dashboard|plan|calendar|achievements|injury|settings|support)\.ts|\/lib\/(navigation|overlay|workout-detail)\.ts/.test(url))).toEqual([]);
  1409 | });
  1410 | 
  1411 | test("mobile date selection opens a full-screen session and returns to its opener", async ({ page }) => {
  1412 |   await page.setViewportSize({width:390, height:844});
  1413 |   await page.emulateMedia({reducedMotion:"no-preference"});
  1414 |   await page.goto("/desk.html#health");
  1415 |   await page.getByRole("navigation", {name:"Workspaces"}).getByRole("link", {name:"Training"}).click();
  1416 |   await page.locator(".mobile-date-ribbon button").first().click();
  1417 |   const session = page.locator(".day-itinerary .session-tile").first();
  1418 |   await session.click();
  1419 |   await expect(page.getByRole("dialog", { name: "Session", exact: true })).toBeVisible();
  1420 |   await expect.poll(async () => {
  1421 |     const bounds = await page.locator(".desk-inspector").boundingBox();
  1422 |     return bounds && Object.fromEntries(Object.entries(bounds).map(([key,value]) => [key,Math.round(value)]));
  1423 |   }).toEqual({x:0,y:0,width:390,height:844});
  1424 |   await expect(page.locator(".prescription")).toContainText("Repeat 4 times:");
  1425 |   await page.getByRole("button", { name: "Back", exact: true }).click();
  1426 |   await expect(session).toBeFocused();
  1427 | });
  1428 | 
  1429 | for (const width of [390, 1440]) {
  1430 |   test(`benchmark scheduling submits parkrun and custom test contracts at ${width}px`, async ({ page }) => {
  1431 |     await page.setViewportSize({ width, height: 900 });
  1432 |     await page.goto("/desk.html#training/week");
  1433 |     const open = page.getByRole("button", { name: "Schedule race or test", exact: true });
  1434 |     await open.click();
  1435 |     const dialog = page.getByRole("dialog", { name: "Schedule race or test", exact: true });
  1436 |     await expect(dialog.getByLabel("Distance (km)")).toHaveValue("5");
  1437 |     await expect(dialog.getByLabel("Distance (km)")).toBeDisabled();
  1438 |     const date = await dialog.getByLabel("Date", { exact: true }).inputValue();
  1439 |     expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  1440 |     await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
  1441 |     await expect(dialog).toHaveCount(0);
  1442 |     expect(await page.evaluate(() => window.__deskCalls.find(call => call.command === "schedule_benchmark").args)).toEqual({ date, name: "parkrun", distanceM: 5000 });
  1443 |     await expect(page.locator(".day-itinerary")).toContainText("Benchmark: parkrun");
  1444 |     await open.click();
  1445 |     await dialog.getByRole("combobox", { name: "Event", exact: true }).selectOption("trial");
  1446 |     await dialog.getByLabel("Event name", { exact: true }).fill("3 km fitness test");
  1447 |     await dialog.getByLabel("Distance (km)").fill("3");
  1448 |     await page.evaluate(() => { window.__deskFail = "schedule_benchmark"; });
  1449 |     await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
  1450 |     await expect(dialog.getByRole("alert")).toContainText("Unable to save right now");
  1451 |     await expect(dialog.getByLabel("Event name", { exact: true })).toHaveValue("3 km fitness test");
  1452 |     await page.screenshot({ path: `test-results/benchmark-${width}.png`, animations: "disabled" });
  1453 |     await page.evaluate(() => { window.__deskFail = ""; });
  1454 |     await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
  1455 |     await expect(dialog).toHaveCount(0);
  1456 |     expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "schedule_benchmark").pop().args)).toMatchObject({ name: "3 km fitness test", distanceM: 3000 });
  1457 |   });
  1458 | }
  1459 | 
  1460 | test("plan creation submits the established contract", async ({ page }) => {
  1461 |   await page.goto("/desk.html?noPlan=1");
  1462 |   await page.getByRole("button", { name: "Create plan", exact: true }).click();
  1463 |   const startDate = await page.getByLabel("Start date", { exact: true }).inputValue();
  1464 |   const selectedStart = new Date(startDate + "T12:00:00");
  1465 |   selectedStart.setDate(selectedStart.getDate() + 7);
  1466 |   const selectedDate = `${selectedStart.getFullYear()}-${String(selectedStart.getMonth() + 1).padStart(2, "0")}-${String(selectedStart.getDate()).padStart(2, "0")}`;
  1467 |   await page.getByLabel("Start date", { exact: true }).fill(selectedDate);
  1468 |   await page.getByLabel("Plan name (optional)").fill("Spring half build");
  1469 |   await page.getByRole("button", { name: "Continue", exact: true }).click();
  1470 |   await expect(page.getByLabel("Threshold pace (min/km)")).toHaveValue("4:40");
  1471 |   await page.getByLabel("Current weekly mileage").fill("40");
  1472 |   await page.getByRole("button", { name: "Continue", exact: true }).click();
  1473 |   await page.getByRole("button", { name: "Build plan", exact: true }).click();
  1474 |   await expect(page.getByRole("dialog")).toHaveCount(0);
  1475 |   const request = await page.evaluate(() => window.__deskCalls.find(call => call.command === "create_plan").args.request);
  1476 |   expect(request).toMatchObject({kind:"race",distance:"21K",startDate:selectedDate,thresholdSecKm:280,weeklyFrequency:4,currentWeeklyKm:40,name:"Spring half build"});
  1477 |   await expect(page.locator(".plan-basis")).toContainText("40.0 km/week");
  1478 | });
  1479 | 
  1480 | for (const width of [390, 1440]) {
  1481 |   test(`plan start date anchors maintenance duration at ${width}px`, async ({ page }) => {
  1482 |     await page.setViewportSize({ width, height: 1000 });
  1483 |     await page.goto("/desk.html?noPlan=1");
  1484 |     await page.getByRole("button", { name: "Create plan", exact: true }).click();
  1485 |     await page.getByLabel("Stay consistent", { exact: true }).check();
  1486 |     await page.getByLabel("Start date", { exact: true }).fill("2030-03-06");
  1487 |     await page.getByRole("combobox", { name: "Block length", exact: true }).selectOption("16");
  1488 |     await expect(page.locator(".composer-summary")).toContainText("06/03/2030");
  1489 |     await expect(page.locator(".composer-summary")).toContainText("26/06/2030");
  1490 |     expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  1491 |     await page.screenshot({ path: `test-results/plan-start-date-${width}.png`, animations: "disabled" });
  1492 |     await page.getByRole("button", { name: "Continue", exact: true }).click();
  1493 |     await page.getByRole("button", { name: "Continue", exact: true }).click();
  1494 |     await page.getByRole("button", { name: "Build plan", exact: true }).click();
  1495 |     await expect(page.getByRole("dialog")).toHaveCount(0);
  1496 |     const request = await page.evaluate(() => window.__deskCalls.find(call => call.command === "create_plan").args.request);
  1497 |     expect(request).toMatchObject({ kind: "maintenance", startDate: "2030-03-06", targetDate: "2030-06-26" });
  1498 |   });
  1499 | }
  1500 | 
  1501 | test("plan start date requires a later end date", async ({ page }) => {
  1502 |   await page.goto("/desk.html?noPlan=1");
  1503 |   await page.getByRole("button", { name: "Create plan", exact: true }).click();
  1504 |   const targetDate = await page.getByLabel("Race date", { exact: true }).inputValue();
```