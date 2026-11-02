# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> disclosure health details and conversation drafts stay accessible no-preference 1440px
- Location: tests/desk.spec.mjs:1595:5

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
  1538 |     test(`disclosure filters retain values and reverse cleanly ${reducedMotion} ${width}px`, async ({ page }, testInfo) => {
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
  1572 |       await page.screenshot({ path: testInfo.outputPath("filters-open.png") });
  1573 |       await page.keyboard.press("Tab");
  1574 |       await expect(page.getByRole("searchbox", { name: "Search activities" })).toBeFocused();
  1575 |       await page.getByRole("searchbox", { name: "Search activities" }).fill("Run");
  1576 |       await sampleToggle();
  1577 |       await toggle.click();
  1578 |       await expect(filters).toHaveAttribute("inert", "");
  1579 |       await expect(filters).toHaveAttribute("aria-hidden", "true");
  1580 |       await expect.poll(() => page.evaluate(() => window.__filterHeights)).not.toBeNull();
  1581 |       const closingHeights = await page.evaluate(() => window.__filterHeights);
  1582 |       expect(closingHeights.at(-1)).toBe(0);
  1583 |       expect(closingHeights.some(height => height > 1 && height < expandedHeight - 1)).toBe(reducedMotion === "no-preference");
  1584 |       await expect(filters).toBeHidden();
  1585 |       await toggle.press("Enter");
  1586 |       await toggle.press("Enter");
  1587 |       await toggle.press("Enter");
  1588 |       await expect(toggle).toHaveAttribute("aria-expanded", "true");
  1589 |       await expect(filters).not.toHaveAttribute("inert", "");
  1590 |       await expect(page.getByRole("searchbox", { name: "Search activities" })).toHaveValue("Run");
  1591 |       await expect.poll(() => filters.evaluate(element => getComputedStyle(element).height)).not.toBe("0px");
  1592 |       expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  1593 |     });
  1594 | 
  1595 |     test(`disclosure health details and conversation drafts stay accessible ${reducedMotion} ${width}px`, async ({ page }, testInfo) => {
  1596 |       await page.setViewportSize({ width, height: 900 });
  1597 |       await page.emulateMedia({ reducedMotion });
> 1598 |       await page.goto("/desk.html#health");
       |                  ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#health
  1599 |       const toggle = page.locator(".health-detail-toggle");
  1600 |       const details = page.locator("#health-details");
  1601 |       await expect(details).toBeHidden();
  1602 |       await expect(details).toHaveAttribute("inert", "");
  1603 |       await toggle.click();
  1604 |       await expect(page.getByRole("heading", { name: "Risk contributors" })).toBeVisible();
  1605 |       await expect.poll(() => details.evaluate(element => element.getAnimations({ subtree: true }).filter(animation => animation.playState === "running").length)).toBe(0);
  1606 |       await expect(toggle.locator(".disclosure-indicator")).toHaveCSS("rotate", "180deg");
  1607 |       await page.screenshot({ path: testInfo.outputPath("health-details-open.png") });
  1608 |       await details.getByRole("button").first().focus();
  1609 |       await toggle.evaluate(button => button.click());
  1610 |       await expect(toggle).toBeFocused();
  1611 |       await expect(details).toHaveAttribute("inert", "");
  1612 |       await expect(details).toBeHidden();
  1613 |       await page.keyboard.press("Tab");
  1614 |       expect(await details.evaluate(element => element.contains(document.activeElement))).toBe(false);
  1615 |       await page.getByRole("link", { name: "Pain journal", exact: true }).click();
  1616 |       await page.getByRole("button", { name: "Log pain", exact: true }).first().click();
  1617 |       await page.getByRole("combobox", { name: "Body region", exact: true }).selectOption("knee.left");
  1618 |       await page.getByRole("button", { name: "Save pain entry" }).click();
  1619 |       await expect(page.getByRole("dialog")).toHaveCount(0);
  1620 |       const discuss = page.getByRole("button", { name: "Discuss episode" });
  1621 |       await expect(discuss).toBeVisible();
  1622 |       await expect(page.locator(".episode-chat")).toHaveCount(0);
  1623 |       await discuss.click();
  1624 |       const question = page.getByRole("textbox", { name: "Your question" });
  1625 |       await question.fill("Keep this unfinished question");
  1626 |       await page.getByRole("checkbox", { name: /I agree to send/ }).check();
  1627 |       await discuss.click();
  1628 |       await expect(page.getByRole("region", { name: "Episode conversation" })).toHaveCount(0);
  1629 |       await page.keyboard.press("Tab");
  1630 |       await expect(page.getByRole("button", { name: "Delete", exact: true })).toBeFocused();
  1631 |       await discuss.click();
  1632 |       await expect(question).toHaveValue("Keep this unfinished question");
  1633 |       await expect(page.getByRole("checkbox", { name: /I agree to send/ })).toBeChecked();
  1634 |       await expect(page.getByRole("button", { name: "Send question" })).toBeEnabled();
  1635 |       expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "ask_assistant").length)).toBe(0);
  1636 |       expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  1637 |     });
  1638 | 
  1639 |     test(`view changes reveal filtered results without transforming their container ${reducedMotion} ${width}px`, async ({ page }) => {
  1640 |       await page.setViewportSize({ width, height: 900 });
  1641 |       await page.emulateMedia({ reducedMotion });
  1642 |       await page.goto("/desk.html#journal");
  1643 |       await expect(page.locator(".activity-row")).toHaveCount(9);
  1644 |       await page.getByRole("button", { name: "Filters", exact: true }).click();
  1645 |       await page.locator(".activity-library").evaluate(library => {
  1646 |         window.__resultArrival = [];
  1647 |         const observer = new MutationObserver(records => {
  1648 |           for (const record of records) {
  1649 |             for (const node of record.addedNodes) {
  1650 |               if (!(node instanceof HTMLElement) || !node.matches(".motion-change")) continue;
  1651 |               const sample = () => {
  1652 |                 const style = getComputedStyle(node);
  1653 |                 window.__resultArrival.push({ opacity: Number(style.opacity), transform: style.transform, filter: style.filter });
  1654 |                 if (Number(style.opacity) < 1) requestAnimationFrame(sample);
  1655 |               };
  1656 |               sample();
  1657 |               observer.disconnect();
  1658 |             }
  1659 |           }
  1660 |         });
  1661 |         observer.observe(library, { childList: true });
  1662 |       });
  1663 |       await page.getByRole("combobox", { name: "Sport", exact: true }).selectOption("Run");
  1664 |       await expect(page.locator(".activity-row")).toHaveCount(6);
  1665 |       await expect.poll(() => page.evaluate(() => window.__resultArrival.at(-1)?.opacity)).toBe(1);
  1666 |       const frames = await page.evaluate(() => window.__resultArrival);
  1667 |       expect(frames.some(frame => frame.opacity > 0 && frame.opacity < 1)).toBe(reducedMotion === "no-preference");
  1668 |       expect(frames.every(frame => frame.transform === "none" && frame.filter === "none")).toBe(true);
  1669 |       await page.getByRole("button", { name: "Reset filters" }).click();
  1670 |       await page.getByRole("combobox", { name: "Source", exact: true }).selectOption("garmin");
  1671 |       await page.getByRole("button", { name: "Reset filters" }).click();
  1672 |       await expect(page.locator(".activity-row")).toHaveCount(9);
  1673 |       await expect(page.locator(".activity-library > .motion-change")).toHaveCount(1);
  1674 |       await expect(page.locator(".activity-library > .motion-change")).not.toHaveAttribute("inert", "");
  1675 |     });
  1676 |   }
  1677 | }
  1678 | 
  1679 | test("Activities list hides filters until revealed and opens recorded splits", async ({ page }) => {
  1680 |   await page.goto("/desk.html#journal");
  1681 |   await expect(page.locator(".activity-row")).toHaveCount(9);
  1682 |   await expect(page.locator("#activity-filters")).toBeHidden();
  1683 |   await page.getByRole("button", { name: "Filters", exact: true }).click();
  1684 |   await expect(page.getByRole("combobox", { name: "Sport", exact: true })).toBeVisible();
  1685 |   await page.getByRole("combobox", { name: "Sport", exact: true }).selectOption("Run");
  1686 |   await expect(page.locator(".activity-row")).toHaveCount(6);
  1687 |   await page.getByLabel("Search activities").fill("not-a-run");
  1688 |   await expect(page.getByRole("heading", { name: "No matching activities" })).toBeVisible();
  1689 |   await page.getByRole("button", { name: "Reset filters" }).click();
  1690 |   await expect(page.locator(".activity-row")).toHaveCount(9);
  1691 |   await page.locator(".activity-row").first().click();
  1692 |   await expect(page.getByRole("complementary", { name: "Activity review" })).toBeVisible();
  1693 |   await page.getByRole("tab", { name: "Splits", exact: true }).click();
  1694 |   await expect(page.getByRole("heading", { name: "Kilometre splits" })).toBeVisible();
  1695 |   // Each kilometre wears a horizontal bar scaled against the fastest kilometre
  1696 |   // of the run, so the fastest row is full width and every slower row shorter.
  1697 |   const bars = page.getByRole("region", { name: "Kilometre splits" }).locator(".split-bar > span");
  1698 |   expect(await bars.count()).toBeGreaterThan(1);
```