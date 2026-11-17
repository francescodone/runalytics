import { test, expect } from "@playwright/test";
import { installDeskFixture } from "./desk.fixture.mjs";

test.beforeEach(async ({ page }) => {
  await installDeskFixture(page);
  await page.emulateMedia({ reducedMotion: "no-preference" });
});

async function openWorkspace(page, route) {
  await page.goto(`/desk.html#${route}`);
  await expect(page.locator(".workspace-heading h1")).toBeVisible();
  await expect(page.locator(".brand-splash")).toHaveCount(0);
}

async function partGeometry(icon) {
  return icon.evaluate(element => ({
    paths: [...element.querySelectorAll("path")].map(path => path.getAttribute("d")),
    parts: [...element.querySelectorAll("[data-icon-part]")].map(part => {
      const style = getComputedStyle(part);
      return [part.dataset.iconPart, style.transform, style.opacity, style.strokeDashoffset];
    }),
    width: element.offsetWidth,
    height: element.offsetHeight,
  }));
}

async function partFrames(icon, count = 60) {
  return icon.evaluate(async (element, count) => {
    const frames = [];
    for (let index = 0; index < count; index++) {
      await new Promise(requestAnimationFrame);
      frames.push(JSON.stringify([...element.querySelectorAll("[data-icon-part]")].map(part => {
        const style = getComputedStyle(part);
        return [style.transform, style.opacity, style.strokeDashoffset];
      })));
    }
    return frames;
  }, count);
}

async function runnerSteps(icon) {
  return icon.evaluate(element => {
    const animations = element.getAnimations({ subtree: true });
    const duration = Number(animations[0].effect.getTiming().duration);
    const part = name => element.querySelector(`[data-icon-part="run-${name}"]`);
    const endpoint = (path, end) => {
      const point = path.getPointAtLength(end ? path.getTotalLength() : 0);
      return new DOMPoint(point.x, point.y).matrixTransform(path.getScreenCTM());
    };
    const angle = node => {
      const matrix = new DOMMatrix(getComputedStyle(node).transform);
      return Math.atan2(matrix.b, matrix.a) * 180 / Math.PI;
    };
    for (const animation of animations) animation.pause();
    const poses = [0, .25, .5, .75].map(phase => {
      for (const animation of animations) animation.currentTime = duration * phase;
      const legs = ["front", "back"].map(side => {
        const thigh = part(`leg-${side}`).querySelector(":scope > path");
        const shin = part(`knee-${side}`).querySelector("path");
        const upper = endpoint(thigh, true), lower = endpoint(shin, false), foot = endpoint(shin, true);
        return { foot: { x: foot.x, y: foot.y }, jointGap: Math.hypot(upper.x - lower.x, upper.y - lower.y), knee: angle(part(`knee-${side}`)) };
      });
      return { legs, thigh: angle(part("leg-front")), arm: angle(part("arm-front")) };
    });
    for (const animation of animations) animation.play();
    return poses;
  });
}

async function svgFrames(icon, count = 60) {
  return icon.evaluate(async (element, count) => {
    const frames = [];
    for (let index = 0; index < count; index++) {
      await new Promise(requestAnimationFrame);
      frames.push(JSON.stringify([...element.querySelectorAll("svg path, svg g, svg circle, svg rect")].map(node =>
        ["d", "transform", "opacity", "stroke-dashoffset", "stroke-dasharray", "cx", "cy", "r"].map(attribute => node.getAttribute(attribute))
      )));
    }
    return frames;
  }, count);
}

function expectOneShot(frames) {
  expect(new Set(frames).size).toBeGreaterThan(2);
  expect(new Set(frames.slice(-6)).size).toBe(1);
}

async function expectStatic(icon) {
  await expect(icon).toHaveAttribute("data-icon-playback", "static");
  await expect(icon.locator("[data-useanimations], [data-custom-animation]")).toHaveCount(0);
  await expect(icon.locator(":scope > svg")).toHaveCount(1);
}

async function inputIcon(input) {
  await expect(input).toHaveJSProperty("tagName", "INPUT");
  const icon = input.locator("..").locator(".animated-icon");
  await expect(icon).toHaveAttribute("aria-hidden", "true");
  await expect(icon.locator('input, button, a, [tabindex], [role="checkbox"], [role="radio"]')).toHaveCount(0);
  return icon;
}

