import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { readFile } from "node:fs/promises";
import { installDeskFixture } from "./desk.fixture.mjs";

test.beforeEach(async ({ page }) => installDeskFixture(page));

for (const reducedMotion of ["reduce", "no-preference"]) test(`packaged motion preserves CSS time units with ${reducedMotion} motion`, async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ reducedMotion });
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.route("**/assets/*", async route => {
    const name = new URL(route.request().url()).pathname.split("/").pop();
    const contentType = name.endsWith(".js") ? "text/javascript" : name.endsWith(".css") ? "text/css" : "application/octet-stream";
    await route.fulfill({ body: await readFile(new URL(`../dist/assets/${name}`, import.meta.url)), contentType });
  });
  await page.route("**/desk.html", async route => route.fulfill({ body: await readFile(new URL("../dist/desk.html", import.meta.url)), contentType: "text/html" }));
  await page.addInitScript(() => {
    window.__packagedAnimations = [];
    const animate = Element.prototype.animate;
    Element.prototype.animate = function (keyframes, options) {
      window.__packagedAnimations.push({ duration: options?.duration, keyframes });
      return animate.call(this, keyframes, options);
    };
    window.__packagedCounts = [];
    new MutationObserver(() => {
      const counter = document.querySelector('.health-verdict .sr-only + [aria-hidden="true"]');
      if (counter) window.__packagedCounts.push({ value: counter.textContent, time: performance.now() });
    }).observe(document, { subtree: true, characterData: true, childList: true });
  });
  await page.goto("/desk.html#health");
  const counter = page.locator('.health-verdict .sr-only + [aria-hidden="true"]').first();
  await expect(counter).toBeVisible();
  expect(errors).toEqual([]);
  const final = counter.locator("..").locator(".sr-only");
  await expect.poll(() => counter.textContent()).toBe(await final.textContent());
  if (reducedMotion === "no-preference") {
    expect(await page.evaluate(() => window.__packagedAnimations.map(animation => animation.duration))).toContain(560);
    const counts = await page.evaluate(() => window.__packagedCounts);
    expect(new Set(counts.map(sample => sample.value)).size).toBeGreaterThan(2);
    expect(counts.at(-1).time - counts[0].time).toBeGreaterThan(200);
    await expect(page.locator(".health-verdict")).toHaveAttribute("data-reveal", "settled");
    const frames = await page.getByRole("link", { name: "Activities", exact: true }).evaluate(async link => {
      link.click();
      const frames = [];
      for (let index = 0; index < 12; index++) {
        await new Promise(requestAnimationFrame);
        const leaf = document.querySelector('[data-transition-state="exiting"] .health-verdict');
        if (leaf) frames.push({ opacity: Number(getComputedStyle(leaf).opacity), y: new DOMMatrixReadOnly(getComputedStyle(leaf).transform).m42 });
      }
      return frames;
    });
    expect(frames.length).toBeGreaterThan(1);
    expect(frames.some(frame => frame.opacity > 0 && frame.opacity < .95 && frame.y < -1)).toBe(true);
    expect(await page.evaluate(() => window.__packagedAnimations.map(animation => animation.duration))).toContain(500);
    await expect(page.locator(".activity-row")).toHaveCount(9);
  } else {
    expect(new Set(await page.evaluate(() => window.__packagedCounts.map(sample => sample.value))).size).toBe(1);
    expect(await page.locator(".health-verdict").evaluate(element => element.getAnimations().length)).toBe(0);
  }
  expect(errors).toEqual([]);
});

for (const reducedMotion of ["reduce", "no-preference"]) {
  test(`entry counters preserve final values with ${reducedMotion} motion`, async ({ page }) => {
    await page.emulateMedia({ reducedMotion });
    await page.addInitScript(() => {
      window.__counterValues = [];
      window.__counterTimes = [];
      new MutationObserver(records => {
        for (const record of records) {
          const element = record.target.nodeType === Node.TEXT_NODE ? record.target.parentElement : record.target;
          if (element?.matches?.('.health-verdict [aria-hidden="true"]') && element.previousElementSibling?.classList.contains("sr-only")) { window.__counterValues.push(element.textContent); window.__counterTimes.push(performance.now()); }
        }
      }).observe(document, { subtree: true, characterData: true, childList: true });
    });
    for (const [route, selector] of [["training/week", ".schedule-summary"], ["health", ".health-verdict"], ["journal/1", ".activity-review .metrics-line"]]) {
      await page.goto(`/desk.html#${route}`);
      const counter = page.locator(`${selector} .sr-only + [aria-hidden="true"]`).first();
      await expect(counter).toBeVisible();
      const final = counter.locator("..").locator(".sr-only");
      await expect(final).not.toBeEmpty();
      await expect.poll(async () => counter.textContent()).toBe(await final.textContent());
      if (route === "health" && reducedMotion === "no-preference") {
        expect(new Set(await page.evaluate(() => window.__counterValues)).size).toBeGreaterThan(1);
        expect(await page.evaluate(() => window.__counterTimes.at(-1) - window.__counterTimes[0])).toBeGreaterThan(200);
      }
    }
  });
}

for (const listenerStyle of ["camel", "snake"]) {
  test(`native chrome transport uses the installed Tauri listener contract ${listenerStyle}`, async ({ page }) => {
    await page.goto("/desk.html#training");
    await expect(page.locator(".desk-navigation")).toBeVisible();
    const result = await page.evaluate(async style => {
      const { createNativeChromeBridge, createTauriChromeTransport } = await import("/src/desk/native-chrome.ts");
      const original = window.__TAURI_INTERNALS__.invoke;
      const calls = [];
      const routes = [];
      const states = [];
      let handler;
      let current;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        if (!command.startsWith("plugin:native-chrome|")) return original(command, args);
        calls.push(command);
        if (command.endsWith("register_listener") || command.endsWith("remove_listener")) {
          if (style === "camel") throw new Error("Unknown native command");
        }
        if (command.endsWith("registerListener") || command.endsWith("register_listener")) {
          handler = args.handler;
          return null;
        }
        if (command.endsWith("|attach")) {
          current = args;
          return { protocolVersion: 1, session: args.session, revision: args.revision, bottomInset: 83 };
        }
        return null;
      };
      const bridge = createNativeChromeBridge(await createTauriChromeTransport(), {
        onNavigate: workspace => routes.push(workspace),
        onState: state => states.push(state),
      });
      try {
        const attached = await bridge.update({
          tabs: [{ id: "training", title: "Training" }, { id: "journal", title: "Activities" }, { id: "health", title: "Health" }],
          selected: "training", theme: "light",
        });
        handler.onmessage({ session: current.session, revision: current.revision, eventId: "native-tap", action: "navigate", workspace: "journal" });
        await bridge.close();
        return { attached, calls, routes, states, hash: location.hash };
      } finally {
        await bridge.close();
        window.__TAURI_INTERNALS__.invoke = original;
      }
    }, listenerStyle);
    expect(result.attached).toBe(true);
    expect(result.routes).toEqual(["journal"]);
    expect(result.hash).toBe("#training");
    expect(result.states).toContainEqual({ ready: true, bottomInset: 83 });
    expect(result.states.at(-1)).toEqual({ ready: false, bottomInset: 0 });
    expect(result.calls[0]).toBe("plugin:native-chrome|register_listener");
    expect(result.calls).toContain("plugin:native-chrome|attach");
    expect(result.calls.at(-1)).toBe(`plugin:native-chrome|${listenerStyle === "camel" ? "removeListener" : "remove_listener"}`);
    await expect(page.locator(".desk-navigation")).toBeVisible();
  });
}

for (const width of [390, 1440]) for (const reducedMotion of ["reduce", "no-preference"]) {
  test(`counters wait for viewport entry ${width}px ${reducedMotion}`, async ({ page }) => {
    await page.setViewportSize({ width, height: 844 });
    await page.emulateMedia({ reducedMotion });
    await page.addInitScript(() => {
      document.addEventListener("DOMContentLoaded", () => {
        const style = document.createElement("style");
        style.textContent = ".health-verdict { margin-top: 1400px; }";
        document.head.append(style);
      });
      window.__viewportCounts = [];
      new MutationObserver(() => {
        const counter = document.querySelector('.health-verdict .sr-only + [aria-hidden="true"]');
        if (counter) window.__viewportCounts.push(counter.textContent);
      }).observe(document, { subtree: true, characterData: true, childList: true });
    });
    await page.goto("/desk.html#health");
    const counter = page.locator('.health-verdict .sr-only + [aria-hidden="true"]').first();
    await expect(counter).toBeAttached();
    const final = await counter.locator("..").locator(".sr-only").textContent();
    await page.evaluate(async () => { for (let frame = 0; frame < 65; frame++) await new Promise(requestAnimationFrame); });
    await expect(counter).toHaveText(reducedMotion === "reduce" ? final : "0");
    await counter.scrollIntoViewIfNeeded();
    await expect(counter).toBeInViewport();
    await expect(counter).toHaveText(final);
    const values = new Set(await page.evaluate(() => window.__viewportCounts));
    expect(values.size).toBeGreaterThan(reducedMotion === "reduce" ? 0 : 2);
    if (reducedMotion === "reduce") expect(values.size).toBe(1);
    await page.evaluate(() => document.getElementById("workspace").scrollTo({ top: 0, behavior: "instant" }));
    await counter.scrollIntoViewIfNeeded();
    await expect(counter).toHaveText(final);
    await expect(counter.locator("..").locator(".sr-only")).toHaveText(final);
  });
}

for (const startup of ["probe", "iphone"]) for (const attachment of ["ready", "unavailable"]) {
  test(`native chrome ${startup} startup keeps guarded UI ownership ${attachment}`, async ({ page }) => {
    await page.addInitScript(({ mode, startup }) => {
      if (startup === "iphone") {
        Object.defineProperty(navigator, "userAgent", { value: "Mozilla/5.0 (iPhone; CPU iPhone OS 27_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148" });
        window.isTauri = true;
      }
      document.addEventListener("DOMContentLoaded", () => {
        const original = window.__TAURI_INTERNALS__.invoke;
        window.__nativeProbeAttaches = 0;
        window.__nativeProbeDetaches = 0;
        window.__TAURI_INTERNALS__.invoke = async (command, args) => {
          if (command === "plugin:native-chrome|register_listener") throw new Error("Unknown command");
          if (command === "plugin:native-chrome|registerListener") { window.__nativeProbeHandler = args.handler; return null; }
          if (command === "plugin:native-chrome|attach") {
            window.__nativeProbeAttaches += 1;
            if (mode === "unavailable") throw new Error("Native host unavailable");
            window.__nativeProbeSnapshot = args;
            return { protocolVersion: 1, session: args.session, revision: args.revision, bottomInset: 83 };
          }
          if (command === "plugin:native-chrome|detach") { window.__nativeProbeDetaches += 1; return null; }
          if (command.startsWith("plugin:native-chrome|remove")) return null;
          return original(command, args);
        };
      });
    }, { mode: attachment, startup });
    await page.goto(startup === "probe" ? "/desk.html?nativeChromeProbe=1#training" : "/desk.html#training");
    await expect(page.locator("html")).toHaveAttribute("data-native-navigation", attachment === "ready" ? "ready" : "fallback");
    if (attachment === "unavailable") {
      await expect(page.locator(".desk-navigation")).toBeVisible();
      await expect.poll(() => page.evaluate(() => window.__nativeProbeDetaches)).toBeGreaterThan(0);
      return;
    }
    await expect(page.locator(".desk-navigation")).toBeHidden();
    for (const theme of ["dark", "light"]) for (const palette of ["sakura", "glacier"]) {
      const expected = await page.evaluate(({ theme, palette }) => {
        const root = document.documentElement;
        root.dataset.deskTheme = theme;
        root.dataset.deskPalette = palette;
        const style = getComputedStyle(root);
        const read = token => {
          const hex = style.getPropertyValue(token).trim().slice(1);
          const value = hex.length === 3 ? [...hex].map(character => character + character).join("") : hex;
          return [0, 2, 4].map(offset => parseInt(value.slice(offset, offset + 2), 16) / 255).concat(1);
        };
        return { surface: read("--desk-surface"), selected: read("--desk-action"), unselected: read("--desk-muted") };
      }, { theme, palette });
      await expect.poll(() => page.evaluate(() => ({ theme: window.__nativeProbeSnapshot.theme, colors: window.__nativeProbeSnapshot.colors }))).toEqual({ theme, colors: expected });
      await expect(page.locator("html")).toHaveAttribute("data-native-navigation", "ready");
    }
    const initial = await page.evaluate(() => window.__nativeProbeAttaches);
    const tap = workspace => page.evaluate(selected => {
      window.__nativeProbeHandler.onmessage({ ...window.__nativeProbeSnapshot, action: "navigate", workspace: selected, eventId: crypto.randomUUID() });
    }, workspace);
    await tap("training");
    await expect.poll(() => page.evaluate(() => window.__nativeProbeAttaches)).toBeGreaterThan(initial);
    await expect(page.locator("html")).toHaveAttribute("data-native-navigation", "ready");
    await expect(page.locator(".desk-navigation")).toBeHidden();
    await tap("journal");
    await expect(page.locator(".workspace-heading h1")).toHaveText("Activities");
    await expect(page.locator("html")).toHaveAttribute("data-native-navigation", "ready");
    await page.getByRole("button", { name: "Help", exact: true }).click();
    await expect.poll(() => page.evaluate(() => window.__nativeProbeSnapshot.selected)).toBeNull();
    const inset = await page.locator("#workspace").evaluate(element => getComputedStyle(element).paddingBottom);
    await page.getByRole("button", { name: "Understand recovery and risk", exact: true }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("data-native-navigation", "fallback");
    await expect.poll(() => page.evaluate(() => window.__nativeProbeDetaches)).toBeGreaterThan(0);
    await expect(page.locator(".desk-navigation")).toBeHidden();
    await expect(page.locator("#workspace")).toHaveCSS("padding-bottom", inset);
    await page.getByRole("dialog").getByRole("button", { name: /^Close / }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.locator("html")).toHaveAttribute("data-native-navigation", "ready");
    await expect(page.locator(".desk-navigation")).toBeHidden();
    await page.setViewportSize({ width: 900, height: 402 });
    await expect(page.locator("html")).toHaveAttribute("data-native-navigation", "fallback");
    await expect(page.locator(".desk-navigation")).toBeVisible();
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(page.locator("html")).toHaveAttribute("data-native-navigation", "ready");
    await page.evaluate(() => {
      const input = document.createElement("input");
      input.id = "native-navigation-input";
      document.body.append(input);
      input.focus();
    });
    await expect(page.locator("html")).toHaveAttribute("data-native-navigation", "fallback");
    await tap("health");
    await expect(page.locator(".workspace-heading h1")).toHaveText("Help");
    await page.locator("#native-navigation-input").evaluate(input => input.remove());
    await expect(page.locator("html")).toHaveAttribute("data-native-navigation", "ready");
  });
}

test("native chrome stays inactive in ordinary browsers and desktop Tauri", async ({ page }) => {
  await page.goto("/desk.html#training");
  await expect(page.locator(".desk-navigation")).toBeVisible();
  await expect(page.locator("html")).not.toHaveAttribute("data-native-navigation");
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command.startsWith("plugin:native-chrome|")))).toBe(false);
});

test("macOS window dragging is limited to empty outer-shell background", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.addInitScript(() => {
    document.addEventListener("DOMContentLoaded", () => { document.documentElement.dataset.nativeWindow = "macos"; });
  });
  await page.goto("/desk.html#health/pace");
  await expect(page.locator(".workspace-heading h1")).toHaveText("Health");
  const dragCount = () => page.evaluate(() => window.__deskCalls.filter(call => call.command === "plugin:window|start_dragging").length);
  await page.locator("#desk-root").dispatchEvent("mousedown", { button: 0 });
  await expect.poll(dragCount).toBe(1);
  await page.locator("body").dispatchEvent("mousedown", { button: 0 });
  await expect.poll(dragCount).toBe(2);
  await page.locator("html").dispatchEvent("mousedown", { button: 0 });
  await expect.poll(dragCount).toBe(3);
  await page.locator("#desk-root").dispatchEvent("mousedown", { button: 2 });
  await page.locator(".desk-appbar").dispatchEvent("mousedown", { button: 0 });
  await page.locator("#workspace").dispatchEvent("mousedown", { button: 0 });
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(page.locator(".workspace-heading h1")).toHaveText("Settings");
  expect(await dragCount()).toBe(3);
  await page.evaluate(() => { delete document.documentElement.dataset.nativeWindow; });
  await page.locator("#desk-root").dispatchEvent("mousedown", { button: 0 });
  expect(await dragCount()).toBe(3);
});

for (const width of [320, 820, 1440]) for (const theme of ["light", "dark"]) {
  test(`desktop updater keeps progress and feedback spaced ${width}px ${theme}`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(`/desk.html?theme=${theme}#settings/about`);
    await page.getByRole("button", { name: "Check for updates", exact: true }).click();
    const install = page.getByRole("button", { name: "Install 0.26.0 and restart", exact: true });
    await expect(install).toBeEnabled();
    await install.click();
    await expect(install).toBeDisabled();
    await page.evaluate(() => {
      window.__deskUpdateEvent({ event: "Started", data: { contentLength: 100 } });
      window.__deskUpdateEvent({ event: "Progress", data: { chunkLength: 50 } });
    });
    const progress = page.getByRole("progressbar", { name: "Update download" });
    await expect(progress).toHaveAttribute("value", "50");
    const actionBox = await install.boundingBox();
    const progressBox = await progress.boundingBox();
    const status = page.locator(".app-update [role=status]");
    const statusBox = await status.boundingBox();
    const releasesBox = await page.getByRole("button", { name: "Browse releases" }).boundingBox();
    expect(progressBox.y - actionBox.y - actionBox.height).toBeGreaterThanOrEqual(12);
    expect(statusBox.y - progressBox.y - progressBox.height).toBeGreaterThanOrEqual(12);
    expect(releasesBox.y - statusBox.y - statusBox.height).toBeGreaterThanOrEqual(16);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.evaluate(() => window.__deskUpdateEvent({ event: "Finished" }));
    await expect(status).toHaveText("Download complete. Installing...");
    expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "plugin:process|restart"))).toBe(false);
    expect((await new AxeBuilder({ page }).include(".settings-editor").analyze()).violations).toEqual([]);
    await page.screenshot({ path: test.info().outputPath(`updater-${width}-${theme}.png`), fullPage: true });
    await page.evaluate(() => window.__deskFinishUpdate());
    await expect(status).toHaveText("Installed. Restarting...");
    await expect.poll(() => page.evaluate(() => window.__deskCalls.some(call => call.command === "plugin:process|restart"))).toBe(true);
    expect(errors).toEqual([]);
  });
}

test("desktop updater failures clear stale progress and allow retry", async ({ page }) => {
  await page.goto("/desk.html#settings/about");
  await page.getByRole("button", { name: "Check for updates", exact: true }).click();
  const install = page.getByRole("button", { name: "Install 0.26.0 and restart", exact: true });
  await install.click();
  await page.evaluate(() => {
    window.__deskUpdateEvent({ event: "Started", data: { contentLength: 100 } });
    window.__deskUpdateEvent({ event: "Progress", data: { chunkLength: 50 } });
    window.__deskRejectUpdate();
  });
  await expect(page.getByRole("progressbar", { name: "Update download" })).toHaveCount(0);
  await expect(page.locator(".app-update [role=status]")).toHaveText("Update not installed. Try again or browse releases.");
  await expect(install).toBeEnabled();
  await install.click();
  await expect(page.getByRole("progressbar", { name: "Update download" })).not.toHaveAttribute("value");
  await page.evaluate(() => window.__deskFinishUpdate());
  await expect(page.locator(".app-update [role=status]")).toHaveText("Installed. Restarting...");
});

test.describe("plan freshness", () => {
  test("failed checks retry without overwriting the visible plan and remote endings clear it", async ({ page }) => {
    await page.clock.install();
    await page.addInitScript(() => { window.__deskFail = "plan_sync_refresh"; });
    await page.goto("/desk.html#training/plan");
    await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_refresh").length)).toBe(1);
    await page.evaluate(async () => {
      const pair = await window.__TAURI_INTERNALS__.invoke("current_plan");
      window.__deskRemotePlan = [{ ...pair[0], name: "Recovered remote plan" }, pair[1]];
      window.__deskFail = "";
    });
    await page.clock.fastForward(16000);
    await expect(page.locator("#workspace")).toContainText("Recovered remote plan");
    await page.evaluate(() => { window.__deskRemotePlan = null; window.dispatchEvent(new Event("online")); });
    await page.clock.fastForward(2500);
    await expect.poll(() => page.evaluate(() => window.__TAURI_INTERNALS__.invoke("current_plan"))).toBeNull();
    expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_push").length)).toBe(0);
  });

  test("open inspectors refresh evidence and retain drafts when a remote session disappears", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.goto("/desk.html");
    await page.getByRole("button", { name: "Next week", exact: true }).click();
    const tile = page.locator(".week-board .session-tile.movable").first();
    const workoutId = await tile.getAttribute("data-workout-id");
    await tile.click();
    const inspector = page.getByRole("dialog", { name: "Session", exact: true });
    const date = inspector.getByLabel("Run on", { exact: true });
    const draft = "2026-12-01";
    await date.fill(draft);
    await page.evaluate(async workoutId => {
      const pair = await window.__TAURI_INTERNALS__.invoke("current_plan");
      window.__deskRemotePlan = [pair[0], pair[1].map(workout => workout.id === workoutId ? { ...workout, title: "Updated remote prescription" } : workout)];
      await window.__TAURI_INTERNALS__.invoke("plan_sync_refresh");
    }, workoutId);
    await expect(inspector.getByRole("heading", { name: "Updated remote prescription" })).toBeVisible();
    await expect(date).toHaveValue(draft);
    await page.evaluate(async workoutId => {
      const pair = await window.__TAURI_INTERNALS__.invoke("current_plan");
      window.__deskRemotePlan = [pair[0], pair[1].filter(workout => workout.id !== workoutId)];
      await window.__TAURI_INTERNALS__.invoke("plan_sync_refresh");
    }, workoutId);
    await expect(inspector.getByRole("alert")).toContainText("no longer in the latest plan");
    await expect(date).toHaveValue(draft);
    await expect(inspector.getByRole("button", { name: "Move session", exact: true })).toBeDisabled();
    await expect(inspector.getByRole("button", { name: "Skip session", exact: true })).toBeDisabled();
  });

  for (const width of [320, 390, 768, 820, 1440]) {
    for (const theme of ["light", "dark"]) {
      test(`sync status fits ${width}px ${theme}`, async ({ page }) => {
        await page.setViewportSize({ width, height: 900 });
        await page.goto(`/desk.html?theme=${theme}#settings/devices`);
        await expect(page.getByText("Plan up to date", { exact: true })).toBeVisible();
        await page.getByRole("button", { name: "More options", exact: true }).click();
        await expect(page.getByRole("button", { name: "Sync now", exact: true })).toBeEnabled();
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
        expect((await new AxeBuilder({ page }).include(".settings-editor").analyze()).violations).toEqual([]);
        await page.screenshot({ path: test.info().outputPath(`plan-sync-${width}-${theme}.png`), fullPage: true });
      });
    }
  }

  for (const width of [390, 1440]) {
    test(`foreground and polling adopt the Gist plan at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.clock.install();
      await page.goto("/desk.html#training/plan");
      await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_refresh").length)).toBe(1);
      await page.clock.fastForward(2500);
      await page.evaluate(async () => {
        const pair = await window.__TAURI_INTERNALS__.invoke("current_plan");
        window.__deskRemotePlan = [{ ...pair[0], id: "desktop-new", name: "Desktop replacement" }, pair[1].map(workout => ({ ...workout, planId: "desktop-new" }))];
        window.dispatchEvent(new Event("focus"));
        window.dispatchEvent(new Event("pageshow"));
        document.dispatchEvent(new Event("visibilitychange"));
      });
      await page.clock.fastForward(250);
      await expect(page.locator("#workspace")).toContainText("Desktop replacement");
      expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_refresh").length)).toBe(2);
      await page.evaluate(async () => {
        const pair = await window.__TAURI_INTERNALS__.invoke("current_plan");
        window.__deskRemotePlan = [{ ...pair[0], name: "Latest mobile edit" }, pair[1]];
      });
      await page.clock.fastForward(60000);
      await expect(page.locator("#workspace")).toContainText("Latest mobile edit");
      expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_push").length)).toBe(0);
    });
  }

  test("offline and hidden checks wait for reconnection and preserve credential drafts", async ({ page }) => {
    await page.clock.install();
    await page.goto("/desk.html#settings/calendar");
    await page.getByRole("button", { name: "Connect a calendar", exact: true }).click();
    await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_refresh").length)).toBe(1);
    const upload = page.getByLabel("Upload URL", { exact: true });
    await upload.fill("https://example.test/unsaved-calendar.ics");
    await page.evaluate(async () => {
      const pair = await window.__TAURI_INTERNALS__.invoke("current_plan");
      window.__deskRemotePlan = [{ ...pair[0], name: "Remote while offline" }, pair[1]];
      Object.defineProperty(navigator, "onLine", { configurable: true, get: () => false });
      Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "hidden" });
    });
    await page.clock.fastForward(60000);
    expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_refresh").length)).toBe(1);
    await page.evaluate(() => {
      Object.defineProperty(navigator, "onLine", { configurable: true, get: () => true });
      Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "visible" });
      window.dispatchEvent(new Event("online"));
    });
    await page.clock.fastForward(250);
    await expect.poll(() => page.evaluate(async () => (await window.__TAURI_INTERNALS__.invoke("current_plan"))[0].name)).toBe("Remote while offline");
    await expect(upload).toHaveValue("https://example.test/unsaved-calendar.ics");
    await expect(upload).toBeFocused();
  });

  test("event bursts do not overlap checks and disabled sync does not poll", async ({ page }) => {
    await page.clock.install();
    await page.addInitScript(() => { window.__deskHoldRefresh = true; });
    await page.goto("/desk.html");
    await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_refresh").length)).toBe(1);
    await page.evaluate(() => { window.dispatchEvent(new Event("focus")); window.dispatchEvent(new Event("online")); });
    await page.clock.fastForward(60000);
    expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_refresh").length)).toBe(1);
    await page.evaluate(async () => {
      await window.__TAURI_INTERNALS__.invoke("plan_sync_configure", { config: { enabled: false } });
      window.__deskHoldRefresh = false;
      window.__deskReleaseRefresh();
    });
    await page.clock.fastForward(120000);
    expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_refresh").length)).toBe(1);
  });
});

test.describe("Strava settings", () => {
  test("strava credentials, privacy opt-in, pending retries and local disconnect", async ({ page }) => {
    await page.goto("/desk.html#settings/strava");
    await expect(page.getByRole("button", { name: "Connect with Strava" })).toBeDisabled();
    await page.getByLabel("Strava Client ID", { exact: true }).fill("12345");
    await page.getByLabel("Strava Client Secret", { exact: true }).fill("synthetic-secret");
    await page.getByRole("button", { name: "Save API credentials" }).click();
    await expect(page.getByLabel("Strava Client Secret", { exact: true })).toHaveValue("");
    await page.getByRole("button", { name: "Save API credentials" }).click();
    await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call => call.command === "strava_configure").at(-1)?.args.clientSecret)).toBeNull();
    await page.getByRole("button", { name: "Connect with Strava" }).click();
    await expect(page.getByText("Strava connected: athlete 42", { exact: true })).toBeVisible();
    const toggle = page.getByRole("checkbox", { name: "Update paired activity descriptions" });
    await expect(toggle).not.toBeChecked();
    await toggle.click();
    const confirmation = page.getByRole("dialog", { name: "Enable Strava description updates?" });
    await expect(confirmation).toContainText("injury-risk information");
    await confirmation.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(toggle).not.toBeChecked();
    await toggle.click();
    await confirmation.getByRole("button", { name: "Enable updates", exact: true }).click();
    await expect(toggle).toBeChecked();
    await page.getByRole("button", { name: "Retry pending descriptions" }).click();
    await expect(page.getByRole("status").filter({ hasText: "1 copied, 1 already present" })).toBeVisible();
    await page.getByRole("button", { name: "Disconnect Strava" }).click();
    const disconnect = page.getByRole("dialog", { name: "Disconnect Strava?" });
    await expect(disconnect).toContainText("not revoked");
    await page.keyboard.press("Escape");
    await expect(disconnect).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Disconnect Strava" })).toBeFocused();
    await page.getByRole("button", { name: "Disconnect Strava" }).click();
    await disconnect.getByRole("button", { name: "Disconnect", exact: true }).click();
    await expect(page.getByText("Strava not connected", { exact: true })).toBeVisible();
    await expect(toggle).not.toBeChecked();
  });

  test("strava failed secure save retains the credential draft", async ({ page }) => {
    await page.goto("/desk.html#settings/strava");
    await page.getByLabel("Strava Client ID", { exact: true }).fill("12345");
    await page.getByLabel("Strava Client Secret", { exact: true }).fill("synthetic-secret");
    await page.evaluate(() => { window.__deskFail = "strava_configure"; });
    await page.getByRole("button", { name: "Save API credentials" }).click();
    await expect(page.getByRole("alert")).toContainText("Unable to save right now");
    await expect(page.getByLabel("Strava Client Secret", { exact: true })).toHaveValue("synthetic-secret");
    await expect(page.getByLabel("Strava Client ID", { exact: true })).toHaveValue("12345");
  });

  test("strava browser sign-in can be cancelled without enabling writes", async ({ page }) => {
    await page.goto("/desk.html#settings/strava");
    await page.getByLabel("Strava Client ID", { exact: true }).fill("12345");
    await page.getByLabel("Strava Client Secret", { exact: true }).fill("synthetic-secret");
    await page.getByRole("button", { name: "Save API credentials" }).click();
    await page.evaluate(() => { window.__deskHoldStrava = true; });
    await page.getByRole("button", { name: "Connect with Strava" }).click();
    await page.getByRole("button", { name: "Cancel sign-in", exact: true }).click();
    await expect(page.getByRole("alert")).toContainText("Strava sign-in was cancelled.");
    await expect(page.getByRole("button", { name: "Connect with Strava" })).toBeEnabled();
    await expect(page.getByRole("checkbox", { name: "Update paired activity descriptions" })).not.toBeChecked();
  });

  for (const width of [320, 390, 768, 820, 1440]) {
    for (const theme of ["light", "dark"]) {
      test(`strava settings fit ${width}px ${theme}`, async ({ page }) => {
        await page.setViewportSize({ width, height: 900 });
        await page.goto(`/desk.html?theme=${theme}#settings/strava`);
        await expect(page.getByRole("heading", { name: "Strava", exact: true })).toBeVisible();
        await expect(page.getByLabel("Strava Client ID", { exact: true })).toBeVisible();
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
        const accessibility = await new AxeBuilder({ page }).include(".settings-editor").analyze();
        expect(accessibility.violations).toEqual([]);
        await page.screenshot({ path: test.info().outputPath(`strava-${width}-${theme}.png`) });
      });
    }
  }
});

