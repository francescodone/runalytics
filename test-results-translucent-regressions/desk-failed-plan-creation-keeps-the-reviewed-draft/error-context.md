# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> failed plan creation keeps the reviewed draft
- Location: tests/desk.spec.mjs:1512:1

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
  1505 |   await page.getByLabel("Start date", { exact: true }).fill(targetDate);
  1506 |   await page.getByRole("button", { name: "Continue", exact: true }).click();
  1507 |   await expect(page.getByLabel("Start date", { exact: true })).toBeVisible();
  1508 |   expect(await page.getByLabel("Race date", { exact: true }).evaluate(input => input.validity.rangeUnderflow)).toBe(true);
  1509 |   expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "create_plan"))).toEqual([]);
  1510 | });
  1511 | 
  1512 | test("failed plan creation keeps the reviewed draft", async ({ page }) => {
> 1513 |   await page.goto("/desk.html?noPlan=1");
       |              ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html?noPlan=1
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
  1562 |       };
  1563 |       await sampleToggle();
  1564 |       await toggle.focus();
  1565 |       await page.keyboard.press("Enter");
  1566 |       await expect(toggle).toHaveAttribute("aria-expanded", "true");
  1567 |       await expect.poll(() => page.evaluate(() => window.__filterHeights)).not.toBeNull();
  1568 |       const heights = await page.evaluate(() => window.__filterHeights);
  1569 |       const expandedHeight = heights.at(-1);
  1570 |       expect(expandedHeight).toBeGreaterThan(100);
  1571 |       expect(heights.some(height => height > 1 && height < expandedHeight - 1)).toBe(reducedMotion === "no-preference");
  1572 |       await page.keyboard.press("Tab");
  1573 |       await expect(page.getByRole("searchbox", { name: "Search activities" })).toBeFocused();
  1574 |       await page.getByRole("searchbox", { name: "Search activities" }).fill("Run");
  1575 |       await sampleToggle();
  1576 |       await toggle.click();
  1577 |       await expect(filters).toHaveAttribute("inert", "");
  1578 |       await expect(filters).toHaveAttribute("aria-hidden", "true");
  1579 |       await expect.poll(() => page.evaluate(() => window.__filterHeights)).not.toBeNull();
  1580 |       const closingHeights = await page.evaluate(() => window.__filterHeights);
  1581 |       expect(closingHeights.at(-1)).toBe(0);
  1582 |       expect(closingHeights.some(height => height > 1 && height < expandedHeight - 1)).toBe(reducedMotion === "no-preference");
  1583 |       await expect(filters).toBeHidden();
  1584 |       await toggle.press("Enter");
  1585 |       await toggle.press("Enter");
  1586 |       await toggle.press("Enter");
  1587 |       await expect(toggle).toHaveAttribute("aria-expanded", "true");
  1588 |       await expect(filters).not.toHaveAttribute("inert", "");
  1589 |       await expect(page.getByRole("searchbox", { name: "Search activities" })).toHaveValue("Run");
  1590 |       await expect.poll(() => filters.evaluate(element => getComputedStyle(element).height)).not.toBe("0px");
  1591 |       expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  1592 |     });
  1593 |   }
  1594 | }
  1595 | 
  1596 | test("Activities list hides filters until revealed and opens recorded splits", async ({ page }) => {
  1597 |   await page.goto("/desk.html#journal");
  1598 |   await expect(page.locator(".activity-row")).toHaveCount(9);
  1599 |   await expect(page.locator("#activity-filters")).toBeHidden();
  1600 |   await page.getByRole("button", { name: "Filters", exact: true }).click();
  1601 |   await expect(page.getByRole("combobox", { name: "Sport", exact: true })).toBeVisible();
  1602 |   await page.getByRole("combobox", { name: "Sport", exact: true }).selectOption("Run");
  1603 |   await expect(page.locator(".activity-row")).toHaveCount(6);
  1604 |   await page.getByLabel("Search activities").fill("not-a-run");
  1605 |   await expect(page.getByRole("heading", { name: "No matching activities" })).toBeVisible();
  1606 |   await page.getByRole("button", { name: "Reset filters" }).click();
  1607 |   await expect(page.locator(".activity-row")).toHaveCount(9);
  1608 |   await page.locator(".activity-row").first().click();
  1609 |   await expect(page.getByRole("complementary", { name: "Activity review" })).toBeVisible();
  1610 |   await page.getByRole("tab", { name: "Splits", exact: true }).click();
  1611 |   await expect(page.getByRole("heading", { name: "Kilometre splits" })).toBeVisible();
  1612 |   // Each kilometre wears a horizontal bar scaled against the fastest kilometre
  1613 |   // of the run, so the fastest row is full width and every slower row shorter.
```