test("animated runner takes connected alternating steps on hover and keyboard focus without activating", async ({ page, browserName }) => {
  await page.setViewportSize({ width: 1100, height: 900 });
  await openWorkspace(page, "help");
  const link = page.locator('.desk-navigation a[href="#journal"]');
  const icon = link.locator('[data-icon="run"]');
  await expect(icon).toHaveAttribute("data-icon-playback", "ready");
  const resting = await partGeometry(icon);
  const calls = await page.evaluate(() => window.__deskCalls);
  const host = await link.elementHandle();
  for (const gesture of ["hover", "keyboard"]) {
    await page.mouse.move(0, 0);
    const playback = partFrames(icon);
    if (gesture === "hover") {
      await link.hover();
      const poses = await runnerSteps(icon);
      for (const pose of poses) for (const leg of pose.legs) expect(leg.jointGap).toBeLessThan(.05);
      expect(poses[0].legs[0].foot.x).toBeGreaterThan(poses[0].legs[1].foot.x + 3);
      expect(poses[2].legs[1].foot.x).toBeGreaterThan(poses[2].legs[0].foot.x + 3);
      expect(poses[1].legs[0].foot.y).toBeGreaterThan(poses[1].legs[1].foot.y + 2);
      expect(poses[3].legs[1].foot.y).toBeGreaterThan(poses[3].legs[0].foot.y + 2);
      expect(poses[2].thigh - poses[0].thigh).toBeGreaterThan(50);
      expect(poses[2].arm - poses[0].arm).toBeLessThan(-50);
      expect(poses[3].legs[0].knee - poses[1].legs[0].knee).toBeGreaterThan(40);
    } else {
      await page.locator('.desk-navigation a[href="#training"]').focus();
      // Safari uses Option-Tab to include links in native keyboard navigation.
      await page.keyboard.press(browserName === "webkit" ? "Alt+Tab" : "Tab");
      await expect(link).toBeFocused();
    }
    expectOneShot(await playback);
    expect(await partGeometry(icon)).toEqual(resting);
    await expect(page).toHaveURL(/#help$/);
    await expect(link).toHaveAccessibleName("Activities");
    expect(await link.evaluate((element, host) => element === host, host)).toBe(true);
    expect(await page.evaluate(() => window.__deskCalls)).toEqual(calls);
  }
  await host.dispose();
});

test("animated close stays an unboxed cross, settles quickly and preserves dismissal and focus", async ({ page }) => {
  await openWorkspace(page, "help");
  const opener = page.getByRole("button", { name: "A recent workout is missing", exact: true });
  await opener.click();
  const dialog = page.getByRole("dialog", { name: "A recent workout is missing", exact: true });
  const close = dialog.getByRole("button", { name: "Close A recent workout is missing", exact: true });
  const icon = close.locator('[data-icon="close"]');
  await expect(icon).toHaveAttribute("data-icon-playback", "ready");
  await close.hover();
  await page.mouse.move(0, 0);
  await expect.poll(() => icon.evaluate(element => element.getAnimations({ subtree: true }).length)).toBe(0);
  const resting = await partGeometry(icon);
  expect(resting.paths.length).toBeGreaterThan(0);
  expect(resting.paths.every(path => !/z/i.test(path))).toBe(true);
  await expect(icon.locator("rect, circle, polygon")).toHaveCount(0);
  const size = await close.evaluate(element => [element.offsetWidth, element.offsetHeight]);
  await close.evaluate(element => { window.__closeActivations = 0; element.addEventListener("click", () => window.__closeActivations++); });
  const playback = partFrames(icon, 30);
  await close.hover();
  expectOneShot(await playback);
  expect(await partGeometry(icon)).toEqual(resting);
  expect(await close.evaluate(element => [element.offsetWidth, element.offsetHeight])).toEqual(size);
  expect(await page.evaluate(() => window.__closeActivations)).toBe(0);
  await expect(dialog).toBeVisible();
  await close.press("Enter");
  await expect(dialog).toHaveCount(0);
  await expect(opener).toBeFocused();
  expect(await page.evaluate(() => window.__closeActivations)).toBe(1);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await opener.click();
  await expectStatic(icon);
  expect((await partGeometry(icon)).paths).toEqual(resting.paths);
  await close.click();
  await expect(dialog).toHaveCount(0);
  await expect(opener).toBeFocused();
});

test("animated palette language and provider gestures retain their shapes labels and connection permissions", async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 900 });
  await openWorkspace(page, "settings");
  for (const [section, name, label] of [["appearance", "palette", "Appearance"], ["language", "language", "Language"], ["strava", "strava", "Strava"]]) {
    const link = page.locator(`.settings-task-list a[href="#settings/${section}"]`);
    const icon = link.locator(`[data-icon="${name}"]`);
    await link.scrollIntoViewIfNeeded();
    await expect(icon).toHaveAttribute("data-icon-playback", "ready");
    const resting = await partGeometry(icon);
    const calls = await page.evaluate(() => window.__deskCalls);
    const playback = partFrames(icon);
    await link.hover();
    expectOneShot(await playback);
    expect(await partGeometry(icon)).toEqual(resting);
    await expect(link).toHaveAccessibleName(new RegExp(`^${label}`));
    await expect(page).toHaveURL(/#settings$/);
    expect(await page.evaluate(() => window.__deskCalls)).toEqual(calls);
    await expect(icon.locator('[tabindex], button, a')).toHaveCount(0);
    await page.mouse.move(0, 0);
  }
});