test.describe("motion system", () => {
  test("all action tones share bounded press feedback without moving their labels", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto("/desk.html#health");
    const action = page.getByRole("button", { name: "Log pain", exact: true });
    await expect(page.locator(".desk-toolbar")).toHaveAttribute("data-reveal", "settled");
    const rest = await action.boundingBox();
    await action.hover({ position: { x: rest.width / 2, y: rest.height / 2 } });
    await expect(action.locator(".button-content")).toHaveCSS("transform", "none");
    await expect(action).toHaveCSS("scale", "1.015");
    const hovered = await action.boundingBox();
    expect(hovered.width / rest.width).toBeCloseTo(1.015, 2);
    expect(hovered.x + hovered.width / 2).toBeCloseTo(rest.x + rest.width / 2, 1);
    for (const control of [action, page.getByRole("button", { name: "Settings", exact: true })]) {
      await control.hover();
      await page.mouse.down();
      await expect(control).toHaveCSS("scale", "0.98");
      await expect(control.locator(".button-content")).toHaveCSS("transform", "none");
      await page.mouse.move(0, 0);
      await page.mouse.up();
      await expect(control).toHaveCSS("scale", "none");
    }
    await page.emulateMedia({ reducedMotion: "reduce" });
    await action.hover();
    await page.mouse.down();
    await expect(action).toHaveCSS("scale", "1");
    await page.mouse.move(20, 200);
    await page.mouse.up();
    await action.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("dialog", { name: "Log pain" })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(action).toBeFocused();
  });

  for (const theme of ["light", "dark"]) {
    test(`translucent button depth and accessible fallbacks ${theme}`, async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 1000 });
      await page.emulateMedia({ reducedMotion: "no-preference", colorScheme: theme });
      await page.goto(`/desk.html?theme=${theme}#settings/thresholds`);
      const primary = page.getByRole("button", { name: "Save boundaries", exact: true });
      const secondary = page.getByRole("button", { name: "Restore defaults", exact: true });
      const alpha = control => control.evaluate(element => {
        const canvas = document.createElement("canvas");
        canvas.width = canvas.height = 1;
        const context = canvas.getContext("2d");
        context.fillStyle = getComputedStyle(element).backgroundColor;
        context.fillRect(0, 0, 1, 1);
        return context.getImageData(0, 0, 1, 1).data[3] / 255;
      });
      for (const control of [primary, secondary, page.getByRole("button", { name: "Help", exact: true }), page.getByRole("link", { name: "Health", exact: true })]) {
        await control.scrollIntoViewIfNeeded();
        await expect(control).toBeVisible();
        await expect.poll(() => alpha(control)).toBeLessThan(.7);
        const restShadow = await control.evaluate(element => getComputedStyle(element).boxShadow);
        await control.hover();
        await expect(control).toHaveCSS("translate", "0px -2px");
        await expect(control).toHaveCSS("transform", /matrix3d/);
        await expect.poll(() => control.evaluate(element => getComputedStyle(element).boxShadow)).not.toBe(restShadow);
        await expect.poll(() => alpha(control)).toBeLessThan(.7);
        const hoverShadow = await control.evaluate(element => getComputedStyle(element).boxShadow);
        await page.mouse.down();
        await expect(control).toHaveCSS("translate", "0px 1px");
        await expect(control).toHaveCSS("scale", "0.98");
        await expect.poll(() => control.evaluate(element => getComputedStyle(element).boxShadow)).not.toBe(hoverShadow);
        await expect.poll(() => alpha(control)).toBeLessThan(.7);
        await page.mouse.move(0, 0);
        await page.mouse.up();
        await expect(control).toHaveCSS("transform", "none");
      }
      await secondary.click();
      const dialog = page.getByRole("dialog", { name: "Restore training defaults?" });
      await expect(dialog).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(dialog).toHaveCount(0);
      await expect(secondary).toBeFocused();
      await page.emulateMedia({ reducedMotion: "reduce" });
      await primary.hover();
      await page.mouse.down();
      await expect(primary).toHaveCSS("transform", "none");
      await expect(primary).toHaveCSS("translate", "none");
      await expect(primary).toHaveCSS("scale", "1");
      await page.mouse.move(0, 0);
      await page.mouse.up();
      await primary.evaluate(element => { element.disabled = true; });
      await primary.hover();
      await expect(primary).toHaveCSS("box-shadow", "none");
      await primary.evaluate(element => { element.disabled = false; });
      await page.emulateMedia({ contrast: "more" });
      for (const control of [primary, secondary]) await expect.poll(() => alpha(control)).toBe(1);
      const session = await page.context().newCDPSession(page);
      await session.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-transparency", value: "reduce" }] });
      for (const control of [primary, secondary]) await expect.poll(() => alpha(control)).toBe(1);
      await session.send("Emulation.setEmulatedMedia", { features: [] });
      await session.detach();
      await page.emulateMedia({ reducedMotion: "no-preference", contrast: "no-preference", colorScheme: theme });
      await page.goto(`/desk.html?theme=${theme}#settings/training`);
      const danger = page.getByRole("button", { name: "Clear plans and scheduled training", exact: true });
      await expect(danger).toBeVisible();
      await expect.poll(() => alpha(danger)).toBeLessThan(.7);
      await danger.hover();
      await page.mouse.down();
      await expect(danger).toHaveCSS("translate", "0px 1px");
      await page.mouse.move(0, 0);
      await page.mouse.up();
      await expect(danger).toHaveCSS("transform", "none");
      await expect(page.getByRole("dialog")).toHaveCount(0);
    });
  }

  for (const width of [390, 1440]) {
    for (const theme of ["light", "dark"]) {
      test(`frosted surfaces and opaque fallbacks ${theme} ${width}px`, async ({ page }) => {
        await page.setViewportSize({ width, height: 900 });
        await page.emulateMedia({ reducedMotion: "no-preference", colorScheme: theme });
        await page.goto(`/desk.html?theme=${theme}#health`);
        await expect(page.locator(".health-verdict")).toBeVisible();
        await expect(page.locator(".desk-toolbar")).toHaveCSS("backdrop-filter", /blur\(/);
        for (const selector of [".health-verdict", ".health-analysis"]) await expect(page.locator(selector)).toHaveCSS("backdrop-filter", "none");
        // The header's glass sits on a ::before layer, not on the header
        // itself: backdrop-filter on the header would make it the containing
        // block for the fixed mobile nav island and pin it to the top bar.
        const appbarGlass = await page.evaluate(() => {
          const style = getComputedStyle(document.querySelector(".desk-appbar"), "::before");
          const [, , , a] = style.backgroundColor.match(/[\d.]+/g) ?? [];
          return { filter: style.backdropFilter, alpha: a === undefined ? 1 : Number(a) };
        });
        expect(appbarGlass.filter).toBe("blur(24px) saturate(1.45)");
        expect(appbarGlass.alpha).toBeLessThan(1);
        if (width === 390) {
          await expect(page.locator(".desk-navigation")).toHaveCSS("backdrop-filter", "blur(24px) saturate(1.45)");
          const navigation = await page.locator(".desk-navigation").boundingBox();
          // The island floats: inset from the screen edges and above the
          // bottom, as a rounded pill with content scrolling underneath.
          expect(Math.round(navigation.y + navigation.height)).toBeLessThan(900);
          expect(navigation.x).toBeGreaterThan(0);
          await expect(page.locator(".desk-navigation")).toHaveCSS("border-radius", "999px");
          const settings = page.getByRole("button", { name: "Settings", exact: true });
          await settings.click();
          await expect(page.locator("#workspace h1")).toHaveText("Settings");
          await page.getByRole("link", { name: "Health", exact: true }).click();
        }
        await page.getByRole("button", { name: "Log pain", exact: true }).click();
        const dialog = page.getByRole("dialog", { name: "Log pain" });
        await expect(dialog).toHaveCSS("backdrop-filter", "blur(24px) saturate(1.45)");
        await expect.poll(() => dialog.evaluate(element => getComputedStyle(element, "::backdrop").backdropFilter)).toBe("blur(12px)");
        // Axe samples the painted pixels: measuring while the dialog is still
        // fading in blends its colours with the page behind and invents a
        // contrast failure that never exists once the arrival animation ends.
        await expect.poll(() => page.evaluate(() => document.getAnimations().filter(animation => animation.playState === "running").length)).toBe(0);
        expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
        await page.screenshot({ path: `test-results/glass-${theme}-${width}.png`, animations: "disabled" });
        await page.emulateMedia({ contrast: "more" });
        await expect(dialog).toHaveCSS("backdrop-filter", "none");
        await expect.poll(() => dialog.evaluate(element => getComputedStyle(element, "::backdrop").backdropFilter)).toBe("none");
        for (const selector of [".desk-toolbar", ".health-verdict", ".health-analysis"]) await expect(page.locator(selector)).toHaveCSS("backdrop-filter", "none");
        const surface = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--desk-surface").trim());
        const opaqueColor = await page.evaluate(value => { const probe = document.createElement("span"); probe.style.color = value; document.body.append(probe); const color = getComputedStyle(probe).color; probe.remove(); return color; }, surface);
        await expect(dialog).toHaveCSS("background-color", opaqueColor);
        await page.emulateMedia({ contrast: "no-preference" });
        const session = await page.context().newCDPSession(page);
        await session.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-transparency", value: "reduce" }] });
        await expect(dialog).toHaveCSS("backdrop-filter", "none");
        await expect(dialog).toHaveCSS("background-color", opaqueColor);
        await session.send("Emulation.setEmulatedMedia", { features: [] });
        await session.detach();
        const removed = await page.evaluate(() => {
          let count = 0;
          for (const sheet of document.styleSheets) {
            for (let index = sheet.cssRules.length - 1; index >= 0; index--) {
              const rule = sheet.cssRules[index];
              if (rule instanceof CSSSupportsRule && rule.conditionText.includes("backdrop-filter")) { sheet.deleteRule(index); count++; }
            }
          }
          return count;
        });
        expect(removed).toBeGreaterThan(0);
        await expect(dialog).toHaveCSS("backdrop-filter", "none");
        await expect(dialog).toHaveCSS("background-color", opaqueColor);
        await page.keyboard.press("Escape");
        await expect(dialog).toHaveCount(0);
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      });
    }
  }

  for (const width of [390, 1440]) {
    for (const reducedMotion of ["no-preference", "reduce"]) {
      test(`semantic reveals and compact presses ${reducedMotion} ${width}px`, async ({ page }) => {
        await page.setViewportSize({ width, height: 900 });
        await page.emulateMedia({ reducedMotion });
        await page.addInitScript(() => {
          window.__contentReveals = [];
          window.__motionSamples = [];
          new MutationObserver(records => {
            for (const record of records) {
              const element = record.target;
              if (element.dataset.reveal !== "visible" && record.oldValue !== "visible") continue;
              const style = getComputedStyle(element);
              window.__contentReveals.push({ type: record.oldValue === "visible" ? "end" : "start", className: element.className, width: element.offsetWidth, height: element.offsetHeight, time: performance.now(), delay: parseFloat(element.style.getPropertyValue("--reveal-delay")), transform: style.transform, filter: style.filter });
              if (record.oldValue !== "visible") {
                const sample = () => {
                  if (!element.isConnected || element.dataset.reveal !== "visible") return;
                  window.__motionSamples.push({ className: element.className, transform: getComputedStyle(element).transform });
                  requestAnimationFrame(sample);
                };
                requestAnimationFrame(sample);
              }
            }
          }).observe(document, { subtree: true, attributes: true, attributeFilter: ["data-reveal"], attributeOldValue: true });
        });
        await page.goto("/desk.html#health/overview");
        const verdict = page.locator(".health-verdict");
        await expect(verdict).toBeVisible();
        if (reducedMotion === "no-preference") {
          await expect.poll(() => page.evaluate(() => window.__contentReveals.filter(event => event.className === "health-verdict" && event.type === "end").length)).toBe(1);
          const events = await page.evaluate(() => window.__contentReveals.filter(event => event.className === "health-verdict"));
          expect(events[1].width).toBeCloseTo(events[0].width, 3);
          expect(events[1].height).toBeCloseTo(events[0].height, 3);
          expect(events[1].time - events[0].time).toBeLessThan(events[0].delay * 1000 + 700);
          expect(events.every(event => event.filter === "none")).toBe(true);
          expect(await page.evaluate(() => window.__motionSamples.some(sample => sample.className === "health-verdict" && sample.transform !== "none"))).toBe(true);
          await expect(verdict).toHaveAttribute("data-reveal", "settled");
          await expect(verdict).toHaveCSS("filter", "none");
          await verdict.hover();
          await page.mouse.move(0, 0);
          await expect(verdict).toHaveCSS("animation-name", "none");
        } else {
          expect(await page.evaluate(() => window.__contentReveals)).toEqual([]);
          await expect(verdict).toHaveCSS("animation-name", "none");
        }
        await page.getByRole("link", { name: "Pace insights", exact: true }).click();
        const selector = page.getByRole("button", { name: "Long", exact: true });
        await expect(selector).toBeVisible();
        const bounds = await selector.boundingBox();
        await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
        await page.mouse.down();
        await expect.poll(() => selector.evaluate(element => getComputedStyle(element).scale)).toBe(reducedMotion === "reduce" ? "1" : "0.98");
        expect(await selector.evaluate(element => element.offsetWidth)).toBe(Math.round(bounds.width));
        await page.mouse.up();
        await expect(selector).toHaveCSS("scale", reducedMotion === "reduce" ? "none" : "1");
        await page.mouse.move(0, 0);
        await expect(selector).toHaveCSS("scale", "none");
        await expect(selector).toHaveAttribute("aria-pressed", "true");
        await page.goto("/desk.html#settings");
        await page.getByRole("link", { name: /Offline maps/ }).click();
        const editor = page.locator(".settings-editor");
        await expect(editor).toBeVisible();
        if (reducedMotion === "no-preference") {
          await expect.poll(() => editor.locator("header").getAttribute("data-reveal")).toBe("settled");
        }
        const before = await page.evaluate(() => window.__contentReveals.length);
        await page.getByLabel("Raster PMTiles archive").fill("/synthetic/new-map.pmtiles");
        expect(await page.evaluate(() => window.__contentReveals.length)).toBe(before);
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
        await page.screenshot({ path: `test-results/micro-motion-${reducedMotion}-${width}.png`, fullPage: true });
      });
    }
  }

  for (const width of [390, 1440]) {
    test(`Motion unlock is bounded, staggered and one-shot ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.emulateMedia({ reducedMotion: "no-preference" });
      await page.addInitScript(() => {
        window.__unlockEvents = [];
        new MutationObserver(records => {
          for (const record of records) {
            const element = record.target;
            if (element.dataset.entranceKind !== "unlock") continue;
            const state = element.dataset.reveal;
            if (state === "visible" || record.oldValue === "visible") window.__unlockEvents.push({ className: element.className, shell: Boolean(element.closest(".desk-appbar")), state, time: performance.now(), delay: parseFloat(element.style.getPropertyValue("--reveal-delay")) });
          }
        }).observe(document, { subtree: true, attributes: true, attributeFilter: ["data-reveal"], attributeOldValue: true });
      });
      await page.goto("/desk.html#health/overview");
      await expect(page.locator(".health-verdict")).toHaveAttribute("data-reveal", "settled");
      await expect(page.locator('[data-reveal="visible"]')).toHaveCount(0);
      const events = await page.evaluate(() => window.__unlockEvents);
      const starts = events.filter(event => event.state === "visible");
      const ends = events.filter(event => event.state === "settled");
      expect(starts.length).toBeGreaterThan(5);
      expect(new Set(starts.map(event => event.delay)).size).toBeGreaterThan(2);
      expect(ends.length).toBe(starts.length);
      const shellArrival = Math.max(...starts.filter(event => event.shell).map(event => event.time + event.delay * 1000));
      const contentArrival = Math.min(...starts.filter(event => !event.shell).map(event => event.time + event.delay * 1000));
      expect(contentArrival).toBeGreaterThanOrEqual(shellArrival - 25);
      expect(Math.max(...ends.map(event => event.time)) - Math.min(...starts.map(event => event.time))).toBeLessThan(Math.max(...starts.map(event => event.delay)) * 1000 + 700);
      const brandEvents = events.filter(event => event.className === "desk-brand-name").length;
      await page.getByRole("link", { name: "Activities", exact: true }).click();
      await expect(page.locator(".activity-row")).toHaveCount(9);
      expect(await page.evaluate(() => window.__unlockEvents.filter(event => event.className === "desk-brand-name").length)).toBe(brandEvents);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    });
  }

  for (const width of [390, 1440]) {
    test(`soft view landings replay on return without moving navigation ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.emulateMedia({ reducedMotion: "no-preference" });
      await page.goto("/desk.html#health/overview");
      await expect(page.locator(".health-verdict")).toHaveAttribute("data-reveal", "settled");
      const navigation = await page.locator(".desk-navigation").boundingBox();
      await page.getByRole("link", { name: "Pace insights", exact: true }).click();
      await expect(page.getByRole("button", { name: "Long", exact: true })).toBeVisible();
      await page.getByRole("link", { name: "Overview", exact: true }).click();
      const landing = page.locator('.health-verdict[data-entrance-kind="component"]');
      await expect(landing).toHaveAttribute("data-reveal", "visible");
      await expect(landing).toHaveAttribute("data-reveal", "settled");
      expect(await page.locator(".desk-navigation").boundingBox()).toEqual(navigation);
      await page.getByRole("link", { name: "Activities", exact: true }).click();
      await expect(page.locator(".activity-row")).toHaveCount(9);
      await page.getByRole("link", { name: "Health", exact: true }).click();
      await expect(page.locator('.health-verdict[data-entrance-kind="component"]')).toHaveAttribute("data-reveal", "settled");
      expect(await page.evaluate(() => document.querySelectorAll('[data-reveal]:not([data-reveal="container"]) [data-reveal]:not([data-reveal="container"])').length)).toBe(0);
    });
  }

  test("Motion desktop exits retain outgoing content and cascade vertically in DOM order", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto("/desk.html#health");
    await expect(page.locator(".health-verdict")).toHaveAttribute("data-reveal", "settled");
    const frames = await page.getByRole("link", { name: "Activities", exact: true }).evaluate(async link => {
      link.click();
      const frames = [];
      for (let index = 0; index < 30; index++) {
        await new Promise(requestAnimationFrame);
        const outgoing = document.querySelector('.desk-workspace > .motion-change[data-transition-state="exiting"]');
        if (!outgoing) continue;
        const style = getComputedStyle(outgoing);
        const verdict = outgoing.querySelector(".health-verdict");
        const leaves = [...outgoing.querySelectorAll('[data-reveal="settled"]')].filter(element => element.getAnimations().length);
        frames.push({ connected: outgoing.isConnected, inert: outgoing.inert, transform: style.transform, filter: style.filter, y: verdict ? new DOMMatrixReadOnly(getComputedStyle(verdict).transform).m42 : 0, delays: leaves.map(element => element.getAnimations()[0].effect.getTiming().delay) });
      }
      return frames;
    });
    expect(frames.length).toBeGreaterThan(1);
    expect(frames.every(frame => frame.connected && frame.inert && frame.transform === "none" && frame.filter === "none")).toBe(true);
    expect(frames.some(frame => frame.y < -1)).toBe(true);
    const delays = frames.find(frame => frame.delays.length > 1)?.delays;
    expect(delays?.length).toBeGreaterThan(1);
    expect(delays).toEqual([...delays].sort((first, second) => first - second));
    await expect(page.locator('[data-transition-state="exiting"]')).toHaveCount(0);
    await expect(page.locator(".activity-row")).toHaveCount(9);
  });

  test("Motion preference changes settle active entrances without replay", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto("/desk.html#health");
    await expect(page.locator('[data-reveal="visible"]').first()).toBeAttached();
    await page.emulateMedia({ reducedMotion: "reduce" });
    await expect(page.locator('[data-reveal="visible"], [data-reveal="waiting"]')).toHaveCount(0);
    await expect(page.locator(".health-verdict")).toHaveCSS("transform", "none");
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await expect(page.locator('[data-reveal="visible"], [data-reveal="waiting"]')).toHaveCount(0);
    await page.getByRole("link", { name: "Pace insights", exact: true }).click();
    await expect(page.getByRole("button", { name: "Long", exact: true })).toBeVisible();
  });

  test("Motion Bezier choreography uses exact DOM order without jitter or scale", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto("/desk.html#health");
    await expect(page.locator('.health-verdict')).toHaveAttribute("data-reveal", "settled");
    await page.evaluate(() => {
      const root = document.createElement("section");
      root.className = "motion-change";
      root.id = "choreography-probe";
      for (const name of ["Heading", "Evidence", "Actions"]) {
        const element = document.createElement("div");
        element.dataset.entrance = "";
        element.textContent = name;
        root.append(element);
      }
      document.querySelector("#workspace").prepend(root);
    });
    const leaves = page.locator("#choreography-probe > div");
    await expect.poll(() => leaves.evaluateAll(elements => elements.filter(element => element.getAnimations().length).length)).toBe(3);
    const effects = await leaves.evaluateAll(elements => elements.map(element => {
      const effect = element.getAnimations().find(animation => animation.effect.getKeyframes().some(frame => frame.transform))?.effect;
      const transform = new DOMMatrixReadOnly(effect.getKeyframes()[0].transform);
      return { delay: effect.getTiming().delay, duration: effect.getTiming().duration, easing: effect.getTiming().easing, x: transform.m41, y: transform.m42, scale: transform.m11 };
    }));
    expect(effects.map(effect => effect.delay)).toEqual([0, 60, 120]);
    for (const effect of effects) expect(effect).toMatchObject({ duration: 560, easing: "cubic-bezier(0.22, 1, 0.36, 1)", x: 0, y: 14, scale: 1 });
    await expect(leaves.last()).toHaveAttribute("data-reveal", "settled");
    expect(await leaves.evaluateAll(elements => elements.every(element => element.getAnimations().length === 0 && element.style.transform === ""))).toBe(true);
  });

  test("Motion activity removals retain their DOM until exit completes", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto("/desk.html#journal");
    await expect(page.locator(".activity-row")).toHaveCount(9);
    await expect(page.locator(".activity-row").last()).toHaveCSS("opacity", "1");
    const samples = await page.evaluate(async () => {
      const removed = document.querySelector(".activity-row");
      const moving = removed.nextElementSibling;
      const start = moving.getBoundingClientRect().y;
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        const result = await invoke(command, args);
        return command === "all_activities" ? result.slice(1) : result;
      };
      window.__deskEmit("sync://completed", { ok: true });
      const samples = [];
      for (let index = 0; index < 60; index++) {
        await new Promise(requestAnimationFrame);
        samples.push({ connected: removed.isConnected, opacity: Number(getComputedStyle(removed).opacity), y: moving.getBoundingClientRect().y, start });
      }
      return samples;
    });
    expect(samples.some(sample => sample.connected && sample.opacity > 0 && sample.opacity < .95)).toBe(true);
    expect(samples.at(-1).connected).toBe(false);
    expect(samples.some(sample => sample.y < sample.start - 1 && sample.y > samples.at(-1).y + 1)).toBe(true);
    expect(samples.every(sample => sample.y >= samples.at(-1).y - 1 && sample.y <= sample.start + 1)).toBe(true);
    await expect(page.locator(".activity-row")).toHaveCount(8);
  });

  test("viewport reveals wait for entry, run once, and settle on focus or preference changes", async ({ page }) => {
    await page.setViewportSize({ width: 820, height: 600 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto("/desk.html#settings");
    const lastTask = page.locator(".settings-task-list > a").last();
    await expect(lastTask).toHaveAttribute("data-reveal", "waiting");
    await expect(lastTask).toHaveCSS("opacity", "0");
    await lastTask.evaluate(element => {
      window.__viewportReveals = [];
      new MutationObserver(records => {
        for (const record of records) if (record.oldValue === "visible") window.__viewportReveals.push("settled");
      }).observe(element, { attributes: true, attributeFilter: ["data-reveal"], attributeOldValue: true });
      element.scrollIntoView({ block: "center" });
    });
    await expect(lastTask).toHaveAttribute("data-reveal", "settled");
    await expect(lastTask).toHaveCSS("filter", "none");
    await expect(lastTask).toHaveCSS("opacity", "1");
    await page.locator("#workspace").evaluate(element => element.scrollTo(0, 0));
    await expect(page.locator(".workspace-heading")).toBeInViewport();
    expect(await page.evaluate(() => document.documentElement.scrollHeight <= innerHeight && scrollY === 0)).toBe(true);
    await lastTask.evaluate(element => element.scrollIntoView({ block: "center" }));
    expect(await page.evaluate(() => window.__viewportReveals)).toEqual(["settled"]);

    await page.locator("#workspace").evaluate(element => element.scrollTo(0, 0));
    await page.reload();
    await expect(lastTask).toHaveAttribute("data-reveal", "waiting");
    await lastTask.evaluate(element => element.focus({ preventScroll: true }));
    await expect(lastTask).toBeFocused();
    await expect(lastTask).toHaveAttribute("data-reveal", "settled");
    await expect(lastTask).toHaveCSS("opacity", "1");
    await expect(lastTask).toHaveCSS("filter", "none");
    await expect(lastTask).toHaveCSS("animation-name", "none");

    await page.locator("#workspace").evaluate(element => element.scrollTo(0, 0));
    await page.reload();
    await expect(lastTask).toHaveAttribute("data-reveal", "waiting");
    await page.emulateMedia({ reducedMotion: "reduce" });
    await expect(page.locator('[data-reveal="waiting"], [data-reveal="visible"]')).toHaveCount(0);
    await expect(lastTask).toHaveCSS("opacity", "1");
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await lastTask.evaluate(element => element.scrollIntoView({ block: "center" }));
    await expect(lastTask).toHaveCSS("animation-name", "none");
    expect(await page.evaluate(() => document.querySelectorAll('[data-reveal]:not([data-reveal="container"]) [data-reveal]:not([data-reveal="container"])').length)).toBe(0);
  });

  for (const reducedMotion of ["no-preference", "reduce"]) {
    test(`scatter exits retain content without transforming fixed ancestors ${reducedMotion}`, async ({ page }) => {
      await page.setViewportSize({ width: 390, height: 900 });
      await page.emulateMedia({ reducedMotion });
      await page.addInitScript(() => {
        window.__scatterExits = [];
        document.addEventListener("animationstart", event => {
          if (event.animationName !== "content-depart") return;
          const host = event.target.closest('.motion-change[data-transition-state="exiting"]');
          if (!host) return;
          const effect = event.target.getAnimations().find(animation => animation.animationName === "content-depart")?.effect;
          window.__scatterExits.push({ connected: host.isConnected, inert: host.inert, filter: getComputedStyle(host).filter, transform: getComputedStyle(host).transform, frames: effect?.getKeyframes() });
        });
      });
      await page.goto("/desk.html#health");
      await expect(page.locator(".health-verdict")).toHaveAttribute("data-reveal", "settled");
      await page.getByRole("link", { name: "Activities", exact: true }).click();
      await expect(page.locator(".activity-library")).toBeVisible();
      await expect(page.locator('[data-transition-state="exiting"]')).toHaveCount(0);
      const exits = await page.evaluate(() => window.__scatterExits);
      if (reducedMotion === "reduce" || await page.evaluate(() => matchMedia("(max-width: 800px)").matches)) {
        expect(exits).toEqual([]);
      } else {
        expect(exits.length).toBeGreaterThan(1);
        for (const exit of exits) {
          expect(exit).toMatchObject({ connected: true, inert: true, filter: "none", transform: "none" });
          expect(exit.frames.at(-1).filter).toBe("blur(6px)");
        }
        expect(new Set(exits.map(exit => exit.frames.at(-1).translate)).size).toBeGreaterThan(1);
      }
      const navigation = await page.locator(".desk-navigation").boundingBox();
      expect(navigation.x + navigation.width / 2).toBeCloseTo(195, 1);
      const bottom = await page.locator(".desk-navigation").evaluate(element => parseFloat(getComputedStyle(element).bottom));
      expect(navigation.y + navigation.height).toBeCloseTo(900 - bottom, 1);
    });
  }

  for (const width of [390, 1440]) {
    for (const theme of ["light", "dark"]) {
      test(`complete motion-enabled workspace pass ${theme} ${width}px`, async ({ page }) => {
        test.setTimeout(90000);
        await page.setViewportSize({ width, height: 1000 });
        await page.emulateMedia({ reducedMotion: "no-preference" });
        const errors = [];
        page.on("pageerror", error => errors.push(error.message));
        for (const route of ["training", "training/month", "training/plan", "journal", "health", "health/pace", "health/journal", "health/milestones", "settings", "settings/appearance", "help"]) {
          await page.goto(`/desk.html?theme=${theme}#${route}`);
          await expect(page.locator("#workspace h1")).toBeVisible();
          await expect(page.locator(".desk-loading")).toHaveCount(0);
          expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), route).toBe(true);
          // axe must sample settled colours: with animations left running it
          // catches reveals and the sliding selection marker mid-flight and
          // reports blended colours that no user ever sees. Reveals are
          // staggered, so a single zero-count moment can sit in a gap between
          // two siblings: require the count to stay at zero for 500ms.
          await page.waitForFunction(() => {
            if (document.getAnimations().length > 0) { window.__quiet = 0; return false; }
            window.__quiet = (window.__quiet || 0) + 1;
            return window.__quiet >= 5;
          }, null, { timeout: 20000, polling: 100 });
          expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations, route).toEqual([]);
          await page.screenshot({ path: `test-results/motion-${route.replaceAll("/", "-")}-${theme}-${width}.png`, fullPage: true, animations: "disabled" });
        }
        await page.goto(`/desk.html?theme=${theme}#health/journal`);
        await page.getByRole("button", { name: "Log pain", exact: true }).click();
        await expect(page.getByRole("dialog")).toBeVisible();
        await page.screenshot({ path: `test-results/motion-sheet-${theme}-${width}.png`, animations: "disabled" });
        await page.keyboard.press("Escape");
        await expect(page.getByRole("dialog")).toHaveCount(0);
        await page.getByRole("link", { name: "Training", exact: true }).click();
        if (width < 800) await page.locator(".mobile-date-ribbon button").first().click();
        await page.locator(width < 800 ? ".day-itinerary .session-tile" : ".week-board .session-tile").first().click();
        await expect(page.locator(".desk-inspector")).toBeVisible();
        await page.screenshot({ path: `test-results/motion-inspector-${theme}-${width}.png`, animations: "disabled" });
        await page.keyboard.press("Escape");
        await expect(page.locator(".desk-inspector")).toHaveCount(0);
        expect(errors).toEqual([]);
      });
    }
  }

  test("selection continuity, tooltips and resize remain interruptible", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto("/desk.html#training");
    const navigation = page.getByRole("navigation", { name: "Workspaces" });
    for (const workspace of ["Activities", "Health", "Training", "Health"]) {
      await navigation.getByRole("link", { name: workspace, exact: true }).click();
      await expect(page.locator("#workspace h1")).toHaveText(workspace);
    }
    const active = navigation.locator("a[aria-current]");
    await expect.poll(async () => {
      const link = await active.boundingBox();
      const marker = await active.locator(".selection-marker").boundingBox();
      const border = await active.evaluate(element => parseFloat(getComputedStyle(element).borderLeftWidth) * (parseFloat(getComputedStyle(element).scale) || 1));
      return Math.abs(link.x + border - marker.x) + Math.abs(link.width - 2 * border - marker.width);
    }).toBeLessThan(1);
    const help = page.getByRole("button", { name: "Help", exact: true });
    await help.hover();
    await expect(help.locator(".control-tooltip")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(help.locator(".control-tooltip")).toBeHidden();
    await page.mouse.move(100, 200);
    await expect(help.locator(".control-tooltip")).toBeHidden();
    await help.focus();
    await page.keyboard.press("Tab");
    await page.keyboard.press("Shift+Tab");
    await expect(help).toBeFocused();
    await expect(help.locator(".control-tooltip")).toBeVisible();
    await navigation.getByRole("link", { name: "Training", exact: true }).click();
    await page.locator(".week-board .session-tile").first().click();
    await expect(page.locator(".desk-inspector")).toBeVisible();
    await page.setViewportSize({ width: 390, height: 844 });
    await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe("hidden");
    await page.emulateMedia({ reducedMotion: "reduce" });
    await expect.poll(async () => Math.round((await page.locator(".desk-inspector").boundingBox()).x)).toBe(0);
    await page.keyboard.press("Escape");
    await expect(page.locator(".desk-inspector")).toHaveCount(0);
    expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
  });

  test("buttons preserve geometry through pending, success, error and keyboard retry", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.addInitScript(() => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        if (command === "save_settings") await new Promise(resolve => { window.__releaseMotionAction = resolve; });
        return invoke(command, args);
      };
    });
    await page.goto("/desk.html#settings/maps");
    const save = page.getByRole("button", { name: "Save map source", exact: true });
    await expect(save).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    const layout = () => save.evaluate(element => ({ width: element.offsetWidth, height: element.offsetHeight }));
    const resting = await layout();
    await save.click();
    await expect(save).toHaveAttribute("aria-busy", "true");
    await expect(save).toBeDisabled();
    expect(await layout()).toEqual(resting);
    await page.evaluate(() => window.__releaseMotionAction());
    await expect(save).toHaveAttribute("data-result", "success");
    await expect(save).toBeEnabled();
    expect(await layout()).toEqual(resting);
    await expect(page.getByRole("status")).toContainText("Map source saved");
    await page.evaluate(() => { window.__deskFail = "save_settings"; });
    await page.getByLabel("Raster PMTiles archive").fill("/synthetic/map.pmtiles");
    await page.getByLabel("Raster PMTiles archive").press("Enter");
    await expect(save).toHaveAttribute("aria-busy", "true");
    await page.evaluate(() => window.__releaseMotionAction());
    await expect(save).toHaveAttribute("data-result", "error");
    await expect(page.getByRole("alert")).toContainText("Unable to save");
    await expect(page.getByLabel("Raster PMTiles archive")).toHaveValue("/synthetic/map.pmtiles");
    expect(await layout()).toEqual(resting);
    await page.evaluate(() => { window.__deskFail = ""; });
    await save.press("Enter");
    await page.evaluate(() => window.__releaseMotionAction());
    await expect(save).toHaveAttribute("data-result", "success");
    const notice = page.locator(".desk-notice");
    await page.getByRole("button", { name: "Dismiss notification" }).evaluate(button => button.click());
    await expect(notice).toBeAttached();
    await expect(notice).toHaveCount(0);
  });

  for (const width of [390, 1440]) {
    test(`overlays retain their exit, focus and scroll ownership at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.emulateMedia({ reducedMotion: "no-preference" });
      await page.goto("/desk.html#health/pace");
      const opener = page.getByRole("button", { name: "Log pain", exact: true });
      await opener.click();
      const dialog = page.getByRole("dialog", { name: "Log pain" });
      await expect(dialog).toBeVisible();
      await expect.poll(async () => Math.round((await dialog.boundingBox()).x)).toBe(width === 390 ? 0 : 420);
      await page.getByLabel("Notes", { exact: true }).fill("Retained while the surface exits");
      await page.getByRole("button", { name: "Cancel", exact: true }).evaluate(button => button.click());
      await expect(dialog).toHaveAttribute("data-state", "closing");
      await expect(dialog).toContainText("Save pain entry");
      expect(await page.evaluate(() => document.body.style.overflow)).toBe("hidden");
      await expect(dialog).toHaveCount(0);
      await expect(opener).toBeFocused();
      expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
      await opener.click();
      await page.keyboard.press("Escape");
      await expect(dialog).toHaveCount(0);
      await expect(opener).toBeFocused();
      expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
    });
  }

  test("touch surfaces track, cancel and dismiss without hijacking form controls", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.emulateMedia({ reducedMotion: "no-preference" });
    const session = await page.context().newCDPSession(page);
    await session.send("Emulation.setTouchEmulationEnabled", { enabled: true });
    const frame = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(resolve)));
    const touch = (type, x, y) => session.send("Input.dispatchTouchEvent", { type, touchPoints: type === "touchEnd" || type === "touchCancel" ? [] : [{ x, y }] });
    await page.goto("/desk.html#health/pace");
    const opener = page.getByRole("button", { name: "Log pain", exact: true });
    await opener.click();
    const dialog = page.locator(".desk-dialog");
    const grip = page.locator(".dialog-grip");
    await expect.poll(async () => Math.round((await dialog.boundingBox()).y + (await dialog.boundingBox()).height)).toBe(844);
    const rest = await dialog.boundingBox();
    const handle = await grip.boundingBox();
    const start = { x: handle.x + handle.width / 2, y: handle.y + handle.height / 2 };
    await touch("touchStart", start.x, start.y);
    await touch("touchMove", start.x, start.y + 12);
    await frame();
    await touch("touchMove", start.x, start.y + 48);
    await frame();
    expect((await dialog.boundingBox()).y - rest.y).toBeGreaterThan(20);
    await touch("touchCancel", start.x, start.y + 48);
    await expect(dialog).toHaveCount(1);
    await expect.poll(async () => Math.round((await dialog.boundingBox()).y)).toBe(Math.round(rest.y));
    await page.getByLabel("Notes", { exact: true }).fill("Touch input still works");
    await touch("touchStart", start.x, start.y);
    await touch("touchMove", start.x, start.y + 16);
    await frame();
    await touch("touchMove", start.x, start.y + 150);
    await frame();
    expect((await dialog.boundingBox()).y - rest.y).toBeGreaterThan(60);
    await touch("touchEnd", start.x, start.y + 150);
    await expect(dialog).toHaveCount(0);
    await expect(opener).toBeFocused();
    await page.goto("/desk.html#training");
    await page.locator(".mobile-date-ribbon button").first().click();
    const tile = page.locator(".day-itinerary .session-tile").first();
    await tile.click();
    const inspector = page.locator(".desk-inspector");
    await expect.poll(async () => Math.round((await inspector.boundingBox()).x)).toBe(0);
    const bar = await page.locator(".inspector-bar > span").boundingBox();
    const origin = { x: bar.x + bar.width / 2, y: bar.y + bar.height / 2 };
    await touch("touchStart", origin.x, origin.y);
    await touch("touchMove", origin.x + 16, origin.y);
    await frame();
    await touch("touchMove", origin.x + 140, origin.y);
    await frame();
    expect((await inspector.boundingBox()).x).toBeGreaterThan(40);
    await touch("touchEnd", origin.x + 140, origin.y);
    await expect(inspector).toHaveCount(0);
    await expect(tile).toBeFocused();
    expect(await page.evaluate(() => document.body.style.overflow)).toBe("");
    await session.detach();
  });

  test("reduced motion keeps feedback and keyboard interaction without spatial animation", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/desk.html#health/journal");
    await page.getByRole("button", { name: "Log pain", exact: true }).click();
    const dialog = page.getByRole("dialog");
    const geometry = await dialog.boundingBox();
    expect(Math.round(geometry.x)).toBe(0);
    expect(Math.round(geometry.y + geometry.height)).toBe(844);
    expect(await dialog.evaluate(element => getComputedStyle(element).transform)).toBe("none");
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await page.getByRole("link", { name: "Pace insights", exact: true }).click();
    await page.getByRole("button", { name: "Long", exact: true }).focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("button", { name: "Long", exact: true })).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByRole("region", { name: "Selected pace session", exact: true })).toContainText("Race-pace long run");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  });
});

for (const width of [390, 1440]) {
  test(`map theme overrides preserve contrasting routes and circular finish at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    const tile = await page.evaluate(() => {
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 256;
      const context = canvas.getContext("2d");
      context.fillStyle = "#e5e7eb";
      context.fillRect(0, 0, 256, 256);
      return canvas.toDataURL().split(",")[1];
    });
    await page.route("https://tile.openstreetmap.org/**", route => route.fulfill({ contentType: "image/png", body: Buffer.from(tile, "base64") }));
    await page.route("**/src/desk/map.tsx*", async route => {
      const response = await route.fetch();
      await route.fulfill({ response, body: `${await response.text()}
        const originalRender = maplibregl.Map.prototype._render;
        maplibregl.Map.prototype._render = function(...args) {
          if (!window.__initialMapStyle) window.__initialMapStyle = this.getStyle();
          return originalRender.apply(this, args);
        };
        const originalPaint = maplibregl.Map.prototype.setPaintProperty;
        maplibregl.Map.prototype.setPaintProperty = function(...args) {
          window.__themeMap = this;
          return originalPaint.apply(this, args);
        };
      ` });
    });
    await page.goto("/desk.html#journal/1");
    const finish = page.getByRole("img", { name: "Route finish", exact: true });
    await expect(finish).toBeVisible();
    const initialPaint = await page.evaluate(() => window.__initialMapStyle.layers.find(layer => layer.id === "base").paint);
    expect(initialPaint).toMatchObject({ "raster-saturation": -.85, "raster-brightness-min": .3, "raster-brightness-max": 1, "raster-opacity": .88 });
    expect(await page.evaluate(() => window.__initialMapStyle.layers[0].id)).toBe("map-background");
    const attribution = page.locator(".maplibregl-ctrl-attrib");
    const attributionToggle = attribution.locator("summary");
    await expect(attributionToggle).toHaveCSS("background-image", /url\("data:image\/svg\+xml/);
    await attributionToggle.hover();
    await expect(attributionToggle).toHaveCSS("background-image", /url\("data:image\/svg\+xml/);
    await attributionToggle.focus();
    await expect(attributionToggle).toHaveCSS("background-image", /url\("data:image\/svg\+xml/);
    await page.mouse.down();
    try {
      await expect(attributionToggle).toHaveCSS("background-image", /url\("data:image\/svg\+xml/);
    } finally {
      await page.mouse.up();
    }
    await expect(attribution).toHaveAttribute("open", "");
    await attributionToggle.click();
    await expect(attribution).not.toHaveAttribute("open", "");
    await attribution.locator("summary").click();
    await expect(attribution).toHaveAttribute("open", "");
    await expect(attribution.getByRole("link", { name: "OpenStreetMap contributors" })).toBeVisible();
    await attribution.locator("summary").click();
    await expect(attribution).not.toHaveAttribute("open", "");
    await expect(finish).toHaveCSS("border-radius", "50%");
    await expect(finish).toHaveCSS("width", "24px");
    await expect(finish).toHaveCSS("height", "24px");
    const start = page.getByRole("img", { name: "Route start", exact: true });
    await expect(start).toHaveCSS("width", "24px");
    await expect(start).toHaveCSS("height", "24px");
    await expect(finish.locator("svg")).toHaveCSS("clip-path", "circle(50%)");
    await expect(finish.locator(".flag-pole")).toHaveCount(0);
    await expect(finish.locator(".flag-check")).toHaveCount(1);
    await page.evaluate(() => { window.__originalThemeMap = window.__themeMap; });
    const camera = await page.evaluate(() => ({ center: window.__themeMap.getCenter().toArray(), zoom: window.__themeMap.getZoom() }));
    for (const theme of ["light", "dark"]) {
      for (const palette of ["reference", "amethyst", "glacier", "evergreen", "sakura", "ember"]) {
        await page.evaluate(({ theme, palette }) => {
          document.documentElement.dataset.deskTheme = theme;
          document.documentElement.dataset.deskPalette = palette;
        }, { theme, palette });
        await expect.poll(() => page.evaluate(() => window.__themeMap.getPaintProperty("base", "raster-brightness-max"))).toBe(theme === "dark" ? .32 : 1);
        const colors = await page.evaluate(() => {
          const instance = window.__themeMap;
          const route = instance.getPaintProperty("route", "line-color");
          const outline = instance.getPaintProperty("route-outline", "line-color");
          const luminance = hex => {
            const full = hex.length === 4 ? `#${[...hex.slice(1)].map(channel => channel + channel).join("")}` : hex;
            return [1, 3, 5].map(offset => parseInt(full.slice(offset, offset + 2), 16) / 255).map(channel => channel <= .04045 ? channel / 12.92 : ((channel + .055) / 1.055) ** 2.4).reduce((total, channel, index) => total + channel * [.2126, .7152, .0722][index], 0);
          };
          const values = [luminance(route), luminance(outline)].sort((first, second) => second - first);
          return { ratio: (values[0] + .05) / (values[1] + .05), route, action: getComputedStyle(document.documentElement).getPropertyValue("--desk-action").trim(), gradient: instance.getPaintProperty("route", "line-gradient"), same: instance === window.__originalThemeMap, center: instance.getCenter().toArray(), zoom: instance.getZoom() };
        });
        expect(colors.route).toBe(colors.action);
        expect(colors.gradient).toEqual(["to-color", colors.action]);
        expect(colors.ratio, `${theme}/${palette}`).toBeGreaterThan(3);
        expect(colors.same).toBe(true);
        expect(colors.center).toEqual(camera.center);
        expect(colors.zoom).toBe(camera.zoom);
      }
      await expect.poll(() => page.locator(".maplibregl-canvas").evaluate(canvas => {
        const context = canvas.getContext("webgl2");
        const pixel = new Uint8Array(4);
        context.readPixels(10, 10, 1, 1, context.RGBA, context.UNSIGNED_BYTE, pixel);
        return pixel[3] === 255 && (document.documentElement.dataset.deskTheme === "dark" ? pixel[0] < 110 : pixel[0] > 180);
      })).toBe(true);
      await page.screenshot({ path: `test-results/map-theme-${theme}-${width}.png` });
    }
  });
}

for (const [width, pointCount, span] of [[390, 2, .001], [1440, 1001, .5]]) {
  test(`activity route draws with ease-in-out in three seconds at ${width}px with ${pointCount} GPS points`, async ({page}) => {
    await page.setViewportSize({width, height:1000});
    await page.emulateMedia({reducedMotion:'no-preference'});
    const tile = await page.evaluate(() => {
      const canvas = document.createElement('canvas');
      canvas.width = canvas.height = 256;
      const context = canvas.getContext('2d');
      context.fillStyle = '#e5e7eb';
      context.fillRect(0, 0, 256, 256);
      return canvas.toDataURL().split(',')[1];
    });
    await page.route('https://tile.openstreetmap.org/**', route => route.fulfill({status:200, contentType:'image/png', body:Buffer.from(tile, 'base64')}));
    await page.route('**/src/desk/map.tsx*', async route => {
      const response = await route.fetch();
      await route.fulfill({response, body:`${await response.text()}
        const originalSetPaint = maplibregl.Map.prototype.setPaintProperty;
        maplibregl.Map.prototype.setPaintProperty = function(layer, property, value, ...options) {
          const result = originalSetPaint.call(this, layer, property, value, ...options);
          if (layer === 'route' && property === 'line-gradient') {
            window.__routeTestMap = this;
            (window.__routeFrames ??= []).push({time:performance.now(), progress:value[0] === 'step' ? value[3] : 1});
          }
          return result;
        };
      `});
    });
    await page.addInitScript(({pointCount, span}) => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        const result = await invoke(command, args);
        return command === 'activity_review' ? {...result, track:Array.from({length:pointCount}, (_, index) => ({lat:38.72, lon:-9.14 + span * index / (pointCount - 1)}))} : result;
      };
    }, {pointCount, span});
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.clock.install();
    await page.goto('/desk.html#journal/1');
    await expect.poll(() => page.evaluate(() => window.__routeFrames?.length ?? 0)).toBeGreaterThan(1);
    await page.clock.pauseAt(await page.evaluate(() => Date.now() + 100));
    const elapsed = await page.evaluate(() => performance.now() - window.__routeFrames[0].time);
    expect(elapsed).toBeLessThan(1500);
    await page.clock.runFor(1500 - elapsed);
    const progress = () => page.evaluate(() => window.__routeFrames?.at(-1)?.progress);
    expect(await progress()).toBeCloseTo(.5, 1);
    expect(await page.evaluate(() => window.__routeTestMap.getPaintProperty('route-outline','line-gradient')[3])).toBeCloseTo(.5, 1);
    const pixels = () => page.locator('.maplibregl-canvas').evaluate(canvas => {
      const probe = document.createElement('canvas');
      probe.width = probe.height = 1;
      const paint = probe.getContext('2d');
      paint.fillStyle = getComputedStyle(document.documentElement).getPropertyValue('--desk-action').trim();
      paint.fillRect(0, 0, 1, 1);
      const accent = paint.getImageData(0, 0, 1, 1).data;
      const context = canvas.getContext('webgl2');
      const bytes = new Uint8Array(context.drawingBufferWidth * context.drawingBufferHeight * 4);
      context.readPixels(0, 0, context.drawingBufferWidth, context.drawingBufferHeight, context.RGBA, context.UNSIGNED_BYTE, bytes);
      const counts = {start:0, end:0};
      for (let index = 0; index < bytes.length; index += 4) {
        if (bytes[index + 3] > 240 && [0, 1, 2].every(channel => Math.abs(bytes[index + channel] - accent[channel]) < 20)) {
          counts[(index / 4) % context.drawingBufferWidth < context.drawingBufferWidth / 2 ? 'start' : 'end']++;
        }
      }
      return counts;
    });
    await expect.poll(async () => (await pixels()).start).toBeGreaterThan(50);
    const halfway = await pixels();
    expect(halfway.end).toBeLessThan(halfway.start / 4);
    await page.screenshot({path:`test-results/desk-route-half-${width}.png`});
    await page.clock.runFor(1400);
    expect(await progress()).toBeLessThan(1);
    await page.clock.runFor(150);
    expect(await progress()).toBe(1);
    const frames = await page.evaluate(() => window.__routeFrames);
    expect(frames[0].progress).toBe(0);
    expect(frames.at(-1).time - frames[0].time).toBeGreaterThanOrEqual(3000);
    expect(frames.at(-1).time - frames[0].time).toBeLessThan(3050);
    expect(frames.find(frame => frame.time - frames[0].time >= 750).progress).toBeCloseTo(.15625, 1);
    expect(frames.find(frame => frame.time - frames[0].time >= 2250).progress).toBeCloseTo(.84375, 1);
    expect(frames.every((frame, index) => index === 0 || frame.progress >= frames[index - 1].progress)).toBe(true);
    await expect.poll(async () => (await pixels()).end).toBeGreaterThan(halfway.start / 2);
    await page.screenshot({path:`test-results/desk-route-complete-${width}.png`});
    await page.getByRole('button',{name:'Back',exact:true}).click();
    await page.clock.runFor(250);
    await expect(page.locator('.maplibregl-canvas')).toHaveCount(0);
    await page.evaluate(() => {window.__routeFrames = [];});
    await page.clock.resume();
    await page.locator('.activity-row').first().click();
    await expect.poll(() => page.evaluate(() => window.__routeFrames.length)).toBeGreaterThan(1);
    expect(await progress()).toBeLessThan(.5);
    await page.getByRole('button',{name:'Back',exact:true}).click();
    await expect(page.locator('.maplibregl-canvas')).toHaveCount(0);
    const stopped = await page.evaluate(() => window.__routeFrames.length);
    await page.clock.runFor(3100);
    expect(await page.evaluate(() => window.__routeFrames.length)).toBe(stopped);
    await page.emulateMedia({reducedMotion:'reduce'});
    await page.evaluate(() => {window.__routeFrames = [];});
    await page.locator('.activity-row').first().click();
    await expect.poll(progress).toBe(1);
    expect(await page.evaluate(() => window.__routeFrames.map(frame => frame.progress))).toEqual([1]);
    await expect(page.locator('.route-figure figcaption')).toHaveCount(0);
    expect(errors).toEqual([]);
  });
}

test('graded performance proposals preview race estimates and only change them after approval', async ({page}) => {
  await page.goto('/desk.html?pacingProposal=1#health/pace');
  await page.getByRole('button',{name:'Review changes',exact:true}).click();
  const preview = page.getByRole('region',{name:'Proposed race estimates'});
  await expect(preview).toBeVisible();
  for (const distance of ['5 km','10 km']) {
    const cells = preview.getByRole('row').filter({has:page.getByRole('rowheader',{name:distance,exact:true})}).locator('td');
    for (const cell of await cells.all()) {
      const seconds = Number(await cell.getAttribute('data-seconds'));
      await expect(cell).toHaveText(`${Math.floor(seconds/60)}m ${String(seconds%60).padStart(2,'0')}s`);
    }
  }
  expect(await page.evaluate(async () => {
    const {fmtDuration}=await import('/src/lib/ipc.ts');
    return [fmtDuration(1348),fmtDuration(1348,true),fmtDuration(2810,true),fmtDuration(3599.6,true),fmtDuration(3601,true),fmtDuration(1200,true)];
  })).toEqual(['22m','22m 28s','46m 50s','1h 00m 00s','1h 00m 01s','20m 00s']);
  const times = await preview.locator('tbody tr').evaluateAll(rows => rows.map(row => [...row.querySelectorAll('td')].map(cell => Number(cell.dataset.seconds))));
  expect(times).toHaveLength(4);
  expect(times.every(([before,after]) => after < before && after > before*.95)).toBe(true);
  expect(await page.evaluate(async () => {
    const {trainingRaceTime}=await import('/src/desk/race-estimates.tsx');
    return [5,10,21.0975,42.195].map(distance=>trainingRaceTime(285,0,distance));
  })).toEqual([1348,2810,6201,12928]);
  await expect(page.getByRole('region',{name:'Proposed session changes'})).toContainText('10.3 km');
  await expect(page.getByRole('region',{name:'Proposed session changes'})).toContainText('4:30 - 4:40/km');
  expect(await page.evaluate(()=>window.__deskCalls.filter(call=>call.command==='apply_proposal'))).toHaveLength(0);
  await page.evaluate(()=>{window.__deskFail='apply_proposal';});
  await page.getByRole('button',{name:'Apply changes',exact:true}).click();
  await expect(page.getByRole('dialog').getByRole('alert')).toBeVisible();
  await expect(preview).toBeVisible();
  await page.evaluate(()=>{window.__deskFail='';});
  await page.getByRole('button',{name:'Apply changes',exact:true}).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.getByRole('link',{name:'Overview',exact:true}).click();
  await page.getByRole('button',{name:'Show health details',exact:true}).click();
  const estimates=page.getByRole('region',{name:'Training race estimates'});
  await expect(estimates).toBeVisible();
  expect(await estimates.locator('tbody td').evaluateAll(cells=>cells.map(cell=>Number(cell.dataset.seconds)))).toEqual(times.map(values=>values[1]));
});

for (const width of [320,1440]) {
  test(`percentage charts and tinted glass respect accessibility preferences at ${width}px`, async ({page}) => {
    await page.setViewportSize({width,height:1000});
    for (const theme of ['light','dark']) {
      await page.goto(`/desk.html?theme=${theme}&palette=amethyst#health`);
      await expect(page.locator('.trend-chart[data-percent]')).toHaveCount(5);
      await page.evaluate(()=>document.fonts.ready);
      expect(await page.evaluate(()=>[...document.fonts].some(font=>font.family==='Outfit' && font.status==='loaded'))).toBe(true);
      const charts=await page.locator('.trend-chart[data-percent]').evaluateAll(figures=>figures.map(figure=>({
        label:figure.querySelector('figcaption span').textContent,
        latest:figure.querySelector('figcaption strong').textContent,
        values:[...figure.querySelectorAll('circle')].map(circle=>Number(circle.getAttribute('cy'))),
        gradient:figure.querySelector('linearGradient')?.id,
        stop:figure.querySelector('stop:last-child')?.getAttribute('stop-opacity'),
        ticks:[...figure.querySelectorAll('svg text')].map(text=>text.textContent),
      })));
      expect(charts.every(chart=>chart.latest.endsWith('%') && chart.values.every(value=>value>=20 && value<=128))).toBe(true);
      expect(charts.every(chart=>chart.stop==='0' && chart.ticks.includes('100%') && chart.ticks.includes('0%'))).toBe(true);
      expect(new Set(charts.map(chart=>chart.gradient)).size).toBe(5);
      const load=charts.find(chart=>chart.label==='Training load (normalized)');
      const raw=await page.evaluate(async()=>window.__TAURI_INTERNALS__.invoke('dashboard',{range:'7d'}));
      const latest=raw.loadHistory.at(-1).ratio;
      expect(load.latest).toBe(`${Math.round(Math.min(100,Math.max(0,latest/raw.thresholds.ratioOptimizedHi*100)))}%`);
      const readiness=charts.find(chart=>chart.label==='Readiness');
      expect(readiness.latest).toBe(`${raw.scores.globalScore}%`);
      const amethyst=await page.locator('body').evaluate(element=>getComputedStyle(element).backgroundImage);
      await page.goto(`/desk.html?theme=${theme}&palette=ember#health`);
      await expect(page.locator('.trend-chart[data-percent]')).toHaveCount(5);
      expect(await page.locator('body').evaluate(element=>getComputedStyle(element).backgroundImage)).not.toBe(amethyst);
      const glass=await page.locator('.desk-toolbar').evaluate(element=>getComputedStyle(element).backdropFilter);
      expect(glass).toContain('blur');
      expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
      expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations).toEqual([]);
      await page.screenshot({path:`test-results/desk-glass-health-${theme}-${width}.png`,fullPage:true});
    }
    await page.emulateMedia({reducedMotion:'reduce',contrast:'more'});
    expect(await page.locator('.desk-toolbar').evaluate(element=>getComputedStyle(element).backdropFilter)).toBe('none');
    expect(await page.locator('.desk-button').first().evaluate(element=>getComputedStyle(element).transitionDuration)).toBe('0s');
  });
}

for (const width of [320, 1440]) {
  test(`pace insights compare normalized rep ranges and support inspection at ${width}px`, async ({page}) => {
    await page.setViewportSize({width,height:1000});
    for (const theme of ['light','dark']) {
      await page.goto(`/desk.html?theme=${theme}#health/pace`);
      await expect(page.getByRole('heading',{name:'Pace on point',exact:true})).toBeVisible();
      const points = page.locator('.pace-point-session');
      await expect(points).toHaveCount(5);
      await expect(page.locator('.pace-summary')).toHaveText('2/4 sessions within target');
      await expect(page.getByText('Next speed workout in 3 days', {exact:true})).toBeVisible();
      const geometry = await points.evaluateAll(groups => groups.map(group => ({
        top:Number(group.querySelector('.pace-point-range').getAttribute('y1')),
        bottom:Number(group.querySelector('.pace-point-range').getAttribute('y2')),
        circles:group.querySelectorAll('circle').length,
      })));
      expect(geometry[0].top).toBeLessThan(geometry[0].bottom);
      expect(geometry[2].circles).toBe(1);
      expect(geometry[2].top).toBeCloseTo(140, 3);
      const band = await page.locator('.pace-point-band').evaluate(element => ({top:Number(element.getAttribute('y')),height:Number(element.getAttribute('height'))}));
      expect(geometry[1].top).toBeGreaterThan(band.top);
      expect(geometry[1].bottom).toBeLessThan(band.top + band.height);
      expect(geometry[0].top).toBeLessThan(band.top);
      expect(geometry[0].bottom).toBeGreaterThan(band.top + band.height);
      expect(await page.locator('.pace-insights-figure svg').evaluate(svg => {
        const band = svg.querySelector('.pace-point-band');
        const left = Number(band.getAttribute('x'));
        const right = left + Number(band.getAttribute('width'));
        return [...svg.querySelectorAll('circle')].every(point => Number(point.getAttribute('cx')) >= left && Number(point.getAttribute('cx')) <= right);
      })).toBe(true);
      const picker = page.getByRole('group',{name:'Inspect pace session'});
      await picker.getByRole('button').nth(2).focus();
      await page.keyboard.press('Enter');
      await expect(page.getByRole('region',{name:'Selected pace session'})).toContainText('Continuous tempo');
      await expect(page.getByRole('region',{name:'Pace insight repetitions'}).locator('tbody tr')).toHaveCount(1);
      await picker.getByRole('button').first().click();
      await expect(page.getByRole('region',{name:'Selected pace session'})).toContainText('Outdated: excluded from current pace status');
      await page.getByRole('button',{name:'Long',exact:true}).click();
      await expect(points).toHaveCount(1);
      await expect(page.getByRole('region',{name:'Selected pace session'})).toContainText('Race-pace long run');
      await expect(page.getByText('Next long run in 5 days',{exact:true})).toBeVisible();
      await page.getByRole('button',{name:'Speed',exact:true}).click();
      await expect(points).toHaveCount(5);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations).toEqual([]);
      await page.locator('.pace-insights').screenshot({path:`test-results/desk-pace-insights-${theme}-${width}.png`});
    }
    await page.getByRole('button',{name:'Open activity review',exact:true}).click();
    await expect(page).toHaveURL(/#journal\/1$/);
    // The review opens on Overview; the execution chart is one tab away.
    await expect(page.getByRole('heading',{name:'Session execution',exact:true})).toHaveCount(0);
    await page.getByRole('tab',{name:'Session execution',exact:true}).click();
    await expect(page).toHaveURL(/#journal\/1\/execution$/);
    await expect(page.getByRole('heading',{name:'Session execution',exact:true})).toBeVisible();
  });
}

test('pace insights cap chart and inspection at five recent sessions regardless of outdated status', async ({page}) => {
  await page.addInitScript(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, args) => {
      const result = await invoke(command, args);
      if (command !== 'pace_insights') return result;
      const sessions = Array.from({length: 9}, (_, index) => ({
        ...result.speed[0],
        workoutId: `recent-${index}`,
        title: `Recent session ${index}`,
        date: `2026-10-${String(index + 1).padStart(2, '0')}`,
        outdated: index === 6,
        reps: index === 8 ? [] : result.speed[0].reps,
      })).reverse();
      return {...result, speed: sessions, long: sessions.map(session => ({...session, outdated: false}))};
    };
  });
  for (const width of [320, 1440]) {
    await page.setViewportSize({width, height: 1000});
    for (const theme of ['light', 'dark']) {
      await page.goto(`/desk.html?theme=${theme}#health/pace`);
      for (const mode of ['Speed', 'Long']) {
        await page.getByRole('button', {name: mode, exact: true}).click();
        const points = page.locator('.pace-point-session');
        await expect(points).toHaveCount(5);
        expect(await points.evaluateAll(elements => elements.map(element => element.dataset.workout)))
          .toEqual(['recent-3', 'recent-4', 'recent-5', 'recent-6', 'recent-7']);
        const picker = page.getByRole('group', {name: 'Inspect pace session'});
        await expect(picker.getByRole('button')).toHaveCount(5);
        await expect(page.getByRole('region', {name: 'Selected pace session'})).toContainText('Recent session 7');
        await expect(page.locator('.pace-summary')).toContainText(mode === 'Speed' ? '/4 sessions' : '/5 sessions');
        await picker.getByRole('button').first().click();
        await expect(page.getByRole('region', {name: 'Selected pace session'})).toContainText('Recent session 3');
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      }
    }
  }
});

test('pace insights handle no plan, empty history and retry without changing the plan', async ({page}) => {
  await page.goto('/desk.html?noPlan=1#health/pace');
  await expect(page.getByText('Create a training plan and link completed runs to see pace insights.')).toBeVisible();
  await expect(page.locator('.pace-point-session')).toHaveCount(0);
  await page.addInitScript(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    let fail = true;
    window.__TAURI_INTERNALS__.invoke = async (command,args) => {
      if(command === 'pace_insights') {
        if(fail) { fail=false;throw new Error('Synthetic insight failure'); }
        return {speed:[],long:[],nextSpeed:null,nextLong:null};
      }
      return invoke(command,args);
    };
  });
  await page.goto('/desk.html#health/pace');
  await expect(page.getByText(/Synthetic insight failure/)).toBeVisible();
  await page.getByRole('button',{name:'Try again',exact:true}).click();
  await expect(page.getByText(/Completed tempo and interval sessions/)).toBeVisible();
  await page.getByRole('button',{name:'Long',exact:true}).click();
  await expect(page.getByText(/Conversational runs are not graded/)).toBeVisible();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => ['apply_proposal','save_settings','create_plan'].includes(call.command)))).toEqual([]);
});

