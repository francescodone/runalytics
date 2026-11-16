import { test, expect } from '@playwright/test';
import { installDeskFixture } from './desk.fixture.mjs';

test.beforeEach(async ({ page }) => {
  await installDeskFixture(page);
  await page.clock.setFixedTime(new Date('2027-02-15T12:00:00'));
});

for (const scenario of [
  { name: '5K race target', distance: '5K', available: true, width: 320 },
  { name: 'marathon race target', distance: '42K', available: true, width: 1440 },
  { name: 'missing starting volume', distance: '21K', baseline: null, available: false, width: 320 },
  { name: 'maintenance plan', distance: '21K', kind: 'maintenance', available: false, width: 1440 },
  { name: 'no scheduled training', distance: '21K', noWorkouts: true, available: false, width: 1440 },
]) {
  test(`race projections across distances: ${scenario.name}`, async ({ page }) => {
    await page.setViewportSize({ width: scenario.width, height: 844 });
    await page.addInitScript(options => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        const result = await invoke(command, args);
        if (command !== 'current_plan' || !result) return result;
        return [{ ...result[0], kind: options.kind ?? 'race', status: 'active', targetDate: '2027-03-15', distance: options.distance, baselineWeeklyKm: options.baseline === null ? null : 40, thresholdSecKm: 300, qualityOffsetSecKm: 0 }, options.noWorkouts ? [] : [
          { ...result[1][0], date: '2027-02-15', kind: 'easy', distanceM: 20000 },
          { ...result[1][0], date: '2027-02-17', kind: 'long_run', distanceM: 24000 },
        ]];
      };
    }, scenario);
    await page.goto('/desk.html#pace');
    const estimates = page.getByRole('region', { name: 'Training race estimates', exact: true });
    const cells = estimates.locator('tbody tr td:last-child');
    await expect(cells).toHaveCount(4);
    if (scenario.available) {
      const expected = [5, 10, 21.0975, 42.195].map(km => Math.round(3600 * Math.pow(km / (3600 / 294), 1.06)));
      await expect.poll(() => cells.evaluateAll(nodes => nodes.map(node => Number(node.dataset.projection)))).toEqual(expected);
    } else {
      await expect(cells).toHaveText(Array(4).fill('Race day projection—'));
      await expect(estimates.locator('td[data-projection]')).toHaveCount(0);
    }
  });
}