test("animated QR scan gestures never alter or regenerate the encrypted transfer image", async ({ page }) => {
  await openWorkspace(page, "settings/transfer");
  const create = page.getByRole("button", { name: "Create transfer QR", exact: true });
  const icon = create.locator('[data-icon="qr"]');
  const resting = await partGeometry(icon);
  const playback = partFrames(icon);
  await create.hover();
  expectOneShot(await playback);
  expect(await partGeometry(icon)).toEqual(resting);
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "export_connection_settings"))).toEqual([]);
  await create.click();
  const dialog = page.getByRole("dialog", { name: "Create connection transfer", exact: true });
  await dialog.getByLabel("Transfer passphrase", { exact: true }).fill("synthetic icon transfer passphrase");
  const generate = dialog.getByRole("button", { name: "Generate encrypted QR", exact: true });
  await generate.hover({ trial: true });
  const scan = partFrames(generate.locator('[data-icon="qr"]'));
  await generate.hover();
  expectOneShot(await scan);
  await expect(generate).toHaveAccessibleName("Generate encrypted QR");
  await generate.press("Enter");
  const image = dialog.getByRole("img", { name: "Encrypted connection transfer QR code", exact: true });
  await expect(image).toBeVisible();
  const source = await image.getAttribute("src");
  const decode = () => image.evaluate(async image => {
    const { default: jsQR } = await import("/node_modules/.vite/deps/jsqr.js");
    await image.decode();
    const canvas = document.createElement("canvas");
    canvas.width = image.naturalWidth;
    canvas.height = image.naturalHeight;
    const context = canvas.getContext("2d");
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height);
    return jsQR(pixels.data, pixels.width, pixels.height)?.data;
  });
  const payload = await decode();
  expect(payload).toMatch(/^runalytics:connections:1:/);
  expect(payload).not.toContain("synthetic-github-token");
  expect(payload).not.toContain("synthetic-calendar-secret");
  const download = dialog.getByRole("link", { name: "Download QR image", exact: true });
  await page.mouse.move(0, 0);
  await download.scrollIntoViewIfNeeded();
  await expect(download.locator('[data-icon="download"]')).toHaveAttribute("data-icon-playback", "ready");
  await download.hover({ trial: true });
  const downloadPlayback = svgFrames(download.locator('[data-icon="download"]'));
  await download.hover();
  expectOneShot(await downloadPlayback);
  await expect(download).toHaveAttribute("href", source);
  await expect(image).toHaveAttribute("src", source);
  expect(await decode()).toBe(payload);
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "export_connection_settings").length)).toBe(1);
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(create).toBeFocused();
});