for (const session of ['intervals', 'tempo']) {
  test(`${session} activity shows measured lap bars against prescribed pace ranges`, async ({page}) => {
    await page.setViewportSize({width:390, height:1000});
    if (session === 'tempo') await page.addInitScript(() => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        const result = await invoke(command, args);
        if (command !== 'activity_review') return result;
        return {...result, laps:[result.laps[0], {...result.laps[3], index:2, label:'Continuous tempo'}, {...result.laps.at(-1), index:3}]};
      };
    });
    await page.goto('/desk.html#journal/1/execution');
    const chart = page.getByRole('region',{name:'Lap pace chart'});
    const buttons = chart.getByRole('button');
    await expect(buttons).toHaveCount(session === 'tempo' ? 3 : 9);
    await expect(chart.locator('.lap-target-band')).toHaveCount(session === 'tempo' ? 1 : 4);
    await expect(chart.locator('.lap-target-band.on-target')).toHaveCount(session === 'tempo' ? 1 : 2);
    // The table states the same thing in words: one row per lap, with the
    // graded laps' target cells highlighted where the chart does.
    await expect(page.locator('.lap-execution-table tbody tr')).toHaveCount(session === 'tempo' ? 3 : 9);
    await expect(page.locator('.lap-execution-table .lap-row-target.on-target')).toHaveCount(session === 'tempo' ? 1 : 2);
    const geometry = await buttons.evaluateAll(columns => columns.map(column => ({
      actual:parseFloat(column.querySelector('.lap-pace-actual').style.height),
      bottom:parseFloat(column.querySelector('.lap-target-band')?.style.bottom),
      band:parseFloat(column.querySelector('.lap-target-band')?.style.height),
      bandPx:column.querySelector('.lap-target-band')?.getBoundingClientRect().height ?? 0,
      width:column.getBoundingClientRect().width,
    })));
    if (session === 'intervals') {
      // The axis is linear over every pace the chart draws (laps and band edges),
      // padded 15% of the span either side, so a target range is a readable band
      // rather than a hairline. Fastest 268 s/km, slowest 402 s/km, so the axis
      // runs 247.9 to 422.1 over a 174.2 s span.
      const pos = pace => (422.1 - pace) / 174.2 * 100;
      expect(geometry[1].actual).toBeCloseTo(pos(268), 3);
      expect(geometry[1].bottom).toBeCloseTo(pos(285), 3);
      expect(geometry[1].bottom + geometry[1].band).toBeCloseTo(pos(275), 3);
      // Each column is as wide as its lap is long: the 24% warm-up is roughly
      // twice the 12% rep and ten times a 2.5% recovery.
      expect(geometry[0].width).toBeGreaterThan(geometry[1].width * 1.7);
      expect(geometry[1].width).toBeGreaterThan(geometry[2].width * 3);
      expect(geometry[2].width).toBeGreaterThanOrEqual(20);
      expect(geometry[1].actual).toBeGreaterThan(geometry[1].bottom + geometry[1].band);
      // A 10 s/km target range has to be something the eye can land on, not the
      // 5 px line the old reciprocal scale drew.
      expect(geometry[1].bandPx).toBeGreaterThanOrEqual(12);
      expect(geometry[3].actual).toBeGreaterThan(geometry[3].bottom);
      expect(geometry[3].actual).toBeLessThan(geometry[3].bottom + geometry[3].band);
      expect(geometry[5].actual).toBeLessThan(geometry[5].bottom);
      for (const index of [0,2,4,6,8]) await expect(buttons.nth(index).locator('.lap-target-band')).toHaveCount(0);
      await buttons.nth(5).click();
      await expect(page.locator('.lap-pace-selection')).toContainText('Slower than target');
      await expect(page.locator('.lap-execution-table tr[data-selected="true"]')).toContainText('4:51/km');
    } else {
      expect(geometry[1].actual).toBeGreaterThan(geometry[1].bottom);
      expect(geometry[1].actual).toBeLessThan(geometry[1].bottom + geometry[1].band);
    }
    await buttons.nth(1).focus();
    await page.keyboard.press('Enter');
    await expect(buttons.nth(1)).toHaveAttribute('aria-pressed','true');
    await expect(page.locator('.lap-pace-selection')).toContainText('4:35 - 4:45/km');
    await expect(page.getByRole('region',{name:'Recorded laps'}).getByRole('columnheader',{name:'Time',exact:true})).toBeVisible();
    await expect(page.getByRole('region',{name:'Recorded laps'}).getByRole('columnheader',{name:'Pace',exact:true})).toBeVisible();
    await expect(page.getByRole('region',{name:'Recorded laps'}).getByRole('columnheader',{name:'Target pace',exact:true})).toBeVisible();
    const rows = await page.locator('.lap-execution-table tbody tr').evaluateAll(tableRows => tableRows.map(row => row.innerText.replace(/\s+/g, ' ').trim()));
    const paces = await buttons.evaluateAll(columns => columns.map(column => column.querySelector('.lap-pace-actual > span')?.textContent ?? 'Not recorded'));
    for (const [index, column] of geometry.entries()) {
      const row = rows[index];
      expect(row).toContain(paces[index]);
      if (!Number.isNaN(column.bottom)) {
        expect(row).toContain('4:35 - 4:45/km');
      } else {
        expect(row).toContain('Not prescribed');
      }
    }
    await page.getByRole('region',{name:'Recorded laps'}).getByRole('button').first().click();
    await expect(buttons.first()).toHaveAttribute('aria-pressed','true');
    await expect(page.locator('.lap-pace-selection')).toContainText('Warm-up');
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations).toEqual([]);
    await page.locator('.lap-execution').screenshot({path:`test-results/desk-${session}-pace-ranges.png`});
  });
}

