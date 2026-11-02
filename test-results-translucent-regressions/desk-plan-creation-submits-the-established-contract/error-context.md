# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> plan creation submits the established contract
- Location: tests/desk.spec.mjs:1460:1

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html?noPlan=1
Call log:
  - navigating to "http://127.0.0.1:1500/desk.html?noPlan=1", waiting until "load"

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
> 1461 |   await page.goto("/desk.html?noPlan=1");
       |              ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html?noPlan=1
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
  1505 |   await page.getByLabel("Start date", { exact: true }).fill(targetDate);
  1506 |   await page.getByRole("button", { name: "Continue", exact: true }).click();
  1507 |   await expect(page.getByLabel("Start date", { exact: true })).toBeVisible();
  1508 |   expect(await page.getByLabel("Race date", { exact: true }).evaluate(input => input.validity.rangeUnderflow)).toBe(true);
  1509 |   expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "create_plan"))).toEqual([]);
  1510 | });
  1511 | 
  1512 | test("failed plan creation keeps the reviewed draft", async ({ page }) => {
  1513 |   await page.goto("/desk.html?noPlan=1");
  1514 |   await page.getByRole("button", { name: "Create plan", exact: true }).click();
  1515 |   await page.getByRole("button", { name: "Continue", exact: true }).click();
  1516 |   await page.getByLabel("Current weekly mileage").fill("37");
  1517 |   await page.getByRole("button", { name: "Continue", exact: true }).click();
  1518 |   await page.evaluate(() => { window.__deskFail = "create_plan"; });
  1519 |   await page.getByRole("button", { name: "Build plan", exact: true }).click();
  1520 |   await expect(page.getByRole("alert")).toContainText("Unable to save right now");
  1521 |   await expect(page.locator(".composer-summary")).toContainText("37 km/week");
  1522 |   await expect(page.getByRole("button", { name: "Build plan", exact: true })).toBeEnabled();
  1523 | });
  1524 | 
  1525 | for (const width of [320, 390, 768, 1440]) {
  1526 |   test(`training layout and accessibility at ${width}px`, async ({ page }) => {
  1527 |     await page.setViewportSize({ width, height: 1000 });
  1528 |     await page.goto("/desk.html");
  1529 |     await expect(page.locator(width > 800 ? ".week-board" : ".day-itinerary")).toBeVisible();
  1530 |     expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  1531 |     expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
  1532 |     await page.screenshot({path:`test-results/desk-training-${width}.png`,fullPage:true});
  1533 |   });
  1534 | }
  1535 | 
  1536 | for (const width of [390, 1440]) {
  1537 |   for (const reducedMotion of ["no-preference", "reduce"]) {
  1538 |     test(`disclosure filters retain values and reverse cleanly ${reducedMotion} ${width}px`, async ({ page }) => {
  1539 |       await page.setViewportSize({ width, height: 900 });
  1540 |       await page.emulateMedia({ reducedMotion });
  1541 |       await page.goto("/desk.html#journal");
  1542 |       const toggle = page.locator(".filters-toggle");
  1543 |       const filters = page.locator("#activity-filters");
  1544 |       await expect(page.locator(".activity-row")).toHaveCount(9);
  1545 |       await expect(filters).toHaveAttribute("inert", "");
  1546 |       await expect(filters).toBeHidden();
  1547 |       const sampleToggle = async () => {
  1548 |         await toggle.evaluate(button => {
  1549 |           window.__filterHeights = null;
  1550 |           button.addEventListener("click", () => {
  1551 |             const panel = document.getElementById("activity-filters");
  1552 |             const samples = [panel.getBoundingClientRect().height];
  1553 |             const deadline = performance.now() + 600;
  1554 |             const sample = () => {
  1555 |               samples.push(panel.getBoundingClientRect().height);
  1556 |               if (performance.now() < deadline) requestAnimationFrame(sample);
  1557 |               else window.__filterHeights = samples;
  1558 |             };
  1559 |             requestAnimationFrame(sample);
  1560 |           }, { once: true, capture: true });
  1561 |         });
```