test("animated native radios follow keyboard selection and unselection through reduced motion and forced colors", async ({ page }) => {
  await openWorkspace(page, "settings/appearance");
  const light = page.getByRole("radio", { name: "Light", exact: true });
  const dark = page.getByRole("radio", { name: "Dark", exact: true });
  const lightIcon = await inputIcon(light);
  const darkIcon = await inputIcon(dark);
  const lightHost = await light.elementHandle();
  let writes = 0;
  for (const preference of ["normal", "reduced", "forced"]) {
    await page.emulateMedia({ reducedMotion: preference === "reduced" ? "reduce" : "no-preference", forcedColors: preference === "forced" ? "active" : "none" });
    await light.focus();
    const playback = svgFrames(darkIcon);
    await page.keyboard.press("ArrowRight");
    await expect(dark).toBeChecked();
    await expect(light).not.toBeChecked();
    await expect(darkIcon).toHaveAttribute("data-icon-state", "true");
    await expect(lightIcon).toHaveAttribute("data-icon-state", "false");
    await expect(page.locator("html")).toHaveAttribute("data-desk-theme", "dark");
    await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call => call.command === "save_settings").length)).toBe(++writes);
    const frames = await playback;
    if (preference === "normal") expect(new Set(frames).size).toBeGreaterThan(2);
    else {
      await expectStatic(lightIcon);
      await expectStatic(darkIcon);
      expect(await svgFrames(lightIcon, 1)).not.toEqual(await svgFrames(darkIcon, 1));
    }
    await light.focus();
    await page.keyboard.press("Space");
    await expect(light).toBeChecked();
    await expect(dark).not.toBeChecked();
    await expect(lightIcon).toHaveAttribute("data-icon-state", "true");
    await expect(darkIcon).toHaveAttribute("data-icon-state", "false");
    await expect.poll(() => page.evaluate(() => window.__deskCalls.filter(call => call.command === "save_settings").length)).toBe(++writes);
    expect(await light.evaluate((element, host) => element === host, lightHost)).toBe(true);
  }
  await lightHost.dispose();
});

test("animated checkbox filtering and calendar toggles retain native keyboard and single-submit behavior", async ({ page }) => {
  await openWorkspace(page, "milestones");
  const earned = page.getByRole("checkbox", { name: "Earned only", exact: true });
  const checkboxIcon = await inputIcon(earned);
  await earned.scrollIntoViewIfNeeded();
  const total = await page.locator(".milestone-list article").count();
  await earned.focus();
  const filtering = svgFrames(checkboxIcon);
  await page.keyboard.press("Space");
  await expect(earned).toBeChecked();
  await expect(checkboxIcon).toHaveAttribute("data-icon-state", "true");
  expect(new Set(await filtering).size).toBeGreaterThan(2);
  expect(await page.locator(".milestone-list article").count()).toBeLessThan(total);
  await expect(page.locator(".milestone-list article:not(.earned)")).toHaveCount(0);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expectStatic(checkboxIcon);
  const checkedPoster = await svgFrames(checkboxIcon, 1);
  await page.keyboard.press("Space");
  await expect(earned).not.toBeChecked();
  await expect(checkboxIcon).toHaveAttribute("data-icon-state", "false");
  expect(await svgFrames(checkboxIcon, 1)).not.toEqual(checkedPoster);
  await expect(page.locator(".milestone-list article")).toHaveCount(total);

  await page.goto("/desk.html#settings/calendar");
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.getByRole("button", { name: "Connect a calendar", exact: true }).click();
  const toggle = page.getByRole("checkbox", { name: "Publish calendar remotely", exact: true });
  const toggleIcon = await inputIcon(toggle);
  const expectUnpaintedInput = async () => {
    await expect(toggle).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
    await expect(toggle).toHaveCSS("background-image", "none");
    expect(await toggle.evaluate(input => getComputedStyle(input, "::before").content)).toBe("none");
  };
  await expect(toggle).toBeChecked();
  await toggle.focus();
  await toggle.evaluate(input => input.scrollIntoView({ block: "center", behavior: "instant" }));
  await expect(toggleIcon).toHaveAttribute("data-icon-playback", "ready");
  await expectUnpaintedInput();
  const toggling = svgFrames(toggleIcon);
  await page.keyboard.press("Space");
  await expect(toggle).not.toBeChecked();
  await expect(toggleIcon).toHaveAttribute("data-icon-state", "false");
  await expectUnpaintedInput();
  expect(new Set(await toggling).size).toBeGreaterThan(2);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expectStatic(toggleIcon);
  await page.emulateMedia({ reducedMotion: "no-preference", forcedColors: "active" });
  await expectStatic(toggleIcon);
  const disabledPoster = await svgFrames(toggleIcon, 1);
  await page.keyboard.press("Space");
  await expect(toggle).toBeChecked();
  await expect(toggleIcon).toHaveAttribute("data-icon-state", "true");
  expect(await svgFrames(toggleIcon, 1)).not.toEqual(disabledPoster);
  expect(await page.evaluate(() => window.__deskCalls.filter(call => call.command === "save_calendar_publishing"))).toEqual([]);
  await page.getByLabel("Username", { exact: true }).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("status").last()).toContainText("Calendar settings saved");
  const writes = await page.evaluate(() => window.__deskCalls.filter(call => call.command === "save_calendar_publishing"));
  expect(writes).toHaveLength(1);
  expect(writes[0].args).toMatchObject({ config: { enabled: true }, password: null });
  expect(await page.evaluate(() => window.__deskCalls.some(call => call.command === "publish_calendar"))).toBe(false);
});