test('lap ranges handle missing measurements and never invent unprescribed targets', async ({page}) => {
  await page.addInitScript(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, args) => {
      const result = await invoke(command, args);
      if (command !== 'activity_review') return result;
      return {...result, prescribed:!new URLSearchParams(location.search).has('unprescribed'), laps:result.laps.map((lap,index) => index === 1 ? {...lap,avgPaceSKm:null} : index === 3 ? {...lap,bandLoSKm:300,bandHiSKm:280} : lap)};
    };
  });
  await page.goto('/desk.html#journal/1/execution');
  const missing = page.getByRole('region',{name:'Lap pace chart'}).getByRole('button').nth(1);
  await expect(missing.locator('.lap-target-band')).toHaveCount(1);
  await expect(missing.locator('.lap-pace-actual')).toHaveCount(0);
  await missing.click();
  await expect(page.locator('.lap-pace-selection')).toContainText('Not recorded');
  await expect(page.locator('.lap-pace-selection')).toContainText('Not graded');
  await expect(page.locator('.lap-pace-column').nth(3).locator('.lap-target-band')).toHaveCount(0);
  expect(await page.locator('.lap-execution').innerHTML()).not.toMatch(/NaN|Infinity/);
  await page.goto('/desk.html?unprescribed=1#journal/1/execution');
  await expect(page.getByRole('heading',{name:'Recorded laps',exact:true})).toBeVisible();
  await expect(page.locator('.lap-target-band, .lap-pace-chart')).toHaveCount(0);
  await expect(page.locator('.lap-execution-table .lap-row-target.on-target')).toHaveCount(0);
  await expect(page.getByRole('region',{name:'Recorded laps'})).toContainText('Not prescribed');
});

for (const width of [320,1440]) {
  test(`review details remain accessible at ${width}px`, async ({page}) => {
    await page.setViewportSize({width,height:1000});
    for(const theme of ['light','dark']) {
      await page.goto(`/desk.html?theme=${theme}#journal/1/splits`);
      await expect(page.getByRole('heading',{name:'Kilometre splits'})).toBeVisible();
      expect(await page.getByRole('region',{name:'Kilometre splits'}).evaluate(region => region.scrollWidth <= region.clientWidth)).toBe(true);
      await page.getByRole('tab',{name:'Session execution',exact:true}).click();
      await expect(page.getByRole('heading',{name:'Session execution',exact:true})).toBeVisible();
      expect(await page.locator('.lap-execution-table').evaluate(table => [...table.querySelectorAll('th, td')].every(cell => getComputedStyle(cell).whiteSpace === 'nowrap'))).toBe(true);
      expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'activity review').toEqual([]);
      expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
      await page.screenshot({path:`test-results/desk-activity-${theme}-${width}.png`,fullPage:true});
      await page.goto(`/desk.html?proposal=1&theme=${theme}#health/journal`);
      await page.getByRole('button',{name:'Review changes',exact:true}).click();
      await expect(page.getByRole('dialog',{name:'Review plan changes'})).toBeVisible();
      expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'proposal review').toEqual([]);
      await page.screenshot({path:`test-results/desk-proposal-${theme}-${width}.png`,fullPage:true});
    }
  });
}

test('ending a plan requires confirmation and keeps recorded history',async ({page})=>{
  await page.goto('/plan.html');
  await page.getByRole('button',{name:'End current plan',exact:true}).click();
  await expect(page.getByRole('dialog')).toContainText('Other saved plans are kept.');
  expect(await page.evaluate(()=>window.__deskCalls.filter(call=>call.command==='abort_plan'))).toEqual([]);
  await page.getByRole('button',{name:'Keep plan',exact:true}).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByRole('button',{name:'End current plan',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'End current plan',exact:true}).click();
  await page.getByRole('dialog').getByRole('button',{name:'End current plan',exact:true}).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect(await page.evaluate(()=>window.__deskCalls.filter(call=>call.command==='abort_plan'))).toHaveLength(1);
  await page.getByRole('link',{name:'Activities',exact:true}).click();
  await expect(page.locator('.activity-row')).toHaveCount(9);
});

