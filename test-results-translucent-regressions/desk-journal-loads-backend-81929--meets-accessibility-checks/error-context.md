# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: desk.spec.mjs >> journal loads backend evidence and meets accessibility checks
- Location: tests/desk.spec.mjs:1652:3

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#journal
Call log:
  - navigating to "http://127.0.0.1:1500/desk.html#journal", waiting until "load"

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
  1614 |   const bars = page.getByRole("region", { name: "Kilometre splits" }).locator(".split-bar > span");
  1615 |   expect(await bars.count()).toBeGreaterThan(1);
  1616 |   const widths = await bars.evaluateAll(nodes => nodes.map(node => node.getBoundingClientRect().width));
  1617 |   expect(Math.max(...widths)).toBeGreaterThan(Math.min(...widths) * 1.05);
  1618 |   await expect(page.getByRole("columnheader", { name: "Elevation" })).toBeVisible();
  1619 |   await expect(page.getByRole("region", { name: "Kilometre splits" }).locator("tbody tr").first().locator("td").nth(2)).toContainText(/m|Not recorded/);
  1620 |   await page.getByRole("button", { name: "Back", exact: true }).click();
  1621 |   await expect(page.locator(".activity-row").first()).toBeFocused();
  1622 | });
  1623 | 
  1624 | test("pain logging retains failure drafts and episode conversations survive refresh", async ({ page }) => {
  1625 |   await page.goto("/desk.html#health/journal");
  1626 |   await page.getByRole("button", { name: "Log pain", exact: true }).first().click();
  1627 |   await page.getByRole("combobox", { name: "Body region", exact: true }).selectOption("knee.left");
  1628 |   await page.getByRole("textbox", { name: "Notes", exact: true }).fill("Synthetic test: soreness after running");
  1629 |   await page.evaluate(() => { window.__deskFail = "save_pain_entry"; });
  1630 |   await page.getByRole("button", { name: "Save pain entry" }).click();
  1631 |   await expect(page.getByRole("alert")).toContainText("Unable to save right now");
  1632 |   await expect(page.getByRole("textbox", { name: "Notes", exact: true })).toHaveValue("Synthetic test: soreness after running");
  1633 |   await page.evaluate(() => { window.__deskFail = ""; });
  1634 |   await page.getByRole("button", { name: "Save pain entry" }).click();
  1635 |   await expect(page.getByRole("dialog")).toHaveCount(0);
  1636 |   await expect(page.locator(".pain-episode")).toContainText("Left knee");
  1637 |   await page.getByRole("button", { name: "Discuss episode" }).click();
  1638 |   await page.getByRole("textbox", { name: "Your question" }).fill("Should I take a rest day?");
  1639 |   await expect(page.getByRole("button", { name: "Send question" })).toBeDisabled();
  1640 |   await page.getByRole("checkbox", { name: /I agree to send/ }).check();
  1641 |   await page.getByRole("button", { name: "Send question" }).click();
  1642 |   await expect(page.locator(".chat-message.assistant")).toContainText("reducing the session");
  1643 |   await expect(page.getByRole("textbox", { name: "Your question" })).toHaveValue("");
  1644 |   await page.getByRole("button", { name: "Mark resolved" }).click();
  1645 |   await expect(page.locator(".pain-episode")).toHaveCount(0);
  1646 |   await page.getByRole("checkbox", { name: "Include resolved" }).check();
  1647 |   await page.getByRole("button", { name: "Reopen", exact: true }).click();
  1648 |   await expect(page.locator(".pain-episode")).toContainText("Active");
  1649 | });
  1650 | 
  1651 | for (const route of ["journal", "health", "health/milestones"]) {
  1652 |   test(`${route} loads backend evidence and meets accessibility checks`, async ({ page }) => {
  1653 |     const errors = [];
  1654 |     page.on("pageerror", error => errors.push(error.message));
> 1655 |     await page.goto(`/desk.html#${route}`);
       |                ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:1500/desk.html#journal
  1656 |     await expect(page.locator(".desk-loading")).toHaveCount(0);
  1657 |     await expect(page.locator(".error-notice")).toHaveCount(0);
  1658 |     expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  1659 |     expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
  1660 |     expect(errors).toEqual([]);
  1661 |   });
  1662 | }
  1663 | 
  1664 | test("settings directory searches tasks and preserves unrelated settings", async ({ page }) => {
  1665 |   await page.goto("/desk.html#settings");
  1666 |   await page.getByRole("searchbox", { name: "Search settings" }).fill("colour");
  1667 |   await expect(page.locator(".settings-task-list a")).toHaveCount(1);
  1668 |   await page.getByRole("link", { name: /Appearance/ }).click();
  1669 |   await page.getByRole("radio", { name: "Dark", exact: true }).check();
  1670 |   await expect(page.locator("html")).toHaveAttribute("data-desk-theme", "dark");
  1671 |   const settings = await page.evaluate(() => window.__deskCalls.find(call => call.command === "save_settings").args.settings);
  1672 |   expect(settings.llmEndpoint).toBe("https://example.test/v1");
  1673 |   expect(settings.thresholds.buildWeeks).toBe(3);
  1674 |   await page.getByRole("radio", { name: "Glacier", exact: true }).check();
  1675 |   await expect(page.locator("html")).toHaveAttribute("data-desk-palette", "glacier");
  1676 | });
  1677 | 
  1678 | test("calendar settings never publish on read and retain blank credentials and sync metadata", async ({ page }) => {
  1679 |   await page.goto("/desk.html#settings/calendar");
  1680 |   await expect(page.getByRole("button", { name: "Save calendar settings" })).toBeVisible();
  1681 |   expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "publish_calendar"))).toEqual([]);
  1682 |   await page.getByRole("checkbox", { name: "Publish calendar remotely" }).check();
  1683 |   await page.getByRole("button", { name: "Save calendar settings" }).click();
  1684 |   await expect(page.getByRole("status").last()).toContainText("Calendar settings saved");
  1685 |   const payload = await page.evaluate(() => window.__deskCalls.find(call => call.command === "save_calendar_publishing").args);
  1686 |   expect(payload.password).toBeNull();
  1687 |   expect(payload.config.lastPublished).toBe("2026-10-29T12:00:00Z");
  1688 |   await page.getByRole("checkbox", { name: "Sync this plan across devices" }).uncheck();
  1689 |   const sync = await page.evaluate(() => window.__deskCalls.find(call => call.command === "plan_sync_configure").args);
  1690 |   expect(sync).toEqual({config:{enabled:false},token:null});
  1691 |   await expect(page.getByText("synthetic-account-channel", {exact:true})).toBeVisible();
  1692 |   await expect(page.getByRole("button", { name: "Import plan", exact:true })).toBeDisabled();
  1693 | });
  1694 | 
  1695 | test("clearing training data needs confirmation and never touches recorded history", async ({ page }) => {
  1696 |   await page.goto("/desk.html#settings/training");
  1697 |   await expect(page.getByRole("link", { name: "Only end the current plan" })).toHaveAttribute("href", "#training/plan");
  1698 |   await expect(page.getByRole("button", { name: "Remove all saved plans", exact: true })).toBeVisible();
  1699 |   await page.getByRole("button", { name: "Remove all saved plans", exact: true }).click();
  1700 |   await expect(page.getByRole("dialog")).toContainText("Pending plan offers are also removed. This cannot be undone.");
  1701 |   expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "clear_local_plan_data"))).toEqual([]);
  1702 |   await page.getByRole("button", { name: "Keep my plans", exact: true }).click();
  1703 |   await expect(page.getByRole("dialog")).toHaveCount(0);
  1704 |   await page.getByRole("button", { name: "Remove all saved plans", exact: true }).click();
  1705 |   await page.getByRole("dialog").getByRole("button", { name: "Remove all saved plans", exact: true }).click();
  1706 |   await expect(page.getByRole("dialog")).toHaveCount(0);
  1707 |   expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "clear_local_plan_data"))).toHaveLength(1);
  1708 |   expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "delete_pain_entry" || call.command === "clear_pain_messages"))).toBe(false);
  1709 |   await expect(page.getByRole("button", { name: "Remove all saved plans", exact: true })).toBeDisabled();
  1710 | });
  1711 | 
  1712 | test("plan import only overwrites after explicit confirmation", async ({ page }) => {
  1713 |   await page.goto("/desk.html?syncConflict=1#settings/calendar");
  1714 |   await page.getByRole("button", { name: "Import plan", exact: true }).click();
  1715 |   await expect(page.getByRole("dialog", {name:"Replace the plan on this device?"})).toBeVisible();
  1716 |   expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_pull").map(call => call.args.force))).toEqual([false]);
  1717 |   await page.getByRole("button", { name:"Keep local plan" }).click();
  1718 |   await page.getByRole("button", { name:"Import plan", exact:true }).click();
  1719 |   await page.getByRole("button", { name:"Replace local plan", exact:true }).click();
  1720 |   await expect(page.getByRole("dialog")).toHaveCount(0);
  1721 |   await expect(page.getByRole("status").last()).toContainText("Imported 33 sessions");
  1722 |   expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_pull").map(call => call.args.force))).toEqual([false,false,true]);
  1723 | });
  1724 | 
  1725 | test("training boundaries keep fractional units and re-read saved values", async ({ page }) => {
  1726 |   await page.goto("/desk.html#settings/thresholds");
  1727 |   await page.getByRole("spinbutton", { name:"Recovery week load (%)" }).fill("65");
  1728 |   await page.getByRole("button", { name:"Save boundaries" }).click();
  1729 |   await expect(page.getByRole("status")).toContainText("Training boundaries saved");
  1730 |   const settings = await page.evaluate(() => window.__deskCalls.find(call => call.command === "save_settings").args.settings);
  1731 |   expect(settings.thresholds.downWeekFactor).toBe(.65);
  1732 |   expect(settings.llmModel).toBe("test-model");
  1733 |   await expect(page.getByRole("spinbutton", { name:"Recovery week load (%)" })).toHaveValue("65");
  1734 | });
  1735 | 
  1736 | test("removing an assistant clears its key without deleting pain data", async ({ page }) => {
  1737 |   await page.goto("/desk.html#settings/assistant");
  1738 |   await page.getByRole("button", { name:"Remove assistant", exact:true }).click();
  1739 |   await page.getByRole("dialog").getByRole("button", {name:"Remove assistant",exact:true}).click();
  1740 |   await expect(page.getByRole("button", {name:"Add assistant",exact:true})).toBeVisible();
  1741 |   expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "clear_llm_api_key"))).toBe(true);
  1742 |   expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "delete_pain_entry" || call.command === "clear_pain_messages"))).toBe(false);
  1743 |   await expect(page.getByRole("button", {name:"Add assistant",exact:true})).toBeDisabled();
  1744 | });
  1745 | 
  1746 | for (const width of [320, 1440]) {
  1747 |   test(`information dialogs preserve focus and scroll locks at ${width}px`, async ({ page }) => {
  1748 |     await page.setViewportSize({ width, height: 844 });
  1749 |     for (const [route, title, content] of [
  1750 |       ["health", "Data for readiness", "Readiness measurements"],
  1751 |       ["health", "Training boundaries", "Decreasing ceiling"],
  1752 |       ["health", "Estimate basis", "Riegel model"],
  1753 |       ["health/pace", "About pace insights", "Rep boundaries are inferred"],
  1754 |       ["settings/watch", "Connection diagnostics", "Check COROS API"],
  1755 |       ["settings/calendar", "Local calendar file", "Reveal calendar"],
```