test("animated reward first visibility plays actual limbs once and unrelated filtering does not restart playback", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 600 });
  await openWorkspace(page, "milestones");
  const reward = page.locator(".milestone-list article").filter({ has: page.getByRole("heading", { name: "Warm-up crew", exact: true }) });
  const icon = reward.locator('[data-icon="run"]');
  await expectStatic(icon);
  const resting = await partGeometry(icon);
  await reward.scrollIntoViewIfNeeded();
  await expect(icon).toHaveAttribute("data-icon-playback", "ready");
  await expect.poll(() => icon.evaluate(element => element.querySelector('[data-icon-part="run-leg-front"]').getAnimations().length)).toBe(1);
  await icon.evaluate(element => { window.__rewardPart = element.querySelector('[data-icon-part="run-leg-front"]'); window.__rewardAnimation = window.__rewardPart.getAnimations()[0]; });
  const playback = partFrames(icon);
  await page.getByRole("checkbox", { name: "Earned only", exact: true }).evaluate(input => input.click());
  expect(await icon.evaluate(element => element.querySelector('[data-icon-part="run-leg-front"]') === window.__rewardPart && window.__rewardPart.getAnimations()[0] === window.__rewardAnimation)).toBe(true);
  expectOneShot(await playback);
  expect(await partGeometry(icon)).toEqual(resting);
  const host = await icon.elementHandle();
  await page.getByRole("checkbox", { name: "Earned only", exact: true }).evaluate(input => input.click());
  expect(await icon.evaluate((element, host) => element === host, host)).toBe(true);
  expect(new Set(await partFrames(icon, 12)).size).toBe(1);
  expect(await icon.evaluate(element => [...element.querySelectorAll('[data-icon-part]')].flatMap(part => part.getAnimations()).length)).toBe(0);
  await host.dispose();
});

test("animated reward interruption cancels hidden inert and clipped playback without stale first-visible replay", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 600 });
  await page.addInitScript(() => {
    window.__iconDocumentHidden = false;
    Object.defineProperty(document, "hidden", { configurable: true, get: () => window.__iconDocumentHidden });
  });
  for (const interruption of ["hidden", "inert", "offscreen"]) {
    await page.goto(`/desk.html?iconInterruption=${interruption}#milestones`);
    await expect(page.locator(".workspace-heading h1")).toHaveText("Milestones");
    await expect(page.locator(".brand-splash")).toHaveCount(0);
    const reward = page.locator(".milestone-list article").filter({ has: page.getByRole("heading", { name: "Warm-up crew", exact: true }) });
    const icon = reward.locator('[data-icon="run"]');
    await reward.scrollIntoViewIfNeeded();
    await expect(icon).toHaveAttribute("data-icon-playback", "ready");
    await expect.poll(() => icon.evaluate(element => element.querySelector('[data-icon-part="run-leg-front"]').getAnimations().length)).toBe(1);
    expect(new Set(await partFrames(icon, 5)).size).toBeGreaterThan(1);
    const part = await icon.locator('[data-icon-part="run-leg-front"]').elementHandle();
    if (interruption === "hidden") await page.evaluate(() => { window.__iconDocumentHidden = true; document.dispatchEvent(new Event("visibilitychange")); });
    if (interruption === "inert") await reward.evaluate(element => { element.inert = true; });
    if (interruption === "offscreen") await page.locator("#workspace").evaluate(element => element.scrollTo({ top: 0, behavior: "instant" }));
    await expectStatic(icon);
    expect(await part.evaluate(element => element.getAnimations().length)).toBe(0);
    if (interruption === "hidden") await page.evaluate(() => { window.__iconDocumentHidden = false; document.dispatchEvent(new Event("visibilitychange")); });
    if (interruption === "inert") await reward.evaluate(element => { element.inert = false; });
    if (interruption === "offscreen") await reward.scrollIntoViewIfNeeded();
    await expect(icon).toHaveAttribute("data-icon-playback", "ready");
    expect(new Set(await partFrames(icon, 16)).size).toBe(1);
    expect(await icon.evaluate(element => [...element.querySelectorAll('[data-icon-part]')].flatMap(part => part.getAnimations()).length)).toBe(0);
    await part.dispose();
  }
});