test("original entry routes load the independent workspace and retain deep links", async ({page}) => {
  const loaded=[];
  page.on('request',request=>loaded.push(request.url()));
  for(const [url,heading,section] of [
    ['/', 'Training', 'Week'],
    ['/index.html', 'Training', 'Week'],
    ['/plan.html', 'Training', 'Plan'],
    ['/calendar.html', 'Training', 'Month'],
    ['/achievements.html', 'Activities', null],
    ['/achievements.html#performance', 'Health', 'Overview'],
    ['/achievements.html#recovery', 'Health', 'Overview'],
    ['/achievements.html#risk', 'Health', 'Overview'],
    ['/achievements.html#milestones', 'Health', 'Milestones'],
    ['/injury.html', 'Health', 'Pain journal'],
    ['/settings.html', 'Settings', null],
    ['/support.html', 'Help', null],
  ]) {
    await page.goto(url);
    await expect(page.locator('#workspace h1')).toHaveText(heading);
    if(section) await expect(page.locator('.horizon-switch a[aria-current="page"]')).toHaveText(section);
    await expect(page.locator('#desk-root')).toBeVisible();
  }
  for(const [hash,heading] of [['theme','Appearance'],['zones','Training boundaries'],['tiles','Offline maps'],['publishing','Calendar apps'],['setup','Watch connection']]) {
    await page.goto(`/settings.html#${hash}`);
    await expect(page.getByRole('heading',{name:heading,exact:true})).toBeVisible();
  }
  await page.goto('/index.html#journal/1');
  await expect(page.getByRole('complementary',{name:'Activity review'})).toBeVisible();
  expect(loaded.filter(url=>/\/(app|redesign)\.css|\/src\/(dashboard|plan|calendar|achievements|injury|settings|support)\.ts|\/lib\/(navigation|overlay|workout-detail)\.ts/.test(url))).toEqual([]);
});

for (const theme of ['light','dark']) {
  for (const width of [320,390,768,1440]) {
    test(`visual matrix ${theme} ${width}px`, async ({page}) => {
      test.setTimeout(90000);
      const errors=[];
      page.on('pageerror',error=>errors.push(error.message));
      await page.setViewportSize({width,height:1000});
      for (const route of ['training','training/month','training/plan','journal','health','health/journal','health/milestones','settings','help']) {
        await page.goto(`/desk.html?theme=${theme}#${route}`);
        await expect(page.locator('#workspace h1')).toBeVisible();
        await expect(page.locator('.desk-loading')).toHaveCount(0);
        await expect(page.getByRole('alert')).toHaveCount(0);
        expect(await page.evaluate(() => document.documentElement.scrollWidth<=innerWidth),route).toBe(true);
        expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,route).toEqual([]);
        await page.screenshot({path:`test-results/desk-${route.replace('/','-')}-${theme}-${width}.png`,fullPage:true});
      }
      await page.goto(`/desk.html?theme=${theme}#training`);
      if (width<=800) await page.locator('.mobile-date-ribbon button').first().click();
      await page.locator(width>800 ? '.week-board .session-tile' : '.day-itinerary .session-tile').first().click();
      await expect(page.getByRole('dialog',{name:'Session',exact:true})).toBeVisible();
      if (width<=800) {
        for(let index=0;index<14;index++) {await page.keyboard.press('Tab');expect(await page.evaluate(()=>Boolean(document.activeElement?.closest('.desk-inspector')))).toBe(true);}
      }
      expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()).violations,'session inspector').toEqual([]);
      await page.screenshot({path:`test-results/desk-session-${theme}-${width}.png`,fullPage:true});
      await page.keyboard.press('Escape');
      await expect(page.getByRole('dialog',{name:'Session',exact:true})).toHaveCount(0);
      await expect(page.getByRole('navigation',{name:'Workspaces'})).toBeVisible();
      expect(errors).toEqual([]);
    });
  }
}

test("native sync completion refreshes evidence and watch progress is visible", async ({page}) => {
  await page.goto('/desk.html');
  await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='plugin:event|listen').map(call=>call.args.event))).toEqual(expect.arrayContaining(['sync://started','sync://completed','plan://sync','plan://stage','plan://updated']));
  await page.evaluate(() => window.__deskEmit('sync://started', null));
  await expect(page.getByRole('button',{name:'Syncing activities'})).toBeDisabled();
  const before = await page.evaluate(() => window.__deskCalls.filter(call=>call.command==='current_plan').length);
  await page.evaluate(() => window.__deskEmit('sync://completed',{ok:true}));
  await expect(page.getByRole('button',{name:'Sync activities'})).toBeEnabled();
  await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='current_plan').length)).toBeGreaterThan(before);
  // A plan that has never reached the watch is pushed by itself: the page
  // opens on ?noPush=1&holdPush=1, the fixture holds the push open, and the
  // backend's plan://sync stages must drive the progress bar unattended.
  await page.goto('/desk.html?noPush=1&holdPush=1');
  await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='push_plan_to_provider').length)).toBe(1);
  await page.evaluate(() => window.__deskEmit('plan://sync',{op:'push',percent:45,stage:'Publishing sessions'}));
  await expect(page.getByRole('progressbar',{name:'Watch publishing progress'})).toHaveAttribute('value','45');
  await page.evaluate(() => window.__deskResolvePush());
  await expect(page.getByRole('status')).toContainText('Plan sent to COROS');
});

test("a plan already on the watch is never re-pushed by the page", async ({page}) => {
  await page.goto('/desk.html');
  await expect(page.locator('#workspace h1')).toBeVisible();
  await page.waitForTimeout(500);
  expect(await page.evaluate(() => window.__deskCalls.filter(call=>call.command==='push_plan_to_provider').length)).toBe(0);
});

test("pointer drag moves a session to the lane under the pointer and never opens it", async ({page}) => {
  await page.setViewportSize({width:1440,height:1000});
  await page.goto('/desk.html');
  await page.getByRole('button',{name:'Next week',exact:true}).click();
  const tile = page.locator('.week-board .session-tile.movable').first();
  const lanes = page.locator('.week-board .day-lane');
  const fromIndex = await lanes.evaluateAll(nodes => nodes.findIndex(node => node.querySelector('.session-tile.movable')));
  const target = lanes.nth(fromIndex + 1);
  const a = await tile.boundingBox();
  const b = await target.boundingBox();
  await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
  await page.mouse.down();
  await page.mouse.move(a.x + a.width / 2 + 30, a.y + a.height / 2, {steps: 4});
  await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2, {steps: 8});
  await expect(target).toHaveAttribute('data-drop-target', '');
  await page.mouse.up();
  await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='move_workout').map(call=>call.args.date))).toHaveLength(1);
  await expect(page.getByRole('status')).toContainText('Moved to');
  // The click the browser fires after the release must not open the inspector.
  await expect(page.getByRole('dialog',{name:'Session',exact:true})).toHaveCount(0);
});

test("month view marks finished sessions like the week board does", async ({page}) => {
  await page.setViewportSize({width:1440,height:1000});
  await page.goto('/desk.html#training/month');
  const tiles = page.locator('.month-sessions .session-tile');
  await expect(tiles.first()).toBeVisible();
  const done = page.locator('.month-sessions .session-tile [aria-label="Completed"]');
  expect(await done.count()).toBeGreaterThan(0);
  await expect(done.first()).toBeVisible();
  await expect(page.locator('.month-sessions .session-kind').first()).toBeVisible();
  await expect(page.locator('button.month-number')).toHaveCount(0);
  await expect(page.locator('.month-dates')).toHaveCSS('border-radius', '8px');
  await expect(page.locator('.month-day').first()).toHaveCSS('border-radius', '0px');
  await expect(page.locator('.month-day').last()).toHaveCSS('border-bottom-width', '1px');
  await expect(page.locator('.month-day').last()).toHaveCSS('border-right-width', '1px');
  // Today stays highlighted in the month grid: the number carries
  // aria-current="date" and wears the filled disc, not plain ink.
  const todayCell = page.locator('.month-number[aria-current="date"]');
  await expect(todayCell).toHaveCount(1);
  await expect.poll(async () => (await todayCell.evaluate(el => getComputedStyle(el).backgroundColor)) !== 'rgba(0, 0, 0, 0)').toBe(true);
});

test("mobile month today fills its calendar cell", async ({page}) => {
  await page.setViewportSize({width: 390, height: 844});
  await page.goto('/desk.html#training/month');
  const today = page.locator('.month-number[aria-current="date"]');
  await expect(today).toBeVisible();
  const dimensions = await today.evaluate(element => {
    const button = element.getBoundingClientRect();
    const cell = element.parentElement.getBoundingClientRect();
    return {width: cell.width - button.width, height: cell.height - button.height};
  });
  expect(dimensions.width).toBeLessThanOrEqual(1);
  expect(dimensions.height).toBeLessThanOrEqual(1);
  await expect(page.locator('.month-dates')).toHaveCSS('border-radius', '8px');
});

test("schedule mutations retain intent and Undo restores the original date", async ({page}) => {
  await page.setViewportSize({width:1440,height:1000});
  await page.goto('/desk.html');
  await page.getByRole('button',{name:'Next week',exact:true}).click();
  await page.locator('.week-board .session-tile.movable').first().click();
  const original = await page.getByLabel('Run on',{exact:true}).inputValue();
  const next = new Date(`${original}T12:00:00`); next.setDate(next.getDate()+1);
  const moved = `${next.getFullYear()}-${String(next.getMonth()+1).padStart(2,'0')}-${String(next.getDate()).padStart(2,'0')}`;
  await page.getByLabel('Run on',{exact:true}).fill(moved);
  await page.getByRole('button',{name:'Move session',exact:true}).click();
  await expect(page.getByRole('status')).toContainText('Moved to');
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='move_workout').map(call=>call.args.date))).toEqual([moved,original]);
  await page.locator('.week-board .session-tile.movable').first().click();
  await page.getByRole('button',{name:'Skip session',exact:true}).click();
  await expect(page.getByRole('status')).toContainText('Session skipped');
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call=>call.command==='set_workout_skipped').map(call=>call.args.skipped))).toEqual([true,false]);
});

test("week summary shows run mileage against the plan's planned mileage", async ({page}) => {
  await page.setViewportSize({width:1440,height:1000});
  await page.goto('/desk.html');
  const strip = page.locator('.schedule-summary');
  await expect(strip).toBeVisible();
  // The km reading carries the plan's denominator: "X km / Y km done", so a
  // week at 38 run of 30 planned reads as 38/30 rather than a bare 38.
  await expect(strip.locator('span').nth(1)).toContainText("/");
  await expect(strip.locator('span').nth(1)).toContainText("done");
});

test("reset week undoes a scheduled benchmark and a moved session", async ({page}) => {
  await page.setViewportSize({width:1440,height:1000});
  await page.goto('/desk.html');
  await page.getByRole('button',{name:'Next week',exact:true}).click();
  // Schedule a benchmark over a planned session, the way the user did.
  await page.getByRole('button',{name:'Schedule race or test',exact:true}).click();
  const dialog = page.getByRole('dialog',{name:'Schedule race or test',exact:true});
  await dialog.getByRole('button',{name:'Schedule benchmark',exact:true}).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.locator('.week-board')).toContainText('Benchmark: parkrun');
  // The reset offer appears on the summary strip for the week horizon.
  await page.getByRole('button',{name:'Reset week',exact:true}).click();
  const reset = page.getByRole('dialog',{name:'Reset this week',exact:true});
  await expect(reset).toBeVisible();
  await expect(reset.locator('.reset-list li').filter({hasText:'Benchmark'})).toBeVisible();
  await reset.getByRole('button',{name:/^Reset \d+$/}).click();
  await expect(reset).toHaveCount(0);
  await expect(page.getByRole('status')).toContainText('back on the plan');
  await expect(page.locator('.week-board')).not.toContainText('Benchmark: parkrun');
  expect(await page.evaluate(() => window.__deskCalls.some(call=>call.command==='reset_week'))).toBe(true);
});

test("matching uses the chosen recorded activity", async ({page}) => {
  await page.setViewportSize({width:1440,height:1000});
  await page.goto('/desk.html?unmatched=1');
  await page.locator('.week-board .session-tile.movable').first().click();
  await page.getByRole('combobox',{name:'Recorded activity'}).selectOption('99');
  await page.getByRole('button',{name:'Match run',exact:true}).click();
  await expect(page.getByRole('status')).toContainText('Recorded run matched');
  expect(await page.evaluate(() => window.__deskCalls.find(call=>call.command==='match_calendar_activity').args.activityId)).toBe(99);
});

for (const action of ['Apply','Dismiss']) {
  test(`${action} proposal requires review and preserves a failed decision`, async ({page}) => {
    await page.goto('/desk.html?proposal=1#health/journal');
    await page.getByRole('button',{name:'Review changes',exact:true}).click();
    await expect(page.getByRole('dialog')).toContainText('Easy run');
    expect(await page.evaluate(() => window.__deskCalls.some(call=>['apply_proposal','dismiss_proposal'].includes(call.command)))).toBe(false);
    if (action==='Dismiss') await page.getByRole('textbox',{name:'Reason for dismissing (optional)'}).fill('Prefer to review with my clinician.');
    await page.evaluate(command => {window.__deskFail=command;},`${action.toLowerCase()}_proposal`);
    await page.getByRole('button',{name:`${action} changes`,exact:true}).click();
    await expect(page.getByRole('dialog').getByRole('alert')).toContainText('Unable to save');
    await page.evaluate(() => {window.__deskFail='';});
    await page.getByRole('button',{name:`${action} changes`,exact:true}).click();
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(page.getByRole('button',{name:'Review changes',exact:true})).toHaveCount(0);
  });
}

for (const width of [390,1440]) {
  test(`recorded route map renders geographic pixels and responds to zoom at ${width}px`, async ({page}) => {
    await page.setViewportSize({width,height:1000});
    await page.goto('/desk.html#journal/1');
    await expect(page.getByRole('region',{name:'Recorded GPS route'})).toBeVisible();
    const pixels = () => page.locator('.maplibregl-canvas').evaluate(canvas => {
      const context=canvas.getContext('webgl2');
      const bytes=new Uint8Array(context.drawingBufferWidth*context.drawingBufferHeight*4);
      context.readPixels(0,0,context.drawingBufferWidth,context.drawingBufferHeight,context.RGBA,context.UNSIGNED_BYTE,bytes);
      const probe = document.createElement("canvas");
      probe.width = probe.height = 1;
      const paint = probe.getContext("2d");
      paint.fillStyle = getComputedStyle(document.documentElement).getPropertyValue("--desk-action").trim();
      paint.fillRect(0, 0, 1, 1);
      const accent = paint.getImageData(0, 0, 1, 1).data;
      let colored=0, fingerprint=0;
      for(let index=0;index<bytes.length;index+=64) { if(bytes[index+3]>0 && Math.abs(bytes[index] - accent[0]) < 30 && Math.abs(bytes[index+1] - accent[1]) < 30 && Math.abs(bytes[index+2] - accent[2]) < 30) colored++; fingerprint=(fingerprint+bytes[index]*(index+1))%1000000007; }
      return {colored,fingerprint};
    });
    await expect.poll(async () => (await pixels()).colored).toBeGreaterThan(10);
    const before=await pixels();
    await page.getByRole('button',{name:'Zoom in',exact:true}).click();
    await expect.poll(async () => (await pixels()).fingerprint).not.toBe(before.fingerprint);
    await page.getByRole('button',{name:'Fit route',exact:true}).click();
    expect(await page.evaluate(() => document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    await page.screenshot({path:`test-results/desk-map-${width}.png`,fullPage:true});
  });
}

test("plan weeks use available space beside plan details", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 844 });
  await page.goto("/desk.html#training/plan");
  const list = page.getByRole("region", { name: "Plan weeks", exact: true });
  await expect(list).toBeVisible();
  await list.evaluate(element => {
    const rows = [...element.querySelectorAll(".roadmap-row")];
    for (const row of rows) element.appendChild(row.cloneNode(true));
  });
  await expect.poll(() => list.evaluate(element => element.scrollHeight > element.clientHeight)).toBe(true);
  const before = await list.evaluate(element => element.clientHeight);
  await page.locator(".plan-basis").evaluate(element => { element.style.minHeight = "1400px"; });
  await expect.poll(() => list.evaluate(element => element.clientHeight)).toBeGreaterThan(before);
  const gap = await page.locator(".plan-main").evaluate(element => {
    const overview = element.querySelector(".plan-overview").getBoundingClientRect();
    const evidence = element.querySelector(".adjustment-evidence").getBoundingClientRect();
    return evidence.top - overview.bottom;
  });
  expect(gap).toBeLessThanOrEqual(24);
});

for (const width of [390, 1440]) {
  for (const reducedMotion of ["no-preference", "reduce"]) {
    test(`plan centers the current week on every visit ${width}px ${reducedMotion}`, async ({ page }) => {
      await page.setViewportSize({ width, height: 844 });
      await page.emulateMedia({ reducedMotion });
      await page.goto("/desk.html#training");
      for (let visit = 0; visit < 2; visit++) {
        await page.getByRole("link", { name: "Plan", exact: true }).click();
        const list = page.getByRole("region", { name: "Plan weeks", exact: true });
        const current = list.locator(".is-current");
        await expect(current).toBeInViewport({ ratio: .9 });
        await expect.poll(() => list.evaluate(element => {
          const row = element.querySelector(".is-current");
          const target = Math.min(element.scrollHeight - element.clientHeight, Math.max(0, row.offsetTop - (element.clientHeight - row.offsetHeight) / 2));
          return Math.abs(element.scrollTop - target);
        })).toBeLessThan(2);
        expect(await page.evaluate(() => document.documentElement.scrollHeight <= innerHeight && scrollY === 0)).toBe(true);
        await list.evaluate(element => { element.scrollTop = element.scrollHeight; });
        await page.getByRole("link", { name: "Week", exact: true }).click();
        await expect(list).toHaveCount(0);
      }
    });
  }
}

for (const viewport of [{ width: 320, height: 640 }, { width: 844, height: 390 }, { width: 1440, height: 900 }]) {
  test(`content owns scroll without stretching the shell ${viewport.width}x${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto("/desk.html#health");
    const workspace = page.locator("#workspace");
    await page.getByRole("button", { name: "Show health details", exact: true }).click();
    const navigation = page.getByRole("navigation", { name: "Workspaces" });
    const before = await navigation.boundingBox();
    await workspace.hover({ position: { x: 20, y: 100 } });
    await page.mouse.wheel(0, 800);
    await expect.poll(() => workspace.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
    expect(await navigation.boundingBox()).toEqual(before);
    expect(await page.evaluate(() => ({ height: document.documentElement.scrollHeight, width: document.documentElement.scrollWidth, scroll: scrollY }))).toEqual({ height: viewport.height, width: viewport.width, scroll: 0 });
    await page.goto("/desk.html#journal");
    const library = page.getByRole("region", { name: "Activity history" });
    await expect(page.locator(".activity-row")).toHaveCount(9);
    await expect.poll(() => library.evaluate(element => element.scrollHeight > element.clientHeight)).toBe(true);
    expect(await library.evaluate(element => element.clientHeight <= document.querySelector("#workspace").clientHeight)).toBe(true);
    await library.focus();
    await page.keyboard.press("End");
    await expect.poll(() => library.evaluate(element => Math.abs(element.scrollHeight - element.clientHeight - element.scrollTop))).toBeLessThan(2);
    expect(await page.evaluate(() => scrollY)).toBe(0);
    await page.goto("/desk.html#training");
    if (viewport.width <= 800) await page.locator(".mobile-date-ribbon button").first().click();
    const tiles = page.locator(viewport.width <= 800 ? ".day-itinerary .session-tile" : ".week-board .session-tile");
    await expect(tiles.first()).toBeVisible();
    expect(await tiles.evaluateAll(elements => elements.every(element => {
      const styles = getComputedStyle(element);
      const children = [...element.children].filter(child => getComputedStyle(child).position !== "absolute");
      const content = children.reduce((height, child) => height + child.getBoundingClientRect().height, 0);
      const needed = content + Math.max(0, children.length - 1) * parseFloat(styles.rowGap) + parseFloat(styles.paddingTop) + parseFloat(styles.paddingBottom);
      return element.clientHeight <= Math.max(44, needed) + 2;
    }))).toBe(true);
  });
}

test("floating session details blur and lock their background without changing inline activity review", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/desk.html#training");
  const opener = page.locator(".week-board .session-tile").first();
  await opener.click();
  const inspector = page.getByRole("dialog", { name: "Session", exact: true });
  const backdrop = page.locator(".inspector-backdrop");
  await expect(inspector).toBeVisible();
  await expect(backdrop).toHaveCSS("backdrop-filter", "blur(12px)");
  await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "hidden");
  expect(await page.locator(".desk-appbar").evaluate(element => element.inert)).toBe(true);
  await inspector.getByRole("button", { name: "Close Session", exact: true }).focus();
  await page.keyboard.press("Shift+Tab");
  expect(await inspector.evaluate(element => element.contains(document.activeElement))).toBe(true);
  await backdrop.click({ position: { x: 20, y: 200 } });
  await expect(inspector).toHaveCount(0);
  await expect(backdrop).toHaveCount(0);
  await expect(opener).toBeFocused();
  await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "auto");
  expect(await page.locator(".desk-appbar").evaluate(element => element.inert)).toBe(false);
  await page.goto("/desk.html#journal/1");
  await expect(page.getByRole("complementary", { name: "Activity review" })).toBeVisible();
  await expect(backdrop).toHaveCount(0);
  await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "hidden");
  await expect(page.getByRole("complementary", { name: "Activity review" })).toHaveCSS("overflow-y", "auto");
});

test("nested information dialogs retain review drafts and outer scroll ownership", async ({ page }) => {
  await page.goto("/desk.html?pacingProposal=1#health/pace");
  await page.getByRole("button", { name: "Review changes", exact: true }).click();
  const review = page.getByRole("dialog", { name: "Review plan changes", exact: true });
  const reason = review.getByRole("textbox", { name: "Reason for dismissing (optional)" });
  await reason.fill("Review after my next workout");
  const basis = review.getByRole("button", { name: "Estimate basis", exact: true });
  await basis.click();
  await expect(page.locator("dialog[open]")).toHaveCount(2);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: "Estimate basis", exact: true })).toHaveCount(0);
  await expect(reason).toHaveValue("Review after my next workout");
  await expect(basis).toBeFocused();
  await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "hidden");
  await review.getByRole("button", { name: "Dismiss changes", exact: true }).click();
  await expect(review).toHaveCount(0);
  await page.getByRole("button", { name: "Adjustment history", exact: true }).click();
  const history = page.getByRole("dialog", { name: "Adjustment history", exact: true });
  const entry = history.getByRole("button", { name: /Sharpen quality pace/ });
  await entry.click();
  await expect(review).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(review).toHaveCount(0);
  await expect(entry).toBeFocused();
  await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "hidden");
  await page.keyboard.press("Escape");
  await expect(history).toHaveCount(0);
  await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "auto");
});

test("independent shell loads only new presentation", async ({ page }) => {
  const loaded = [];
  page.on("request", request => loaded.push(request.url()));
  await page.goto("/desk.html");
  await expect(page.getByRole("heading", { name: "Training", exact: true })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Workspaces" }).getByRole("link")).toHaveCount(3);
  await expect(page.locator(".day-itinerary")).toBeVisible();
  expect(loaded.filter(url => /\/(app|redesign)\.css|\/src\/(dashboard|plan|calendar|achievements|injury|settings|support)\.ts|\/lib\/(navigation|overlay|workout-detail)\.ts/.test(url))).toEqual([]);
});

test("mobile date selection opens a full-screen session and returns to its opener", async ({ page }) => {
  await page.setViewportSize({width:390, height:844});
  await page.emulateMedia({reducedMotion:"no-preference"});
  await page.goto("/desk.html#health");
  await page.getByRole("navigation", {name:"Workspaces"}).getByRole("link", {name:"Training"}).click();
  await page.locator(".mobile-date-ribbon button").first().click();
  const session = page.locator(".day-itinerary .session-tile").first();
  await session.click();
  await expect(page.getByRole("dialog", { name: "Session", exact: true })).toBeVisible();
  await expect.poll(async () => {
    const bounds = await page.locator(".desk-inspector").boundingBox();
    return bounds && Object.fromEntries(Object.entries(bounds).map(([key,value]) => [key,Math.round(value)]));
  }).toEqual({x:0,y:0,width:390,height:844});
  await expect(page.locator(".prescription")).toContainText("Repeat 4 times:");
  await page.getByRole("button", { name: "Close Session", exact: true }).click();
  await expect(session).toBeFocused();
});

for (const width of [390, 1440]) {
  test(`benchmark scheduling submits parkrun and custom test contracts at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/desk.html#training/week");
    const open = page.getByRole("button", { name: "Schedule race or test", exact: true });
    await open.click();
    const dialog = page.getByRole("dialog", { name: "Schedule race or test", exact: true });
    await expect(dialog.getByLabel("Distance (km)")).toHaveValue("5");
    await expect(dialog.getByLabel("Distance (km)")).toBeDisabled();
    await expect(dialog.getByLabel("Reduce this week's planned mileage by 20%")).toBeChecked();
    const date = await dialog.getByLabel("Date", { exact: true }).inputValue();
    const expectedPlanId = await page.evaluate(async () => (await window.__TAURI_INTERNALS__.invoke("current_plan"))[0].id);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
    await expect(dialog).toHaveCount(0);
    expect(await page.evaluate(() => window.__deskCalls.find(call => call.command === "schedule_benchmark").args)).toEqual({ date, name: "parkrun", distanceM: 5000, expectedPlanId, reduceMileage: true });
    await expect(page.locator(".day-itinerary")).toContainText("Benchmark: parkrun");
    await open.click();
    await dialog.getByRole("combobox", { name: "Event", exact: true }).selectOption("trial");
    await dialog.getByLabel("Event name", { exact: true }).fill("3 km fitness test");
    await dialog.getByLabel("Distance (km)").fill("3");
    await dialog.getByLabel("Reduce this week's planned mileage by 20%").uncheck();
    await page.evaluate(() => { window.__deskFail = "schedule_benchmark"; });
    await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
    await expect(dialog.getByRole("alert")).toContainText("Unable to save right now");
    await expect(dialog.getByLabel("Event name", { exact: true })).toHaveValue("3 km fitness test");
    await page.screenshot({ path: `test-results/benchmark-${width}.png`, animations: "disabled" });
    await page.evaluate(() => { window.__deskFail = ""; });
    await dialog.getByRole("button", { name: "Schedule benchmark", exact: true }).click();
    await expect(dialog).toHaveCount(0);
    expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "schedule_benchmark").pop().args)).toMatchObject({ name: "3 km fitness test", distanceM: 3000, reduceMileage: false });
  });
}

for (const theme of ["light", "dark"]) {
  for (const width of [320, 390, 768, 1440]) {
    test(`training method chooser ${theme} ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 1000 });
      await page.goto(`/desk.html?noPlan=1&theme=${theme}`);
      await page.getByRole("button", { name: "Create plan", exact: true }).click();
      const dialog = page.getByRole("dialog", { name: "Create training plan" });
      const norwegian = dialog.getByRole("radio", { name: "Norwegian method", exact: true });
      const runna = dialog.getByRole("radio", { name: "Runna-like", exact: true });
      await expect(norwegian).toBeChecked();
      await expect(norwegian).toHaveAccessibleDescription(/Controlled sub-threshold repeats/);
      await expect(runna).toHaveAccessibleDescription(/varied mix of intervals, tempo and long runs/);
      await norwegian.focus();
      await page.keyboard.press("ArrowDown");
      await expect(runna).toBeChecked();
      await expect(dialog.locator(".composer-summary")).toContainText("Runna-like");
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      expect(await dialog.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
      await dialog.screenshot({ path: `test-results/training-method-${theme}-${width}.png`, animations: "disabled" });
      await dialog.getByRole("button", { name: "Continue", exact: true }).click();
      await dialog.getByRole("button", { name: "Back", exact: true }).click();
      await expect(runna).toBeChecked();
      await dialog.getByRole("button", { name: "Continue", exact: true }).click();
      await dialog.getByRole("button", { name: "Continue", exact: true }).click();
      await page.evaluate(() => { window.__deskFail = "create_plan"; });
      await dialog.getByRole("button", { name: "Build plan", exact: true }).click();
      await expect(dialog.getByRole("alert")).toContainText("Unable to save right now");
      await expect(dialog.locator(".composer-summary")).toContainText("Runna-like");
      await page.evaluate(() => { window.__deskFail = ""; });
      await dialog.getByRole("button", { name: "Build plan", exact: true }).click();
      await expect(dialog).toHaveCount(0);
      expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "create_plan").pop().args.request.trainingMethod)).toBe("runna_like");
      await expect(page.locator(".metrics-line")).toContainText("Runna-like");
    });
  }
}

test("plan creation submits the established contract", async ({ page }) => {
  await page.goto("/desk.html?noPlan=1");
  await page.getByRole("button", { name: "Create plan", exact: true }).click();
  const startDate = await page.getByLabel("Start date", { exact: true }).inputValue();
  const selectedStart = new Date(startDate + "T12:00:00");
  selectedStart.setDate(selectedStart.getDate() + 7);
  const selectedDate = `${selectedStart.getFullYear()}-${String(selectedStart.getMonth() + 1).padStart(2, "0")}-${String(selectedStart.getDate()).padStart(2, "0")}`;
  await page.getByLabel("Start date", { exact: true }).fill(selectedDate);
  await page.getByLabel("Plan name (optional)").fill("Spring half build");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByLabel("Threshold pace (min/km)")).toHaveValue("4:40");
  await page.getByLabel("Current weekly mileage").fill("40");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await page.getByRole("button", { name: "Build plan", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  const request = await page.evaluate(() => window.__deskCalls.find(call => call.command === "create_plan").args.request);
  expect(request).toMatchObject({kind:"race",trainingMethod:"norwegian",distance:"21K",startDate:selectedDate,thresholdSecKm:280,weeklyFrequency:4,currentWeeklyKm:40,name:"Spring half build"});
  await expect(page.locator(".plan-basis")).toContainText("40.0 km/week");
});

for (const width of [390, 1440]) {
  test(`plan start date anchors maintenance duration at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.goto("/desk.html?noPlan=1");
    await page.getByRole("button", { name: "Create plan", exact: true }).click();
    await page.getByLabel("Stay consistent", { exact: true }).check();
    await page.getByLabel("Start date", { exact: true }).fill("2030-03-06");
    await page.getByRole("combobox", { name: "Block length", exact: true }).selectOption("16");
    await expect(page.locator(".composer-summary")).toContainText("06/03/2030");
    await expect(page.locator(".composer-summary")).toContainText("26/06/2030");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: `test-results/plan-start-date-${width}.png`, animations: "disabled" });
    await page.getByRole("button", { name: "Continue", exact: true }).click();
    await page.getByRole("button", { name: "Continue", exact: true }).click();
    await page.getByRole("button", { name: "Build plan", exact: true }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    const request = await page.evaluate(() => window.__deskCalls.find(call => call.command === "create_plan").args.request);
    expect(request).toMatchObject({ kind: "maintenance", startDate: "2030-03-06", targetDate: "2030-06-26" });
  });
}

test("plan start date requires a later end date", async ({ page }) => {
  await page.goto("/desk.html?noPlan=1");
  await page.getByRole("button", { name: "Create plan", exact: true }).click();
  const targetDate = await page.getByLabel("Race date", { exact: true }).inputValue();
  await page.getByLabel("Start date", { exact: true }).fill(targetDate);
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await expect(page.getByLabel("Start date", { exact: true })).toBeVisible();
  expect(await page.getByLabel("Race date", { exact: true }).evaluate(input => input.validity.rangeUnderflow)).toBe(true);
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "create_plan"))).toEqual([]);
});

test("failed plan creation keeps the reviewed draft", async ({ page }) => {
  await page.goto("/desk.html?noPlan=1");
  await page.getByRole("button", { name: "Create plan", exact: true }).click();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await page.getByLabel("Current weekly mileage").fill("37");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  await page.evaluate(() => { window.__deskFail = "create_plan"; });
  await page.getByRole("button", { name: "Build plan", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Unable to save right now");
  await expect(page.locator(".composer-summary")).toContainText("37 km/week");
  await expect(page.getByRole("button", { name: "Build plan", exact: true })).toBeEnabled();
});

for (const width of [320, 390, 768, 1440]) {
  test(`training layout and accessibility at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.goto("/desk.html");
    await expect(page.locator(width > 800 ? ".week-board" : ".day-itinerary")).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
    await page.screenshot({path:`test-results/desk-training-${width}.png`,fullPage:true});
  });
}

for (const width of [390, 1440]) {
  for (const reducedMotion of ["no-preference", "reduce"]) {
    test(`disclosure filters retain values and reverse cleanly ${reducedMotion} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 900 });
      await page.emulateMedia({ reducedMotion });
      await page.goto("/desk.html#journal");
      const toggle = page.locator(".filters-toggle");
      const filters = page.locator("#activity-filters");
      await expect(page.locator(".activity-row")).toHaveCount(9);
      await expect(filters).toHaveAttribute("inert", "");
      await expect(filters).toBeHidden();
      const sampleToggle = async () => {
        await toggle.evaluate(button => {
          window.__filterHeights = null;
          button.addEventListener("click", () => {
            const panel = document.getElementById("activity-filters");
            const samples = [panel.getBoundingClientRect().height];
            const deadline = performance.now() + 600;
            const sample = () => {
              samples.push(panel.getBoundingClientRect().height);
              if (performance.now() < deadline) requestAnimationFrame(sample);
              else window.__filterHeights = samples;
            };
            requestAnimationFrame(sample);
          }, { once: true, capture: true });
        });
      };
      await sampleToggle();
      await toggle.focus();
      await page.keyboard.press("Enter");
      await expect(toggle).toHaveAttribute("aria-expanded", "true");
      await expect.poll(() => page.evaluate(() => window.__filterHeights)).not.toBeNull();
      const heights = await page.evaluate(() => window.__filterHeights);
      const expandedHeight = heights.at(-1);
      expect(expandedHeight).toBeGreaterThan(100);
      expect(heights.some(height => height > 1 && height < expandedHeight - 1)).toBe(reducedMotion === "no-preference");
      await page.screenshot({ path: testInfo.outputPath("filters-open.png") });
      await page.keyboard.press("Tab");
      await expect(page.getByRole("searchbox", { name: "Search activities" })).toBeFocused();
      await page.getByRole("searchbox", { name: "Search activities" }).fill("Run");
      await sampleToggle();
      await toggle.click();
      await expect(filters).toHaveAttribute("inert", "");
      await expect(filters).toHaveAttribute("aria-hidden", "true");
      await expect.poll(() => page.evaluate(() => window.__filterHeights)).not.toBeNull();
      const closingHeights = await page.evaluate(() => window.__filterHeights);
      expect(closingHeights.at(-1)).toBe(0);
      expect(closingHeights.some(height => height > 1 && height < expandedHeight - 1)).toBe(reducedMotion === "no-preference");
      await expect(filters).toBeHidden();
      await toggle.press("Enter");
      await toggle.press("Enter");
      await toggle.press("Enter");
      await expect(toggle).toHaveAttribute("aria-expanded", "true");
      await expect(filters).not.toHaveAttribute("inert", "");
      await expect(page.getByRole("searchbox", { name: "Search activities" })).toHaveValue("Run");
      await expect.poll(() => filters.evaluate(element => getComputedStyle(element).height)).not.toBe("0px");
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    });

    test(`disclosure health details and conversation drafts stay accessible ${reducedMotion} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 900 });
      await page.emulateMedia({ reducedMotion });
      await page.goto("/desk.html#health");
      const toggle = page.locator(".health-detail-toggle");
      const details = page.locator("#health-details");
      await expect(details).toBeHidden();
      await expect(details).toHaveAttribute("inert", "");
      await toggle.click();
      await expect(page.getByRole("heading", { name: "Risk contributors" })).toBeVisible();
      await expect.poll(() => details.evaluate(element => element.getAnimations({ subtree: true }).filter(animation => animation.playState === "running").length)).toBe(0);
      await expect(toggle.locator(".disclosure-indicator")).toHaveCSS("rotate", "180deg");
      await page.screenshot({ path: testInfo.outputPath("health-details-open.png") });
      await details.getByRole("button").first().focus();
      await toggle.evaluate(button => button.click());
      await expect(toggle).toBeFocused();
      await expect(details).toHaveAttribute("inert", "");
      await expect(details).toBeHidden();
      await page.keyboard.press("Tab");
      expect(await details.evaluate(element => element.contains(document.activeElement))).toBe(false);
      await page.getByRole("link", { name: "Pain journal", exact: true }).click();
      await page.getByRole("button", { name: "Log pain", exact: true }).first().click();
      await page.getByRole("combobox", { name: "Body region", exact: true }).selectOption("knee.left");
      await page.getByRole("button", { name: "Save pain entry" }).click();
      await expect(page.getByRole("dialog")).toHaveCount(0);
      const discuss = page.getByRole("button", { name: "Discuss episode" });
      await expect(discuss).toBeVisible();
      await expect(page.locator(".episode-chat")).toHaveCount(0);
      await discuss.click();
      const question = page.getByRole("textbox", { name: "Your question" });
      await question.fill("Keep this unfinished question");
      await page.getByRole("checkbox", { name: /I agree to send/ }).check();
      await discuss.click();
      await expect(page.getByRole("region", { name: "Episode conversation" })).toHaveCount(0);
      await page.keyboard.press("Tab");
      await expect(page.getByRole("button", { name: "Delete", exact: true })).toBeFocused();
      await discuss.click();
      await expect(question).toHaveValue("Keep this unfinished question");
      await expect(page.getByRole("checkbox", { name: /I agree to send/ })).toBeChecked();
      await expect(page.getByRole("button", { name: "Send question" })).toBeEnabled();
      expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "ask_assistant").length)).toBe(0);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    });

    test(`view changes reveal filtered results without transforming their container ${reducedMotion} ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.emulateMedia({ reducedMotion });
      await page.goto("/desk.html#journal");
      await expect(page.locator(".activity-row")).toHaveCount(9);
      await page.getByRole("button", { name: "Filters", exact: true }).click();
      await page.locator(".activity-library").evaluate(library => {
        window.__resultArrival = [];
        window.__resultHost = library.querySelector(".motion-change");
        document.addEventListener("change", () => {
          const sample = () => {
            const node = library.querySelector('.motion-change:not([inert])');
            if (node) {
              const style = getComputedStyle(node);
              const row = node.querySelector(".activity-row");
              window.__resultArrival.push({ opacity: Number(style.opacity), rowOpacity: row ? Number(getComputedStyle(row).opacity) : 1, transform: style.transform, filter: style.filter });
            }
            if (window.__resultArrival.length < 20) requestAnimationFrame(sample);
          };
          requestAnimationFrame(sample);
        }, { once: true });
      });
      await page.getByRole("combobox", { name: "Sport", exact: true }).selectOption("Run");
      await expect(page.locator(".activity-row")).toHaveCount(6);
      await expect.poll(() => page.evaluate(() => window.__resultArrival.length)).toBe(20);
      await expect.poll(() => page.evaluate(() => window.__resultArrival.at(-1)?.opacity)).toBe(1);
      const frames = await page.evaluate(() => window.__resultArrival);
      expect(frames.every(frame => frame.opacity === 1)).toBe(true);
      expect(frames.some(frame => frame.rowOpacity > 0 && frame.rowOpacity < 1)).toBe(reducedMotion === "no-preference");
      expect(frames.every(frame => frame.transform === "none" && frame.filter === "none")).toBe(true);
      if (width <= 800) expect(await page.locator(".activity-library > .motion-change").evaluate(element => element === window.__resultHost)).toBe(true);
      await page.getByRole("button", { name: "Reset filters" }).click();
      await page.getByRole("combobox", { name: "Source", exact: true }).selectOption("garmin");
      await page.getByRole("button", { name: "Reset filters" }).click();
      await expect(page.locator(".activity-row")).toHaveCount(9);
      await expect(page.locator(".activity-library > .motion-change")).toHaveCount(1);
      await expect(page.locator(".activity-library > .motion-change")).not.toHaveAttribute("inert", "");
    });
  }
}

test("activity splits display signed net elevation", async ({ page }) => {
  await page.addInitScript(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, args) => {
      const result = await invoke(command, args);
      if (command !== "activity_review") return result;
      const elevations = [12, -8, 0, null];
      return { ...result, splits: elevations.map((elevationGainM, index) => ({ ...result.splits[0], km: index + 1, elevationGainM })) };
    };
  });
  await page.goto("/desk.html#journal/1/splits");
  const cells = page.getByRole("region", { name: "Kilometre splits" }).locator("tbody tr td:nth-of-type(3)");
  await expect(cells).toHaveText(["+12 m", "-8 m", "0 m", "Not recorded"]);
});

test("Activities list hides filters until revealed and opens recorded splits", async ({ page }) => {
  await page.goto("/desk.html#journal");
  await expect(page.locator(".activity-row")).toHaveCount(9);
  await expect(page.locator("#activity-filters")).toBeHidden();
  await page.getByRole("button", { name: "Filters", exact: true }).click();
  await expect(page.getByRole("combobox", { name: "Sport", exact: true })).toBeVisible();
  await page.getByRole("combobox", { name: "Sport", exact: true }).selectOption("Run");
  await expect(page.locator(".activity-row")).toHaveCount(6);
  await page.getByLabel("Search activities").fill("not-a-run");
  await expect(page.getByRole("heading", { name: "No matching activities" })).toBeVisible();
  await page.getByRole("button", { name: "Reset filters" }).click();
  await expect(page.locator(".activity-row")).toHaveCount(9);
  await page.locator(".activity-row").first().click();
  await expect(page.getByRole("complementary", { name: "Activity review" })).toBeVisible();
  await page.getByRole("tab", { name: "Splits", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Kilometre splits" })).toBeVisible();
  // Each kilometre wears a horizontal bar scaled against the fastest kilometre
  // of the run, so the fastest row is full width and every slower row shorter.
  const bars = page.getByRole("region", { name: "Kilometre splits" }).locator(".split-bar > span");
  expect(await bars.count()).toBeGreaterThan(1);
  const widths = await bars.evaluateAll(nodes => nodes.map(node => node.getBoundingClientRect().width));
  expect(Math.max(...widths)).toBeGreaterThan(Math.min(...widths) * 1.05);
  await expect(page.getByRole("columnheader", { name: "Elevation" })).toBeVisible();
  await expect(page.getByRole("region", { name: "Kilometre splits" }).locator("tbody tr").first().locator("td").nth(2)).toContainText(/m|Not recorded/);
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect(page.locator(".activity-row").first()).toBeFocused();
});

test("pain logging retains failure drafts and episode conversations survive refresh", async ({ page }) => {
  await page.goto("/desk.html#health/journal");
  await page.getByRole("button", { name: "Log pain", exact: true }).first().click();
  await page.getByRole("combobox", { name: "Body region", exact: true }).selectOption("knee.left");
  await page.getByRole("textbox", { name: "Notes", exact: true }).fill("Synthetic test: soreness after running");
  await page.evaluate(() => { window.__deskFail = "save_pain_entry"; });
  await page.getByRole("button", { name: "Save pain entry" }).click();
  await expect(page.getByRole("alert")).toContainText("Unable to save right now");
  await expect(page.getByRole("textbox", { name: "Notes", exact: true })).toHaveValue("Synthetic test: soreness after running");
  await page.evaluate(() => { window.__deskFail = ""; });
  await page.getByRole("button", { name: "Save pain entry" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator(".pain-episode")).toContainText("Left knee");
  await page.getByRole("button", { name: "Discuss episode" }).click();
  await page.getByRole("textbox", { name: "Your question" }).fill("Should I take a rest day?");
  await expect(page.getByRole("button", { name: "Send question" })).toBeDisabled();
  await page.getByRole("checkbox", { name: /I agree to send/ }).check();
  await page.getByRole("button", { name: "Send question" }).click();
  await expect(page.locator(".chat-message.assistant")).toContainText("reducing the session");
  await expect(page.getByRole("textbox", { name: "Your question" })).toHaveValue("");
  await page.getByRole("button", { name: "Mark resolved" }).click();
  await expect(page.locator(".pain-episode")).toHaveCount(0);
  await page.getByRole("checkbox", { name: "Include resolved" }).check();
  await page.getByRole("button", { name: "Reopen", exact: true }).click();
  await expect(page.locator(".pain-episode")).toContainText("Active");
});

for (const route of ["journal", "health", "health/milestones"]) {
  test(`${route} loads backend evidence and meets accessibility checks`, async ({ page }) => {
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(`/desk.html#${route}`);
    await expect(page.locator(".desk-loading")).toHaveCount(0);
    await expect(page.locator(".error-notice")).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
    expect(errors).toEqual([]);
  });
}

for (const width of [320,390,1440]) for (const theme of ["light","dark"]) {
  test(`settings connection group fits ${width}px ${theme}`, async ({page}) => {
    await page.setViewportSize({width,height:900});
    await page.goto(`/desk.html?theme=${theme}#settings`);
    const group=page.getByRole("region",{name:"Third-party connections",exact:true});
    await expect(group.getByRole("link")).toHaveCount(6);
    await expect(page.locator(".settings-group > h2")).toHaveText(["Third-party connections","General"]);
    await expect(group.getByRole("link").locator("strong")).toHaveText(["Watch connection","Transfer connections","Device sync","Calendar apps","Strava","Injury assistant"]);
    for(const route of ["watch","strava","calendar","devices","transfer","assistant"]) await expect(group.locator(`a[href="#settings/${route}"]`)).toBeVisible();
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    expect((await new AxeBuilder({page}).include(".settings-directory").analyze()).violations).toEqual([]);
    await page.screenshot({path:test.info().outputPath(`settings-groups-${width}-${theme}.png`),fullPage:true});
    await page.getByRole("searchbox",{name:"Search settings"}).fill("third-party");
    await expect(group.getByRole("link")).toHaveCount(6);
    await expect(page.getByRole("region",{name:"General",exact:true})).toHaveCount(0);
    await page.getByRole("searchbox",{name:"Search settings"}).fill("no-such-setting");
    await expect(page.getByText("No matching settings",{exact:true})).toBeVisible();
    await expect(group).toHaveCount(0);
  });
}

test("settings directory searches tasks and preserves unrelated settings", async ({ page }) => {
  await page.goto("/desk.html#settings");
  await page.getByRole("searchbox", { name: "Search settings" }).fill("colour");
  await expect(page.locator(".settings-task-list a")).toHaveCount(1);
  await page.getByRole("link", { name: /Appearance/ }).click();
  await page.getByRole("radio", { name: "Dark", exact: true }).check();
  await expect(page.locator("html")).toHaveAttribute("data-desk-theme", "dark");
  const settings = await page.evaluate(() => window.__deskCalls.find(call => call.command === "save_settings").args.settings);
  expect(settings.llmEndpoint).toBe("https://example.test/v1");
  expect(settings.thresholds.buildWeeks).toBe(3);
  await page.getByRole("radio", { name: "Ice Ice Pacey", exact: true }).check();
  await expect(page.locator("html")).toHaveAttribute("data-desk-palette", "glacier");
});

test("calendar settings never publish on read and retain blank credentials and sync metadata", async ({ page }) => {
  await page.goto("/desk.html#settings/calendar");
  await page.getByRole("button", { name: "Connect a calendar", exact: true }).click();
  await expect(page.getByRole("button", { name: "Save calendar settings" })).toBeVisible();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "publish_calendar"))).toEqual([]);
  await page.getByRole("checkbox", { name: "Publish calendar remotely" }).check();
  await page.getByRole("button", { name: "Save calendar settings" }).click();
  await expect(page.getByRole("status").last()).toContainText("Calendar settings saved");
  const payload = await page.evaluate(() => window.__deskCalls.find(call => call.command === "save_calendar_publishing").args);
  expect(payload.password).toBeNull();
  expect(payload.config.lastPublished).toBe("2026-10-29T12:00:00Z");
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "publish_calendar"))).toBe(false);
  await expect(page.getByRole("checkbox", { name: "Sync this plan across devices" })).toHaveCount(0);
  await page.goto("/desk.html#settings/devices");
  await page.getByRole("checkbox", { name: "Sync this plan across devices" }).uncheck();
  const sync = await page.evaluate(() => window.__deskCalls.find(call => call.command === "plan_sync_configure").args);
  expect(sync).toEqual({config:{enabled:false},token:null});
  await expect(page.getByText("Device sync paused", {exact:true})).toBeVisible();
  await expect(page.getByRole("button", { name: "Import plan", exact:true })).toHaveCount(0);
  await page.getByRole("button", { name:"More options", exact:true }).click();
  await page.getByRole("button", { name:"Sync diagnostics", exact:true }).click();
  await expect(page.getByText("synthetic-account-channel", {exact:true})).toBeVisible();
});

test("connection transfer QR encrypts credentials and enables device sync after review", async ({page}) => {
  await page.goto("/desk.html?calendarReady=1#settings/transfer");
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "export_connection_settings"))).toBe(false);
  await page.getByRole("button", {name:"Create transfer QR", exact:true}).click();
  let dialog = page.getByRole("dialog", {name:"Create connection transfer", exact:true});
  await dialog.getByLabel("Transfer passphrase", {exact:true}).fill("synthetic transfer passphrase");
  await dialog.getByRole("button", {name:"Generate encrypted QR", exact:true}).click();
  const image = dialog.getByRole("img", {name:"Encrypted connection transfer QR code"});
  await expect(image).toBeVisible();
  const source = await image.getAttribute("src");
  const png = Buffer.from(source.split(",")[1], "base64");
  const decoded = await page.evaluate(async () => {
    const {default:jsQR} = await import("/node_modules/.vite/deps/jsqr.js");
    const image = document.querySelector(".transfer-qr");
    const canvas = document.createElement("canvas"); canvas.width=image.naturalWidth; canvas.height=image.naturalHeight;
    const context=canvas.getContext("2d"); context.drawImage(image,0,0);
    const pixels=context.getImageData(0,0,canvas.width,canvas.height);
    return jsQR(pixels.data,pixels.width,pixels.height).data;
  });
  expect(decoded).toMatch(/^runalytics:connections:1:/);
  expect(decoded).not.toContain("synthetic-github-token");
  expect(decoded).not.toContain("synthetic-calendar-secret");
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await page.getByRole("button", {name:"Import transfer QR", exact:true}).click();
  dialog=page.getByRole("dialog", {name:"Import connection transfer", exact:true});
  await dialog.getByLabel("QR image", {exact:true}).setInputFiles({name:"connections.png",mimeType:"image/png",buffer:png});
  await expect(dialog.getByRole("status")).toHaveText("QR code loaded.");
  await dialog.getByLabel("Transfer passphrase", {exact:true}).fill("incorrect transfer phrase");
  await dialog.getByRole("button", {name:"Review settings", exact:true}).click();
  await expect(dialog.getByRole("alert")).toContainText("Could not unlock");
  await dialog.getByLabel("Transfer passphrase", {exact:true}).fill("synthetic transfer passphrase");
  await dialog.getByRole("button", {name:"Review settings", exact:true}).click();
  await expect(dialog.getByRole("heading", {name:"Replace connection settings?"})).toBeVisible();
  await expect(dialog).toContainText("synthetic");
  await expect(dialog).toContainText("12345");
  await expect(dialog).toContainText("Strava Client Secret");
  await expect(dialog).not.toContainText("synthetic-strava-secret");
  await expect(dialog).not.toContainText("synthetic-github-token");
  await expect(dialog).not.toContainText("synthetic-calendar-secret");
  await expect(dialog).not.toContainText("synthetic-assistant-secret");
  await expect(dialog).toContainText("COROS account matches this device.");
  await expect(dialog).toContainText("https://assistant.example.test/v1");
  await expect(dialog).toContainText("transfer-model");
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "strava_configure"))).toBe(false);
  await page.screenshot({path:test.info().outputPath("transfer-import-review.png"),fullPage:true});
  await dialog.getByRole("checkbox",{name:"I trust this QR source and understand the account could not be fully verified."}).check();
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "save_calendar_publishing" || call.command === "plan_sync_configure"))).toBe(false);
  await dialog.getByRole("button", {name:"Replace connections", exact:true}).click();
  await expect(dialog).toHaveCount(0);
  const calls=await page.evaluate(() => window.__deskCalls);
  expect(calls.find(call => call.command === "save_calendar_publishing").args.password).toBe("synthetic-calendar-secret");
  expect(calls.find(call => call.command === "save_calendar_publishing").args.config.enabled).toBe(false);
  const syncCalls=calls.filter(call => call.command === "plan_sync_configure");
  expect(syncCalls.at(-2).args).toEqual({config:{enabled:false},token:"synthetic-github-token"});
  expect(syncCalls.at(-1).args).toEqual({config:{enabled:true},token:null});
  expect(calls.find(call => call.command === "strava_configure").args).toEqual({clientId:"12345",clientSecret:"synthetic-strava-secret"});
  expect(calls.find(call => call.command === "set_llm_api_key").args).toEqual({key:"synthetic-assistant-secret"});
  const importedSettings=calls.find(call=>call.command==="save_settings").args.settings;
  expect(importedSettings.llmEnabled).toBe(false);
  expect(importedSettings.llmEndpoint).toBe("https://assistant.example.test/v1");
  expect(importedSettings.llmModel).toBe("transfer-model");
});