test("animated map portals share eligibility preserve geographic anchors attribution and zoom then tear down cleanly", async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 1000 });
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
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
      const iconOriginalPaint = maplibregl.Map.prototype.setPaintProperty;
      maplibregl.Map.prototype.setPaintProperty = function(...args) { window.__iconMap = this; return iconOriginalPaint.apply(this, args); };
      const iconOriginalZoom = maplibregl.Map.prototype.zoomIn;
      maplibregl.Map.prototype.zoomIn = function(...args) { window.__iconMapZooms = (window.__iconMapZooms || 0) + 1; return iconOriginalZoom.apply(this, args); };
      const iconOriginalRemove = maplibregl.Map.prototype.remove;
      maplibregl.Map.prototype.remove = function(...args) { window.__iconMapRemovals = (window.__iconMapRemovals || 0) + 1; return iconOriginalRemove.apply(this, args); };
    ` });
  });
  await openWorkspace(page, "journal/1");
  const start = page.getByRole("img", { name: "Route start", exact: true });
  const finish = page.getByRole("img", { name: "Route finish", exact: true });
  await expect(start).toBeVisible();
  await expect(finish).toBeVisible();
  for (const marker of [start, finish]) await expect(marker).toHaveClass(/maplibregl-marker-anchor-center/);
  const attribution = page.locator(".maplibregl-ctrl-attrib summary");
  const info = attribution.locator('[data-icon="info"]');
  const zoom = page.getByRole("button", { name: "Zoom in", exact: true });
  for (const icon of [start.locator(".animated-icon"), finish.locator(".animated-icon"), info, zoom.locator(".animated-icon")]) await expect(icon).toHaveAttribute("data-icon-playback", "ready");
  const camera = await page.evaluate(() => ({ center: window.__iconMap.getCenter().toArray(), zoom: window.__iconMap.getZoom() }));
  const preview = svgFrames(info);
  await attribution.hover();
  expectOneShot(await preview);
  expect(await page.evaluate(() => window.__iconMap.getZoom())).toBe(camera.zoom);
  await attribution.click();
  const credit = page.getByRole("link", { name: "OpenStreetMap contributors", exact: true });
  await expect(credit).toBeVisible();
  await expect(credit).toHaveAttribute("href", "https://www.openstreetmap.org/copyright");
  await zoom.focus();
  await zoom.press("Enter");
  expect(await page.evaluate(() => window.__iconMapZooms)).toBe(1);
  expect(await page.evaluate(() => window.__iconMap.getZoom())).toBeGreaterThan(camera.zoom);
  await page.getByRole("button", { name: "Fit route", exact: true }).click();
  expect(await page.evaluate(() => window.__iconMap.getCenter().toArray())).toEqual(camera.center);
  const positions = await Promise.all([start, finish].map(marker => marker.getAttribute("style")));
  await page.emulateMedia({ reducedMotion: "reduce" });
  for (const icon of [start.locator(".animated-icon"), finish.locator(".animated-icon"), info, zoom.locator(".animated-icon")]) await expectStatic(icon);
  expect(await Promise.all([start, finish].map(marker => marker.getAttribute("style")))).toEqual(positions);
  await expect(credit).toBeVisible();
  await page.getByRole("button", { name: "Help", exact: true }).click();
  await expect(page.locator(".maplibregl-canvas")).toHaveCount(0);
  expect(await page.evaluate(() => window.__iconMapRemovals)).toBe(1);
  expect(errors).toEqual([]);
});