test("connection transfer camera denial retains image upload fallback", async ({page}) => {
  await page.addInitScript(() => { Object.defineProperty(navigator,"userAgent",{get:()=>"iPhone"}); navigator.mediaDevices.getUserMedia=async () => { throw new DOMException("Denied", "NotAllowedError"); }; });
  await page.goto("/desk.html#settings/transfer");
  await page.getByRole("button", {name:"Import transfer QR", exact:true}).click();
  const dialog=page.getByRole("dialog", {name:"Import connection transfer"});
  await dialog.getByRole("button", {name:"Scan with camera", exact:true}).click();
  await expect(dialog.getByRole("alert")).toContainText("Upload a QR image instead");
  await expect(dialog.getByRole("button",{name:"Scan with camera",exact:true})).toHaveCount(0);
  await expect(dialog.getByLabel("QR image", {exact:true})).toBeVisible();
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "export_connection_settings"))).toBe(false);
});

test("connection transfer requires a connected watch", async ({page}) => {
  await page.goto("/desk.html#settings/watch");
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke("disconnect_provider", {provider:"coros"});
    location.hash="settings/transfer";
  });
  await expect(page.getByText("Before importing or exporting connections, sign in to your watch account. Use the same COROS account on both devices.")).toBeVisible();
  await expect(page.getByRole("button",{name:"Create transfer QR",exact:true})).toBeDisabled();
  await expect(page.getByRole("button",{name:"Import transfer QR",exact:true})).toBeDisabled();
  await expect(page.getByRole("link",{name:"Connect your watch",exact:true})).toHaveAttribute("href","#settings/watch");
  expect(await page.evaluate(()=>window.__deskCalls.some(call=>call.command === "export_connection_settings"))).toBe(false);
});

test("connection transfer rechecks watch before exporting credentials", async ({page}) => {
  await page.goto("/desk.html#settings/transfer");
  await page.getByRole("button",{name:"Create transfer QR",exact:true}).click();
  const dialog=page.getByRole("dialog",{name:"Create connection transfer"});
  await dialog.getByLabel("Transfer passphrase",{exact:true}).fill("synthetic transfer passphrase");
  await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke("disconnect_provider",{provider:"coros"}));
  await dialog.getByRole("button",{name:"Generate encrypted QR",exact:true}).click();
  await expect(dialog.getByRole("alert")).toContainText("Sign in to your watch account");
  expect(await page.evaluate(()=>window.__deskCalls.some(call=>call.command === "export_connection_settings"))).toBe(false);
});

for (const scenario of ["matching", "coros-mismatch", "strava-mismatch", "legacy", "changed-after-review", "watch-disconnected"]) {
  test(`connection transfer account guard ${scenario}`, async ({page}) => {
    await page.goto("/desk.html#settings/transfer");
    const data=await page.evaluate(async scenario => {
      const {sealConnections}=await import("/src/desk/connection-transfer.tsx");
      const {default:QRCode}=await import("/node_modules/.vite/deps/qrcode.js");
      const source={version:1,created:1,calendar:{uploadUrl:"https://example.test/calendar.ics",subscriptionUrl:"https://example.test/calendar.ics",username:"runner",password:null},githubToken:"synthetic-transfer-token",identity:{corosCode:scenario === "coros-mismatch" ? "different-coros-account" : "synthetic-account-channel"},strava:{clientId:"12345",clientSecret:"synthetic-client-secret",athleteId:42}};
      if(scenario === "legacy"){delete source.identity;delete source.strava;}
      const invoke=window.__TAURI_INTERNALS__.invoke;
      window.__transferAccountChanged=false;
      window.__TAURI_INTERNALS__.invoke=async (command,args)=>{
        if(command === "strava_status")return {...await invoke(command,args),connected:true,athleteId:scenario === "strava-mismatch" ? 99 : 42};
        if(command === "plan_sync_status" && window.__transferAccountChanged)return {...await invoke(command,args),code:"changed-after-review"};
        return invoke(command,args);
      };
      const code=await sealConnections(source,"synthetic transfer passphrase");
      return QRCode.toDataURL(code,{width:1000,margin:4});
    },scenario);
    await page.getByRole("button",{name:"Import transfer QR",exact:true}).click();
    const dialog=page.getByRole("dialog",{name:"Import connection transfer"});
    await dialog.getByLabel("QR image",{exact:true}).setInputFiles({name:"identity.png",mimeType:"image/png",buffer:Buffer.from(data.split(",")[1],"base64")});
    await expect(dialog.getByRole("status")).toHaveText("QR code loaded.");
    await dialog.getByLabel("Transfer passphrase",{exact:true}).fill("synthetic transfer passphrase");
    await dialog.getByRole("button",{name:"Review settings",exact:true}).click();
    const replace=dialog.getByRole("button",{name:"Replace connections",exact:true});
    const writes=()=>page.evaluate(()=>window.__deskCalls.filter(call=>["save_calendar_publishing","plan_sync_configure","strava_configure","set_llm_api_key","save_settings"].includes(call.command)));
    expect(await writes()).toEqual([]);
    if(scenario.endsWith("mismatch")) {
      await expect(dialog.getByRole("alert")).toContainText("Import blocked");
      await expect(replace).toBeDisabled();
    } else if(scenario === "legacy") {
      await expect(replace).toBeDisabled();
      await expect(dialog).toContainText("existing Strava setup stays");
      await dialog.getByRole("checkbox",{name:/I trust this QR source/}).check();
      await expect(replace).toBeEnabled();
    } else if(scenario === "watch-disconnected") {
      await expect(replace).toBeEnabled();
      await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke("disconnect_provider",{provider:"coros"}));
      await replace.click();
      await expect(dialog.getByRole("alert").first()).toContainText("Sign in to your watch account");
      expect(await writes()).toEqual([]);
    } else if(scenario === "changed-after-review") {
      await expect(replace).toBeEnabled();
      await page.evaluate(()=>{window.__transferAccountChanged=true;});
      await replace.click();
      await expect(dialog.getByRole("alert").first()).toContainText("Accounts changed");
      expect(await writes()).toEqual([]);
    } else {
      await expect(dialog).toContainText("COROS account matches this device.");
      await expect(dialog).toContainText("Strava athlete 42 matches this device.");
      await expect(replace).toBeEnabled();
      await expect(dialog.getByRole("checkbox",{name:/I trust this QR source/})).toHaveCount(0);
    }
  });
}

test("connection transfer rejects unsupported and tampered encrypted settings", async ({page}) => {
  await page.goto("/desk.html#settings/transfer");
  const results=await page.evaluate(async () => {
    const {sealConnections,openConnections}=await import("/src/desk/connection-transfer.tsx");
    const valid={version:1,created:1,calendar:{uploadUrl:"https://example.test/feed",subscriptionUrl:"https://example.test/feed",username:"runner",password:null},githubToken:null};
    const phrase="synthetic transfer passphrase";
    const payloads=["not a Runalytics code",await sealConnections({...valid,version:2},phrase),await sealConnections({...valid,calendar:{...valid.calendar,uploadUrl:"http://example.test/feed"}},phrase),await sealConnections({...valid,strava:{clientId:"invalid",clientSecret:"secret"}},phrase)];
    const good=await sealConnections(valid,phrase);
    const legacy=await openConnections(good,phrase);
    if (legacy.strava !== undefined || legacy.calendar.username !== "runner") throw new Error("Legacy transfer changed");
    payloads.push(good.slice(0,-5)+"AAAAA");
    return Promise.all(payloads.map(async code => {try{await openConnections(code,phrase);return false;}catch{return true;}}));
  });
  expect(results).toEqual([true,true,true,true,true]);
});

test("connection transfer lives in its own searchable settings destination", async ({page}) => {
  for(const route of ["calendar","devices"]) {
    await page.goto(`/desk.html#settings/${route}`);
    await expect(page.getByRole("button",{name:"Create transfer QR",exact:true})).toHaveCount(0);
    await expect(page.getByRole("button",{name:"Import transfer QR",exact:true})).toHaveCount(0);
  }
  await page.goto("/desk.html#settings");
  await page.getByRole("searchbox",{name:"Search settings"}).fill("transfer");
  await page.getByRole("link",{name:/Transfer connections/}).click();
  await expect(page).toHaveURL(/#settings\/transfer$/);
  await expect(page.getByRole("button",{name:"Import transfer QR",exact:true})).toBeVisible();
});

test("connection transfer camera decodes locally and stops capture after scanning and close", async ({page}) => {
  await page.addInitScript(() => { Object.defineProperty(navigator,"userAgent",{get:()=>"iPhone"}); });
  await page.goto("/desk.html#settings/transfer");
  await page.evaluate(async () => {
    const {default:QRCode}=await import("/node_modules/.vite/deps/qrcode.js");
    const canvas=document.createElement("canvas");
    await QRCode.toCanvas(canvas,"runalytics:connections:1:synthetic-camera-code",{width:600});
    window.__cameraStops=0;
    navigator.mediaDevices.getUserMedia=async () => {
      const stream=canvas.captureStream(5);
      const timer=setInterval(()=>{canvas.getContext("2d").fillRect(0,0,1,1);},100);
      for(const track of stream.getTracks()){const stop=track.stop.bind(track);track.stop=()=>{clearInterval(timer);window.__cameraStops+=1;stop();};}
      return stream;
    };
  });
  await page.getByRole("button",{name:"Import transfer QR",exact:true}).click();
  let dialog=page.getByRole("dialog",{name:"Import connection transfer"});
  await dialog.getByRole("button",{name:"Scan with camera",exact:true}).click();
  await expect(dialog.getByRole("status")).toHaveText("QR code loaded.");
  await expect.poll(()=>page.evaluate(()=>window.__cameraStops)).toBe(1);
  await expect(dialog.locator("video")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await page.evaluate(() => {
    navigator.mediaDevices.getUserMedia=async () => {
      const canvas=document.createElement("canvas");canvas.width=400;canvas.height=400;
      const stream=canvas.captureStream(5);
      const timer=setInterval(()=>{canvas.getContext("2d").fillRect(0,0,1,1);},100);
      for(const track of stream.getTracks()){const stop=track.stop.bind(track);track.stop=()=>{clearInterval(timer);window.__cameraStops+=1;stop();};}
      return stream;
    };
  });
  await page.getByRole("button",{name:"Import transfer QR",exact:true}).click();
  dialog=page.getByRole("dialog",{name:"Import connection transfer"});
  await dialog.getByRole("button",{name:"Scan with camera",exact:true}).click();
  await expect(dialog.locator("video")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect.poll(()=>page.evaluate(()=>window.__cameraStops)).toBe(2);
});

for(const width of [320,390,1440]) for(const theme of ["light","dark"]) {
  test(`connection transfer dialog fits ${width}px ${theme}`,async ({page})=>{
    await page.setViewportSize({width,height:844});
    await page.goto(`/desk.html?theme=${theme}#settings/transfer`);
    await page.getByRole("button",{name:"Create transfer QR",exact:true}).click();
    const dialog=page.getByRole("dialog",{name:"Create connection transfer"});
    await dialog.getByLabel("Transfer passphrase",{exact:true}).fill("synthetic transfer passphrase");
    await dialog.getByRole("button",{name:"Generate encrypted QR",exact:true}).click();
    await expect(dialog.getByRole("img")).toBeVisible();
    await expect(dialog.getByRole("region",{name:"Transfer contents"}).getByRole("listitem")).toHaveCount(6);
    const qr = await dialog.getByRole("img",{name:"Encrypted connection transfer QR code",exact:true}).evaluate(async image => {
      await image.decode();
      const {default:jsQR}=await import("/node_modules/.vite/deps/jsqr.js");
      const canvas=document.createElement("canvas");canvas.width=image.naturalWidth;canvas.height=image.naturalHeight;
      const context=canvas.getContext("2d");context.drawImage(image,0,0);
      const pixels=context.getImageData(0,0,canvas.width,canvas.height);
      return {code:jsQR(pixels.data,pixels.width,pixels.height)?.data,background:[...pixels.data.slice(0,3)]};
    });
    expect(qr.code).toMatch(/^runalytics:connections:1:/);
    expect(qr.background).toEqual(theme === "dark" ? [36,38,44] : [255,255,255]);
    const footerGap=await dialog.locator(".transfer-export-footer").evaluate(element=>{
      const [link,text]=[...element.children].map(child=>child.getBoundingClientRect());
      return text.top-link.bottom;
    });
    expect(footerGap).toBeGreaterThanOrEqual(16);
    expect(await dialog.evaluate(element=>element.scrollWidth<=element.clientWidth+1)).toBe(true);
    expect((await new AxeBuilder({page}).include(".desk-dialog").analyze()).violations).toEqual([]);
    await page.screenshot({path:test.info().outputPath(`qr-${width}-${theme}.png`),fullPage:true});
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await page.getByRole("button",{name:"Import transfer QR",exact:true}).click();
    const importer=page.getByRole("dialog",{name:"Import connection transfer"});
    await expect(importer.getByRole("button",{name:"Scan with camera",exact:true})).toHaveCount(0);
    const formGaps=await importer.locator("form").evaluate(element=>{
      const children=[...element.children].map(child=>child.getBoundingClientRect());
      return children.slice(1).map((child,index)=>child.top-children[index].bottom);
    });
    expect(formGaps.every(gap=>gap>=20)).toBe(true);
    expect(await importer.evaluate(element=>element.scrollWidth<=element.clientWidth+1)).toBe(true);
    expect((await new AxeBuilder({page}).include(".desk-dialog").analyze()).violations).toEqual([]);
    await page.screenshot({path:test.info().outputPath(`qr-import-${width}-${theme}.png`),fullPage:true});
  });
}

test("calendar connections separate subscription actions from device setup", async ({ page }) => {
  await page.goto("/desk.html?calendarReady=1#settings/calendar");
  await expect(page.getByRole("button", {name:"Open in calendar app", exact:true})).toBeVisible();
  await expect(page.getByLabel("Upload URL", {exact:true})).toBeHidden();
  await expect(page.getByRole("button", {name:"Copy calendar link", exact:true})).toBeHidden();
  await expect(page.getByRole("checkbox", {name:"Sync this plan across devices"})).toHaveCount(0);
  await page.getByRole("button", {name:"More options", exact:true}).click();
  await expect(page.getByRole("button", {name:"Copy calendar link", exact:true})).toBeVisible();
  await page.getByRole("button", {name:"Stop publishing", exact:true}).click();
  const dialog = page.getByRole("dialog", {name:"Stop calendar publishing?", exact:true});
  await expect(dialog).toContainText("existing feed and saved credentials are kept");
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "save_calendar_publishing"))).toBe(false);
  await dialog.getByRole("button", {name:"Keep publishing", exact:true}).click();
  await expect(dialog).toHaveCount(0);
  await page.getByRole("button", {name:"Stop publishing", exact:true}).click();
  await dialog.getByRole("button", {name:"Stop publishing", exact:true}).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByText("Calendar publishing is off", {exact:true})).toBeVisible();
  const saved = await page.evaluate(() => window.__deskCalls.find(call => call.command === "save_calendar_publishing").args);
  expect(saved.password).toBeNull();
  expect(saved.config.enabled).toBe(false);
  expect(saved.config.lastPublished).toBe("2026-10-29T12:00:00Z");
});

test("calendar connections retain failed drafts and offer publishing retry", async ({ page }) => {
  await page.goto("/desk.html#settings/calendar");
  await page.getByRole("button", {name:"Connect a calendar", exact:true}).click();
  await page.getByLabel("Upload URL", {exact:true}).fill("https://example.test/draft.ics");
  await page.getByLabel("Password or token", {exact:true}).fill("synthetic-calendar-token");
  await page.evaluate(() => { window.__deskFail="save_calendar_publishing"; });
  await page.getByRole("button", {name:"Save calendar settings", exact:true}).click();
  await expect(page.getByRole("alert")).toContainText("Unable to save right now");
  await expect(page.getByLabel("Upload URL", {exact:true})).toHaveValue("https://example.test/draft.ics");
  await expect(page.getByLabel("Password or token", {exact:true})).toHaveValue("synthetic-calendar-token");
  await page.goto("/desk.html?calendarError=1#settings/calendar");
  await expect(page.getByText("Calendar host unavailable", {exact:true})).toBeVisible();
  await page.getByRole("button", {name:"Retry publishing", exact:true}).click();
  await expect(page.getByText("Calendar feed available", {exact:true})).toBeVisible();
  await expect(page.getByRole("button", {name:"Open in calendar app", exact:true})).toBeVisible();
});

test("calendar connections never report a failed upload as published", async ({page}) => {
  await page.goto("/desk.html?calendarError=1&calendarUploadFails=1#settings/calendar");
  await page.getByRole("button", {name:"Retry publishing", exact:true}).click();
  await expect(page.getByRole("alert")).toContainText("Calendar upload failed");
  await expect(page.getByRole("status")).not.toContainText("Calendar feed published.");
  await expect(page.getByRole("button", {name:"Retry publishing", exact:true})).toBeVisible();
});

for (const [state, action] of [["unpublished", "Sync now"], ["updateAvailable", "Update this device"], ["importAvailable", "Get my plan"], ["otherPlan", "Review plans"]]) {
  test(`device connections select one main action for ${state}`, async ({page}) => {
    await page.goto(`/desk.html?syncState=${state}${state === "otherPlan" ? "&syncConflict=1" : ""}#settings/devices`);
    await expect(page.locator(".settings-editor .desk-button.primary:visible")).toHaveCount(1);
    await expect(page.getByRole("button", {name:action, exact:true})).toBeVisible();
    await expect(page.getByRole("button", {name:"Publish plan", exact:true})).toHaveCount(0);
    await expect(page.getByRole("button", {name:"Import plan", exact:true})).toHaveCount(0);
    await page.getByRole("button", {name:action, exact:true}).click();
    if (state !== "unpublished") {
      await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_pull").map(call => call.args.force))).toEqual([false]);
    }
    if (state === "otherPlan") await expect(page.getByRole("dialog", {name:"Replace the plan on this device?"})).toBeVisible();
  });
}

test("device connections save credentials independently and retain failed drafts", async ({page}) => {
  await page.goto("/desk.html?syncSetup=1#settings/devices");
  await page.getByRole("button", {name:"Set up device sync", exact:true}).click();
  await page.getByLabel("GitHub token", {exact:true}).fill("synthetic-device-token");
  await page.evaluate(() => { window.__deskFail="plan_sync_configure"; });
  await page.getByRole("button", {name:"Save device connection", exact:true}).click();
  await expect(page.getByRole("alert")).toContainText("Unable to save right now");
  await expect(page.getByLabel("GitHub token", {exact:true})).toHaveValue("synthetic-device-token");
  await page.evaluate(() => { window.__deskFail=""; });
  await page.getByRole("button", {name:"Save device connection", exact:true}).click();
  await expect(page.getByText("Plan up to date", {exact:true})).toBeVisible();
  await expect(page.getByLabel("GitHub token", {exact:true})).toHaveValue("");
  const calls = await page.evaluate(() => window.__deskCalls);
  expect(calls.filter(call => call.command === "plan_sync_configure").at(-1).args).toEqual({config:{enabled:true},token:"synthetic-device-token"});
  expect(calls.some(call => call.command === "save_calendar_publishing" || call.command === "publish_calendar")).toBe(false);
});

test("device connections keep settings accessible when checks fail or prerequisites are missing", async ({page}) => {
  await page.addInitScript(() => { window.__deskFail="plan_sync_check"; });
  await page.goto("/desk.html#settings/devices");
  await expect(page.getByRole("button", {name:"Retry sync", exact:true})).toBeVisible();
  await page.getByRole("button", {name:"More options", exact:true}).click();
  await page.getByRole("button", {name:"Connection settings", exact:true}).click();
  await expect(page.getByLabel("GitHub token", {exact:true})).toBeVisible();
  await page.goto("/desk.html?noAccount=1#settings/devices");
  await expect(page.getByRole("link", {name:"Connect COROS"})).toHaveAttribute("href", "#settings/watch");
  await expect(page.getByRole("button", {name:"Retry sync", exact:true})).toHaveCount(0);
  await page.goto("/desk.html?syncPaused=1#settings/devices");
  await expect(page.getByText("Device sync paused", {exact:true})).toBeVisible();
});

for (const width of [320,390,768,820,1440]) for (const theme of ["light","dark"]) {
  test(`calendar connections fit ${width}px ${theme}`, async ({page}) => {
    await page.setViewportSize({width,height:900});
    await page.goto(`/desk.html?calendarReady=1&theme=${theme}#settings/calendar`);
    const more = page.getByRole("button", {name:"More options", exact:true});
    const collapsedBackground = await more.evaluate(element => getComputedStyle(element).backgroundColor);
    await page.getByRole("button", {name:"More options", exact:true}).click();
    await expect(more).toHaveAttribute("aria-expanded", "true");
    await expect(more.locator(".disclosure-indicator")).toHaveCSS("rotate", "180deg");
    await expect.poll(() => more.evaluate(element => getComputedStyle(element).backgroundColor)).not.toBe(collapsedBackground);
    await page.getByRole("button", {name:"Connection settings", exact:true}).click();
    const connection = page.getByRole("button", {name:"Connection settings", exact:true});
    await expect(connection).toHaveAttribute("aria-expanded", "true");
    await expect(connection.locator(".disclosure-indicator")).toHaveCSS("rotate", "180deg");
    const gaps = await page.locator("#calendar-options .connection-options").evaluate(element => {
      const children = [...element.children].map(child => child.getBoundingClientRect());
      return children.slice(1).map((child, index) => child.top - children[index].bottom);
    });
    expect(gaps.every(gap => gap >= 16)).toBe(true);
    await expect(page.getByLabel("Upload URL", {exact:true})).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect((await new AxeBuilder({page}).include(".settings-editor").analyze()).violations).toEqual([]);
    await page.screenshot({path:test.info().outputPath(`calendar-${width}-${theme}.png`),fullPage:true});
    await page.getByRole("button", {name:"Connection settings", exact:true}).click();
    await expect(connection).toHaveAttribute("aria-expanded", "false");
    await expect(connection.locator(".disclosure-indicator")).toHaveCSS("rotate", "0deg");
    await expect(page.getByLabel("Upload URL", {exact:true})).toBeHidden();
    await page.keyboard.press("Tab");
    expect(await page.evaluate(() => document.activeElement?.closest("#calendar-setup") !== null)).toBe(false);
  });
}

test("clearing training data needs confirmation and never touches recorded history", async ({ page }) => {
  await page.goto("/desk.html#settings/training");
  await expect(page.getByRole("link", { name: "Only end the current plan" })).toHaveAttribute("href", "#training/plan");
  await expect(page.getByRole("button", { name: "Clear plans and scheduled training", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Clear plans and scheduled training", exact: true }).click();
  const confirmation = page.getByRole("dialog");
  await expect(confirmation).toContainText("This cannot be undone.");
  await expect(confirmation).toContainText("This device's database:");
  await expect(confirmation).toContainText("past and upcoming");
  await expect(confirmation).toContainText("Other devices receive the clear on their next successful sync");
  await expect(confirmation).toContainText("The Gist itself is not deleted");
  await expect(confirmation).toContainText("previously imported calendar snapshots are not removed");
  await expect(confirmation).toContainText("Recorded activities, GPS tracks, health history, pain entries and connection credentials stay");
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "clear_local_plan_data"))).toEqual([]);
  await page.getByRole("button", { name: "Keep my plans", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "Clear plans and scheduled training", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Clear plans and scheduled training", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "clear_local_plan_data"))).toHaveLength(1);
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "delete_pain_entry" || call.command === "clear_pain_messages"))).toBe(false);
  await expect(page.getByRole("button", { name: "Clear plans and scheduled training", exact: true })).toBeDisabled();
});

test("plan import only overwrites after explicit confirmation", async ({ page }) => {
  await page.goto("/desk.html?syncConflict=1#settings/devices");
  await page.getByRole("button", { name: "Review plans", exact: true }).click();
  await expect(page.getByRole("dialog", {name:"Replace the plan on this device?"})).toBeVisible();
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_pull").map(call => call.args.force))).toEqual([false]);
  await page.getByRole("button", { name:"Keep local plan" }).click();
  await page.getByRole("button", { name:"Review plans", exact:true }).click();
  await page.getByRole("button", { name:"Replace local plan", exact:true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.getByRole("status").last()).toContainText("Imported 33 sessions");
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "plan_sync_pull").map(call => call.args.force))).toEqual([false,false,true]);
});

test("training boundaries keep fractional units and re-read saved values", async ({ page }) => {
  await page.goto("/desk.html#settings/thresholds");
  await page.getByRole("spinbutton", { name:"Recovery week load (%)" }).fill("65");
  await page.getByRole("button", { name:"Save boundaries" }).click();
  await expect(page.getByRole("status")).toContainText("Training boundaries saved");
  const settings = await page.evaluate(() => window.__deskCalls.find(call => call.command === "save_settings").args.settings);
  expect(settings.thresholds.downWeekFactor).toBe(.65);
  expect(settings.llmModel).toBe("test-model");
  await expect(page.getByRole("spinbutton", { name:"Recovery week load (%)" })).toHaveValue("65");
});

test("removing an assistant clears its key without deleting pain data", async ({ page }) => {
  await page.goto("/desk.html#settings/assistant");
  await page.getByRole("button", { name:"Remove assistant", exact:true }).click();
  await page.getByRole("dialog").getByRole("button", {name:"Remove assistant",exact:true}).click();
  await expect(page.getByRole("button", {name:"Add assistant",exact:true})).toBeVisible();
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "clear_llm_api_key"))).toBe(true);
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "delete_pain_entry" || call.command === "clear_pain_messages"))).toBe(false);
  await expect(page.getByRole("button", {name:"Add assistant",exact:true})).toBeDisabled();
});

for (const width of [320, 1440]) {
  test(`information dialogs preserve focus and scroll locks at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 844 });
    for (const [route, title, content] of [
      ["health", "Data for readiness", "Readiness measurements"],
      ["health", "Training boundaries", "Decreasing ceiling"],
      ["health", "Estimate basis", "Riegel model"],
      ["health/pace", "About pace insights", "Rep boundaries are inferred"],
      ["settings/watch", "Connection diagnostics", "Check COROS API"],
      ["settings/devices", "Sync diagnostics", "synthetic-account-channel"],
      ["help", "A recent workout is missing", "Check watch connection"],
    ]) {
      await page.goto(`/desk.html#${route}`);
      if (route === "settings/devices") await page.getByRole("button", { name: "More options", exact: true }).click();
      if (route === "health" && title !== "Data for readiness") {
        const detailsToggle = page.getByRole("button", { name: /^(Show|Hide) health details$/ });
        await expect(detailsToggle).toBeVisible();
        if (await detailsToggle.getAttribute("aria-expanded") === "false") await detailsToggle.click();
      }
      const trigger = page.getByRole("button", { name: title, exact: true });
      await expect(trigger).toHaveCSS("text-align", "center");
      await expect(trigger.locator(".button-content")).toHaveCSS("justify-content", "center");
      await expect.poll(() => trigger.evaluate(element => {
        const button = element.getBoundingClientRect();
        const content = element.querySelector(".button-content").getBoundingClientRect();
        return Math.max(Math.abs(content.x + content.width / 2 - button.x - button.width / 2), Math.abs(content.y + content.height / 2 - button.y - button.height / 2));
      })).toBeLessThan(1);
      await trigger.click();
      const dialog = page.getByRole("dialog", { name: title, exact: true });
      await expect(dialog).toBeVisible();
      await expect(dialog).toContainText(content);
      await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "hidden");
      expect(await dialog.evaluate(element => getComputedStyle(element, "::backdrop").backdropFilter)).toBe("blur(12px)");
      await expect(page.locator("details")).toHaveCount(0);
      expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
      await page.keyboard.press("Escape");
      await expect(dialog).toHaveCount(0);
      await expect(trigger).toBeFocused();
      await expect(page.locator("#workspace")).toHaveCSS("overflow-y", "auto");
    }
  });

  test(`Settings and Help are accessible at ${width}px`, async ({ page }) => {
    test.setTimeout(90000);
    await page.setViewportSize({width,height:900});
    for (const theme of ["light", "dark"]) for (const section of ["", "/watch", "/appearance", "/thresholds", "/training", "/calendar", "/assistant", "/maps", "/about"]) {
      await page.goto(`/desk.html?theme=${theme}#settings${section}`);
      await expect(page.locator(".desk-loading")).toHaveCount(0);
      await expect(page.getByRole("heading", {name:"Settings",exact:true})).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `${theme} ${section}`).toBe(true);
      expect((await new AxeBuilder({page}).withTags(["wcag2a","wcag2aa","wcag21aa"]).analyze()).violations, `${theme} ${section}`).toEqual([]);
      await page.screenshot({path:`test-results/glass-settings${section.replace("/", "-")}-${theme}-${width}.png`,fullPage:true});
    }
    await page.goto("/desk.html#help");
    await page.getByRole("searchbox", {name:"Search help"}).fill("missing");
    await expect(page.locator(".help-articles > .dialog-action")).toHaveCount(1);
    await page.getByText("A recent workout is missing", {exact:true}).click();
    await page.getByRole("link", {name:"Check watch connection"}).click();
    await expect(page.getByRole("heading", {name:"Watch connection",exact:true})).toBeVisible();
  });
}

const emitStage = (page, event) => page.evaluate(payload => window.__deskEmit('plan://stage', payload), event);

test("sync countdown survives an unavailable startup duration token", async ({ page }) => {
  await page.addInitScript(() => {
    const original = CSSStyleDeclaration.prototype.getPropertyValue;
    let intercepted = false;
    CSSStyleDeclaration.prototype.getPropertyValue = function (property) {
      if (property === "--notice-duration" && !intercepted) {
        intercepted = true;
        window.__noticeTokenIntercepted = true;
        return "";
      }
      return original.call(this, property);
    };
  });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/desk.html");
  expect(await page.evaluate(() => window.__noticeTokenIntercepted)).toBe(true);
  const toast = page.getByRole("status", { name: "Plan sync progress" });
  await emitStage(page, { op: "sync", target: "op", state: "working" });
  await emitStage(page, { op: "sync", target: "strava", state: "working" });
  await emitStage(page, { op: "sync", target: "strava", state: "done", detail: "Strava finished" });
  await emitStage(page, { op: "sync", target: "op", state: "done" });
  await expect(toast.locator(".stage-ring")).toBeVisible();
  await page.waitForTimeout(1000);
  await expect(toast).toContainText("Strava finished");
  await expect(toast).toBeHidden({ timeout: 10000 });
});

for (const width of [320, 1440]) for (const theme of ["light", "dark"]) {
  test(`strava stage reports pending and failed work without blocking dismissal ${width}px ${theme}`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto(`/desk.html?theme=${theme}`);
    const toast = page.getByRole("status", { name: "Plan sync progress" });
    await emitStage(page, { op: "sync", target: "op", state: "working" });
    for (const target of ["db", "calendar", "gist"]) await emitStage(page, { op: "sync", target, state: "done" });
    await expect(toast).not.toContainText("Strava");
    await emitStage(page, { op: "sync", target: "strava", state: "working" });
    await expect(toast.getByText("Updating Strava…", { exact: true })).toBeVisible();
    await expect(toast.locator(".stage-ring")).toHaveCount(0);
    const pending = "Strava: 1 copied, 0 already present, 1 pending, 0 need attention. Waiting for the watch activity to arrive in Strava. Next retry in 5 min.";
    await emitStage(page, { op: "sync", target: "strava", state: "done", detail: pending });
    await emitStage(page, { op: "sync", target: "op", state: "done" });
    await expect(toast.locator(".stage-line")).toHaveText(["Updated local DB", "Updated calendar", "Updated gist", pending]);
    await expect(toast.locator(".stage-working")).toHaveCount(0);
    await expect(toast.locator(".stage-done > .tabler-icon-check")).toHaveCount(4);
    for (const icon of await toast.locator(".stage-done > svg").all()) {
      await expect(icon).toHaveCSS("opacity", "1");
      expect(await icon.evaluate(element => getComputedStyle(element).color === getComputedStyle(element.parentElement).color)).toBe(true);
    }
    await expect(toast.locator(".stage-ring")).toHaveCount(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const box = await toast.boundingBox();
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(width);
    expect((await new AxeBuilder({ page }).include(".notification-viewport").analyze()).violations).toEqual([]);
    await page.screenshot({ path: test.info().outputPath(`strava-stage-${width}-${theme}.png`), fullPage: true });
    await emitStage(page, { op: "sync", target: "op", state: "working" });
    await emitStage(page, { op: "sync", target: "strava", state: "error", detail: "Strava Keychain is unavailable. Unlock your device and try again." });
    await emitStage(page, { op: "sync", target: "op", state: "error" });
    await expect(toast.locator(".stage-line")).toHaveCount(1);
    await expect(toast.locator(".stage-error")).toContainText("Updating Strava failed: Strava Keychain is unavailable.");
    await expect(toast.locator(".stage-error > .tabler-icon-x")).toBeVisible();
    await expect(toast.locator(".stage-ring")).toHaveCount(1);
    await page.mouse.move(0, 0);
    await expect(toast).toBeHidden({ timeout: 12000 });
  });
}

test("strava stage is absent on a later sync without configured Strava", async ({ page }) => {
  await page.goto("/desk.html");
  const toast = page.getByRole("status", { name: "Plan sync progress" });
  await emitStage(page, { op: "sync", target: "op", state: "working" });
  await emitStage(page, { op: "sync", target: "strava", state: "done" });
  await emitStage(page, { op: "sync", target: "op", state: "done" });
  await expect(toast.getByText("Updated Strava", { exact: true })).toBeVisible();
  await emitStage(page, { op: "sync", target: "op", state: "working" });
  await emitStage(page, { op: "sync", target: "db", state: "done" });
  await emitStage(page, { op: "sync", target: "op", state: "done" });
  await expect(toast.locator(".stage-line")).toHaveText(["Updated local DB"]);
});

test("stage toast opens one line per mirror in order and self-dismisses once all settle", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/desk.html");
  const toast = page.getByRole("status", { name: "Plan sync progress" });
  await expect(toast).toHaveCount(0);
  await emitStage(page, { op: "move", target: "db", state: "working", detail: null });
  await expect(toast.getByText("Updating local DB…")).toBeVisible();
  await emitStage(page, { op: "move", target: "calendar", state: "working", detail: null });
  await emitStage(page, { op: "move", target: "db", state: "done", detail: null });
  await emitStage(page, { op: "move", target: "gist", state: "working", detail: null });
  // Lines keep their first-seen (chronological) order and each settles on its own.
  await expect(toast.locator(".stage-line")).toHaveText([
    "Updated local DB",
    "Updating calendar…",
    "Updating gist…",
  ]);
  // Still open while anything works, even after the command itself returned:
  // the watch re-push runs in the background and its line stays until it lands.
  await emitStage(page, { op: "move", target: "calendar", state: "done", detail: null });
  await emitStage(page, { op: "move", target: "op", state: "done", detail: null });
  await expect(toast).toBeVisible();
  await emitStage(page, { op: "move", target: "gist", state: "done", detail: null });
  // Every line has a verdict and the op settled: the toast counts down and
  // removes itself without any click.
  await expect(toast.locator(".stage-ring")).toBeVisible();
  await expect(toast).toBeHidden({ timeout: 12000 });
});

test("a failed mirror stays visible as an error line until the toast dismisses", async ({ page }) => {
  await page.goto("/desk.html");
  const toast = page.getByRole("status", { name: "Plan sync progress" });
  await emitStage(page, { op: "sync", target: "coros", state: "working", detail: null });
  await emitStage(page, { op: "sync", target: "coros", state: "error", detail: "watch offline" });
  await emitStage(page, { op: "sync", target: "op", state: "error", detail: null });
  await expect(toast.getByText("Updating COROS calendar failed")).toBeVisible();
  await expect(toast.locator(".stage-error")).toHaveCount(1);
  await expect(toast.locator(".stage-error")).toHaveAttribute("title", "watch offline");
  await page.getByRole("button", { name: "Dismiss notification" }).click();
  await expect(toast).toHaveCount(0);
});

test("hovering the settled stage toast holds it open and pauses the countdown ring", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/desk.html");
  const toast = page.getByRole("status", { name: "Plan sync progress" });
  await emitStage(page, { op: "reset", target: "db", state: "done", detail: null });
  await emitStage(page, { op: "reset", target: "op", state: "done", detail: null });
  const ring = toast.locator(".stage-ring circle");
  await expect(ring).toHaveCSS("animation-play-state", "running");
  await toast.hover();
  await expect(ring).toHaveCSS("animation-play-state", "paused");
  await page.waitForTimeout(7000);
  await expect(toast).toBeVisible();
  await page.mouse.move(5, 5);
  await page.waitForTimeout(1000);
  await expect(toast).toBeVisible();
  await expect(toast).toBeHidden({ timeout: 8000 });
});

test("the plain notice carries the same countdown ring around its dismiss button", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/desk.html#settings/thresholds");
  await page.getByRole("button", { name: "Save boundaries" }).click();
  await expect(page.getByRole("status")).toContainText("Training boundaries saved");
  await expect(page.locator(".desk-notice .stage-ring")).toBeVisible();
});

for (const theme of ["light", "dark"]) {
  for (const width of [390, 1440]) {
    test(`notifications use translucent theme glass ${theme} ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(`/desk.html?theme=${theme}&palette=aurora#settings/thresholds`);
      await page.getByRole("button", { name: "Save boundaries" }).click();
      const notice = page.locator(".desk-notice");
      await expect(notice).toBeVisible();
      await notice.hover();
      await expect(notice).toHaveCSS("backdrop-filter", "blur(24px) saturate(1.45)");
      const inspect = () => notice.evaluate(element => {
        const style = getComputedStyle(element);
        const canvas = document.createElement("canvas");
        canvas.width = canvas.height = 1;
        const context = canvas.getContext("2d");
        context.fillStyle = style.backgroundColor;
        context.fillRect(0, 0, 1, 1);
        return { alpha: context.getImageData(0, 0, 1, 1).data[3] / 255, image: style.backgroundImage, ink: style.color, expectedInk: getComputedStyle(document.documentElement).getPropertyValue("--desk-ink").trim() };
      });
      const initial = await inspect();
      expect(initial.alpha).toBeCloseTo(.8, 2);
      expect(initial.image).toContain("linear-gradient");
      await expect(notice).toHaveCSS("color", theme === "dark" ? "rgb(243, 245, 247)" : "rgb(25, 27, 32)");
      await page.evaluate(() => { document.documentElement.dataset.deskPalette = "sakura"; });
      expect((await inspect()).image).not.toBe(initial.image);
      await page.screenshot({ path: testInfo.outputPath(`notification-glass-${theme}-${width}.png`) });
      await page.emulateMedia({ contrast: "more" });
      await expect(notice).toHaveCSS("backdrop-filter", "none");
      await expect(notice).toHaveCSS("background-image", "none");
      expect((await inspect()).alpha).toBe(1);
      await notice.getByRole("button", { name: "Dismiss notification" }).click();
      await expect(notice).toHaveCount(0);
    });
  }
}

for (const width of [320, 390, 768, 1440]) {
  test(`unified notifications do not overlap progress or navigation ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/desk.html#settings/thresholds");
    await emitStage(page, { op: "sync", target: "coros", state: "working" });
    await page.getByRole("button", { name: "Save boundaries" }).click();
    const progress = page.getByRole("status", { name: "Plan sync progress" });
    const notice = page.locator(".desk-notice").filter({ hasText: "Training boundaries saved." });
    await expect(progress).toBeVisible();
    await expect(notice).toBeVisible();
    const typography = locator => locator.evaluate(element => {
      const style = getComputedStyle(element);
      return { fontSize: style.fontSize, lineHeight: style.lineHeight, fontFamily: style.fontFamily, fontWeight: style.fontWeight };
    });
    expect(await typography(progress.locator(".stage-line"))).toEqual(await typography(notice.locator(".notice-content")));
    await emitStage(page, { op: "sync", target: "strava", state: "done", detail: "Strava: 1 copied, 0 already present, 1 pending, 0 need attention. Waiting for the watch activity to arrive." });
    const detail = progress.locator(".stage-line").last();
    await expect(detail).toContainText("Waiting for the watch activity");
    expect(await detail.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
    for (const notification of [notice, progress]) {
      const content = await notification.locator(".notice-content").boundingBox();
      const dismissButton = await notification.getByRole("button", { name: "Dismiss notification" }).boundingBox();
      expect(Math.abs(content.y + content.height / 2 - dismissButton.y - dismissButton.height / 2)).toBeLessThan(1);
    }
    const progressBox = await progress.boundingBox();
    const noticeBox = await notice.boundingBox();
    expect(progressBox.y + progressBox.height <= noticeBox.y || noticeBox.y + noticeBox.height <= progressBox.y).toBe(true);
    expect(progressBox.x).toBeGreaterThanOrEqual(0);
    expect(noticeBox.x + noticeBox.width).toBeLessThanOrEqual(width);
    const clipping = await notice.evaluate(element => {
      const viewport = element.closest('.notification-viewport');
      const viewportStyle = getComputedStyle(viewport);
      const noticeStyle = getComputedStyle(element);
      return {
        padding: parseFloat(viewportStyle.paddingTop),
        radius: parseFloat(viewportStyle.borderTopLeftRadius),
        noticeRadius: parseFloat(noticeStyle.borderTopLeftRadius),
      };
    });
    expect(clipping.padding).toBeGreaterThanOrEqual(6);
    expect(clipping.radius).toBeGreaterThanOrEqual(clipping.noticeRadius + clipping.padding);
    const dismiss = notice.getByRole('button', { name: 'Dismiss notification' });
    await dismiss.focus();
    await expect(dismiss).toBeFocused();
    await expect(notice).toHaveCSS('outline-style', 'solid');
    if (width <= 800) {
      const navigation = await page.locator(".desk-navigation").boundingBox();
      expect(Math.max(progressBox.y + progressBox.height, noticeBox.y + noticeBox.height)).toBeLessThan(navigation.y);
    }
    await page.keyboard.press('Enter');
    await expect(notice).toHaveCount(0);
  });
}

test("unified notifications queue distinct messages and deduplicate background failures", async ({ page }) => {
  await page.goto("/desk.html#settings/thresholds");
  await page.getByRole("button", { name: "Save boundaries" }).click();
  await expect(page.locator(".desk-notice")).toContainText("Training boundaries saved.");
  await page.evaluate(() => {
    window.__deskEmit("sync://completed", { ok: false });
    window.__deskEmit("sync://completed", { ok: false });
  });
  await expect(page.locator(".desk-notice")).toHaveCount(1);
  await expect(page.locator(".desk-notice")).toContainText("Training boundaries saved.");
  await page.getByRole("button", { name: "Dismiss notification" }).click();
  await expect(page.locator(".desk-notice")).toContainText("Background sync failed.");
  await page.getByRole("button", { name: "Dismiss notification" }).click();
  await expect(page.locator(".desk-notice")).toHaveCount(0);
});

test("unified notifications remain dismissible above a native modal without losing form state", async ({ page }) => {
  await page.goto("/desk.html#health/journal");
  await page.getByRole("button", { name: "Log pain", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Log pain" });
  await dialog.getByLabel("Notes", { exact: true }).fill("Keep this draft");
  await page.evaluate(() => window.__deskEmit("sync://completed", { ok: false }));
  const notice = dialog.locator(".notification-viewport .desk-notice");
  await expect(notice).toContainText("Background sync failed.");
  await notice.getByRole("button", { name: "Dismiss notification" }).click();
  await expect(notice).toHaveCount(0);
  await expect(dialog).toBeVisible();
  const notes = dialog.getByRole("textbox", { name: "Notes", exact: true });
  await expect(notes).toHaveValue("Keep this draft");
  await expect(notes).toBeFocused();
  await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe("hidden");
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  expect(await page.evaluate(() => document.body.style.overflow)).not.toBe("hidden");
});

test("design system appearance updates cached and native control themes together", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("runalytics.theme", "dark"));
  await page.goto("/desk.html#settings/appearance");
  await expect(page.getByRole("radio", { name: "Light", exact: true })).toBeChecked();
  await expect(page.locator("html")).toHaveCSS("color-scheme", "light");
  await page.getByRole("radio", { name: "Dark", exact: true }).check();
  await expect(page.locator("html")).toHaveAttribute("data-desk-theme", "dark");
  await expect(page.locator("html")).toHaveCSS("color-scheme", "dark");
  await expect.poll(() => page.evaluate(() => localStorage.getItem("runalytics.theme"))).toBe("dark");
  await page.getByRole("radio", { name: "Ice Ice Pacey", exact: true }).check();
  await expect.poll(() => page.evaluate(() => localStorage.getItem("runalytics.palette"))).toBe("glacier");
  await page.getByRole("radio", { name: "System", exact: true }).check();
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.locator("html")).toHaveCSS("color-scheme", "dark");
  await page.emulateMedia({ colorScheme: "light" });
  await expect(page.locator("html")).toHaveCSS("color-scheme", "light");
});

test("new palettes apply and persist across appearance modes", async ({ page }) => {
  await page.goto("/desk.html#settings/appearance");
  const root = page.locator("html");
  await expect(page.locator(".palette-options .palette-name")).toHaveText([
    "Blue Steel Sprint", "Burnt Toast Tempo", "Dashing Blueberry", "Golden Hour Hustle", "Graphite Gallop", "Ice Ice Pacey", "Moss Boss", "Pink It and Sprint", "Purple Reign", "Teal We Meet Again", "The Green Mile",
    "Chasing Confetti", "Fast and Curious", "Kaleidoscope Cadence", "Miles of Mischief", "Sunrise Shenanigans", "The Coastal Chaos",
  ]);
  await expect(page.getByRole("radio", { name: "Fast and Curious", exact: true }).locator("..").locator(".palette-swatch")).toHaveCSS("background-image", /conic-gradient/);
  for (const [name, id, light, dark] of [
    ["Graphite Gallop", "midnight", "#525866", "#c3cad6"],
    ["Teal We Meet Again", "tide", "#176b6a", "#91d9d2"],
    ["Kaleidoscope Cadence", "aurora", "#166b68", "#91d9d2"],
    ["Sunrise Shenanigans", "summit", "#a84a2e", "#f3b49f"],
    ["The Coastal Chaos", "coastline", "#225f9a", "#a9caff"],
    ["Miles of Mischief", "wildflower", "#963e6c", "#efb2d0"],
    ["Chasing Confetti", "citrus", "#ad2835", "#ffb2b7"],
    ["Blue Steel Sprint", "cobalt", "#285ea1", "#a9caff"],
    ["Moss Boss", "moss", "#526c35", "#c0d9a4"],
    ["Dashing Blueberry", "garnet", "#285b98", "#a9caff"],
    ["Golden Hour Hustle", "ochre", "#866000", "#e8cc83"],
  ]) {
    await page.getByRole("radio", { name: "Light", exact: true }).check();
    await page.getByRole("radio", { name, exact: true }).check();
    await expect(root).toHaveAttribute("data-desk-palette", id);
    await expect.poll(() => page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--desk-action").trim())).toBe(light);
    await expect.poll(() => page.evaluate(() => localStorage.getItem("runalytics.palette"))).toBe(id);
    await page.getByRole("radio", { name: "Dark", exact: true }).check();
    await expect.poll(() => page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--desk-action").trim())).toBe(dark);
  }
});

test("multicolor palettes use secondary colors in navigation and sessions", async ({ page }) => {
  await page.goto("/desk.html#settings/appearance");
  await page.getByRole("radio", { name: "Light", exact: true }).check();
  for (const name of ["Fast and Curious", "Kaleidoscope Cadence", "Sunrise Shenanigans", "The Coastal Chaos", "Miles of Mischief", "Chasing Confetti"]) {
    await page.getByRole("radio", { name, exact: true }).check();
    await page.getByRole("link", { name: "Training", exact: true }).first().click();
    await expect(page.locator(".desk-navigation a[aria-current]").first()).toBeAttached();
    const colors = await page.evaluate(() => {
      const root = getComputedStyle(document.documentElement);
      return { action: root.getPropertyValue("--desk-action").trim(), secondary: root.getPropertyValue("--desk-blue").trim(), navigation: getComputedStyle(document.querySelector(".desk-navigation a[aria-current]")).color };
    });
    expect(colors.secondary).not.toBe(colors.action);
    expect(colors.navigation).toBe(await page.evaluate(color => {
      const element = document.createElement("span");
      element.style.color = color;
      document.body.appendChild(element);
      const result = getComputedStyle(element).color;
      element.remove();
      return result;
    }, colors.secondary));
    await page.goto("/desk.html#settings/appearance");
  }
});

test("unified notifications preserve an earlier Undo behind newer action feedback", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/desk.html");
  await page.getByRole("button", { name: "Next week", exact: true }).click();
  await page.locator(".week-board .session-tile.movable").first().click();
  await page.getByRole("button", { name: "Skip session", exact: true }).click();
  await expect(page.getByRole("button", { name: "Undo", exact: true })).toBeVisible();
  await page.locator(".week-board .session-tile.movable").first().click();
  await page.getByRole("button", { name: "Copy prescription", exact: true }).click();
  await expect(page.locator(".desk-notice")).toContainText("Session copied.");
  await page.getByRole("button", { name: "Dismiss notification" }).click();
  await expect(page.locator(".desk-notice")).toContainText("Session skipped");
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call => call.command === "set_workout_skipped").map(call => call.args.skipped))).toEqual([true, false]);
});

test("design system primitives share type, control geometry and bounded glass", async ({ page }) => {
  await page.goto("/desk.html#settings/thresholds");
  await expect(page.getByRole("button", { name: "Save boundaries" })).toBeVisible();
  const controls = await page.locator('.settings-editor :is(input[type="number"], .desk-button)').evaluateAll(elements => elements.map(element => {
    const style = getComputedStyle(element);
    return { font: style.fontFamily, radius: style.borderRadius, height: element.getBoundingClientRect().height, blur: style.backdropFilter, button: element.matches(".desk-button") };
  }));
  expect(controls.length).toBeGreaterThan(10);
  for (const control of controls) {
    expect(control.font).toContain("Outfit");
    expect(control.radius).toBe(control.button ? "999px" : "14px");
    expect(control.height).toBe(44);
    expect(control.blur).toBe("none");
  }
  const drift = await page.evaluate(() => {
    const violations = [];
    const visit = rules => {
      for (const rule of rules) {
        if (rule.cssRules) visit(rule.cssRules);
        if (!rule.style) continue;
        for (const property of ["font-size", "font-weight", "z-index"]) {
          const value = rule.style.getPropertyValue(property).trim();
          if (/^\d/.test(value)) violations.push(`${rule.selectorText}: ${property}: ${value}`);
        }
      }
    };
    for (const sheet of document.styleSheets) {
      const identifier = sheet.ownerNode?.getAttribute("data-vite-dev-id") ?? sheet.href ?? "";
      if (/\/src\/desk\/(desk|interaction)\.css/.test(identifier)) visit(sheet.cssRules);
    }
    return violations;
  });
  expect(drift).toEqual([]);
});

for (const theme of ["light", "dark"]) {
  test(`glass system palette states remain legible and consistent in ${theme}`, async ({ page }) => {
    test.setTimeout(90000);
    await page.setViewportSize({ width: 390, height: 900 });
    for (const palette of ["reference", "amethyst", "glacier", "evergreen", "sakura", "ember"]) {
      await page.goto(`/desk.html?theme=${theme}&palette=${palette}#settings/thresholds`);
      const primary = page.getByRole("button", { name: "Save boundaries" });
      const secondary = page.getByRole("button", { name: "Restore defaults", exact: true });
      await expect(primary).toBeVisible();
      await primary.scrollIntoViewIfNeeded();
      const rest = await primary.evaluate(element => getComputedStyle(element).backgroundColor);
      await primary.hover();
      const hover = await primary.evaluate(element => getComputedStyle(element).backgroundColor);
      expect(hover, palette).not.toBe(rest);
      expect((await new AxeBuilder({ page }).include(".settings-editor").withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations, `${palette} hover`).toEqual([]);
      await page.mouse.down();
      expect(await primary.evaluate(element => getComputedStyle(element).backgroundColor), palette).not.toBe(hover);
      expect((await new AxeBuilder({ page }).include(".settings-editor").withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations, `${palette} pressed`).toEqual([]);
      await page.mouse.move(0, 0);
      await page.mouse.up();
      await secondary.click();
      const dialog = page.getByRole("dialog", { name: "Restore training defaults?" });
      await expect(dialog).toBeVisible();
      await expect(dialog).toHaveCSS("backdrop-filter", /blur/);
      expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations, `${palette} dialog`).toEqual([]);
      await page.screenshot({ path: `test-results/glass-system-${theme}-${palette}.png` });
      await page.keyboard.press("Escape");
      await expect(secondary).toBeFocused();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), palette).toBe(true);
    }
  });
}