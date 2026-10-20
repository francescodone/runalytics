// The one-flow: Connect device -> Sync data -> Create plan -> Train.
// The flow STATE (provider status, last sync, current plan) drives the
// dashboard wizard and the empty-state CTAs; the old always-visible banner
// was removed — pages show guidance only where there is nothing to show yet.
// Also hosts the gamification helpers (streaks + badges) used by the
// Achievements page.

import { tauri, ymdLocal } from "./ipc";
import { icons } from "./icons";
import { isRun, runKm } from "./sport";
import type { Activity, ProviderStatus, TrainingPlan } from "./types";

export interface FlowState {
  connected: boolean;
  lastSync: string | null;
  plan: TrainingPlan | null;
  pushed: boolean;
}

export async function fetchFlowState(): Promise<FlowState> {
  const [statuses, current] = await Promise.all([
    tauri.providerStatus(),
    tauri.currentPlan().catch(() => null),
  ]);
  const coros = statuses.find((s: ProviderStatus) => s.provider === "coros");
  return {
    connected: coros?.connected ?? false,
    lastSync: coros?.lastSync ?? null,
    plan: current?.[0] ?? null,
    // `?.` on the element too: `current` is a tuple, so a null plan inside it
    // must read as "no plan" rather than throwing (which callers report as
    // "backend unreachable" — the wrong diagnosis).
    pushed: current?.[0]?.status === "pushed",
  };
}

/// The step the user should act on next (for empty-state CTAs).
export function nextAction(state: FlowState): "connect" | "sync" | "plan" | "train" {
  if (!state.connected) return "connect";
  if (state.lastSync == null) return "sync";
  if (!state.plan) return "plan";
  return "train";
}

// ---- gamification -----------------------------------------------------------

export interface Achievement {
  id: string;
  name: string;
  desc: string;
  icon: string;
  earned: boolean;
  /// Progress towards the goal, for count-based badges.
  progress?: number;
  target?: number;
  /// Unit shown with the progress numbers, e.g. "days" or "km".
  unit?: string;
}

/// Everything the Achievements page derives from synced activities.
export interface GamificationStats {
  streak: number;
  bestStreak: number;
  totalKm: number;
  weekKm: number;
  monthKm: number;
  sessions: number;
  totalHours: number;
  longestKm: number;
  /// Non-running training, credited separately from the distance stats.
  crossHours: number;
  crossSessions: number;
  achievements: Achievement[];
  /// Distance (km) per day for the last 84 days, oldest first. Days with no
  /// activity are present with 0 so the heatmap grid is complete.
  recent: { date: string; km: number; cross: boolean }[];
}

/// Consecutive days (ending today or yesterday) that have an activity.
export function currentStreak(activities: Activity[]): number {
  const days = new Set(activities.map((a) => a.startTime.slice(0, 10)));
  let streak = 0;
  const d = new Date();
  // Allow the streak to hang on yesterday even if today has no run yet.
  // Keys are local calendar days — toISOString() would shift them a day in
  // UTC+ timezones and break the streak.
  if (!days.has(ymdLocal(d))) d.setDate(d.getDate() - 1);
  while (days.has(ymdLocal(d))) {
    streak++;
    d.setDate(d.getDate() - 1);
  }
  return streak;
}

/// Longest single run (km) in the synced history. Run-only: the longest ride
/// is not a running record, and a 90 km spin would hand out every distance
/// badge in the book.
function longestRunKm(activities: Activity[]): number {
  return activities.filter(isRun).reduce((m, a) => Math.max(m, a.distanceM), 0) / 1000;
}

/// Total running distance (km) logged over the last `days` days, today
/// included.
function kmWithin(activities: Activity[], days: number): number {
  const cutoff = new Date();
  cutoff.setDate(cutoff.getDate() - (days - 1));
  const from = ymdLocal(cutoff);
  return runKm(activities.filter((a) => a.startTime.slice(0, 10) >= from));
}

/// The longest run of consecutive active days anywhere in the history.
function bestStreak(activities: Activity[]): number {
  const days = [...new Set(activities.map((a) => a.startTime.slice(0, 10)))].sort();
  let best = 0;
  let run = 0;
  let prev: number | null = null;
  for (const day of days) {
    const time = Date.parse(`${day}T00:00:00Z`);
    run = prev != null && time - prev === 86_400_000 ? run + 1 : 1;
    best = Math.max(best, run);
    prev = time;
  }
  return best;
}

/// Local (device) hour the activity started.
function hourStart(a: Activity): number {
  return new Date(a.startTime).getHours();
}

/// Local weekday, 0 = Sunday.
function localDow(a: Activity): number {
  return new Date(a.startTime).getDay();
}

/// Month key ("YYYY-MM") of a date, local time.
function monthKey(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
}

/// Longest gap (days) between consecutive active days.
function longestBreakDays(activities: Activity[]): number {
  const days = [...new Set(activities.map((a) => a.startTime.slice(0, 10)))]
    .map((d) => Date.parse(`${d}T00:00:00Z`))
    .sort((x, y) => x - y);
  let gap = 0;
  for (let i = 1; i < days.length; i++) {
    gap = Math.max(gap, Math.round((days[i] - days[i - 1]) / 86_400_000) - 1);
  }
  return gap;
}

/// Whether the runner resumed after a break of at least `min` days somewhere
/// in the history (a gap followed by another activity).
function cameBackFrom(activities: Activity[], min: number): boolean {
  const days = [...new Set(activities.map((a) => a.startTime.slice(0, 10)))]
    .map((d) => Date.parse(`${d}T00:00:00Z`))
    .sort((x, y) => x - y);
  for (let i = 1; i < days.length; i++) {
    if (Math.round((days[i] - days[i - 1]) / 86_400_000) - 1 >= min) return true;
  }
  return false;
}

/// Distinct local months (YYYY-MM) containing at least one activity.
function activeMonths(activities: Activity[]): number {
  return new Set(activities.map((a) => a.startTime.slice(0, 7))).size;
}

/// Average pace (s/km) over the window ]now - fromDays, now - toDays], or
/// null when nothing was run (or a run has no pace recorded). Runs only — a
/// bike's speed would otherwise read as extraordinary running form.
function avgPaceBetween(activities: Activity[], fromDays: number, toDays: number): number | null {
  const now = new Date();
  const lo = new Date(now);
  lo.setDate(lo.getDate() - fromDays);
  const hi = new Date(now);
  hi.setDate(hi.getDate() - toDays);
  const loKey = ymdLocal(lo);
  const hiKey = ymdLocal(hi);
  const recent = activities.filter(
    (a) => isRun(a) && a.startTime.slice(0, 10) > loKey && a.startTime.slice(0, 10) <= hiKey && a.avgPaceSKm != null && a.distanceM > 0,
  );
  if (recent.length === 0) return null;
  return recent.reduce((s, a) => s + a.distanceM * (a.avgPaceSKm as number), 0) / recent.reduce((s, a) => s + a.distanceM, 0);
}

/// Whether a run started on the Nth-to-last day of its calendar month.
function ranMonthEnd(activities: Activity[]): boolean {
  return activities.some((a) => {
    const d = new Date(a.startTime);
    const last = new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate();
    return d.getDate() === last;
  });
}

/// Whether a run started on Friday the 13th.
function ranFriday13(activities: Activity[]): boolean {
  return activities.some((a) => {
    const d = new Date(a.startTime);
    return d.getDay() === 5 && d.getDate() === 13;
  });
}

/// Whether runs happened on both the first and last day of the same month.
function ranBookends(activities: Activity[]): boolean {
  const firsts = new Set<string>();
  const lasts = new Set<string>();
  for (const a of activities) {
    const d = new Date(a.startTime);
    const key = monthKey(d);
    if (d.getDate() === 1) firsts.add(key);
    if (d.getDate() === new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate()) lasts.add(key);
  }
  return [...firsts].some((k) => lasts.has(k));
}

/// Pace in seconds per km rendered as "m:ss", for badge copy.
function paceLabel(s: number): string {
  return `${Math.floor(s / 60)}:${String(Math.round(s % 60)).padStart(2, "0")}`;
}

/// ISO week key (YYYY-Www) of a local date — used for same-week badges.
function isoWeek(d: Date): string {
  const date = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
  const day = (date.getUTCDay() + 6) % 7;
  date.setUTCDate(date.getUTCDate() - day + 3);
  const firstThursday = new Date(Date.UTC(date.getUTCFullYear(), 0, 4));
  const week = 1 + Math.round(((date.getTime() - firstThursday.getTime()) / 86_400_000 - 3 + ((firstThursday.getUTCDay() + 6) % 7)) / 7);
  return `${date.getUTCFullYear()}-W${String(week).padStart(2, "0")}`;
}

/// All streak/badge data for the Achievements page, derived from the full
/// synced history (the dashboard range toggle does not apply here).
export function gamificationStats(
  activities: Activity[],
  plan: TrainingPlan | null,
  pushed: boolean,
): GamificationStats {
  // Two populations, and mixing them is the bug this comment exists to
  // prevent. `runs` carries every distance, pace, elevation and per-run fact
  // (lib/sport.ts, mirroring the backend's RUN_SQL) — a bike day is not a
  // running kilometre. `activities` carries the facts about SHOWING UP:
  // streaks, sessions, hours, active months. A ride keeps a streak alive and
  // earns a session badge, because "I trained today" is true either way.
  const runs = activities.filter(isRun);
  const cross = activities.filter((a) => !isRun(a));
  const totalKm = runKm(runs);
  const streak = currentStreak(activities);
  const longestKm = longestRunKm(runs);

  // ---- facts the badge catalog is evaluated against -------------------------
  const sessions = activities.length;
  const totalHours = activities.reduce((s, a) => s + a.durationS, 0) / 3600;
  const crossHours = cross.reduce((s, a) => s + a.durationS, 0) / 3600;
  const crossSessions = cross.length;
  const kmByDay = new Map<string, number>();
  const crossByDay = new Set<string>();
  for (const a of activities) {
    const day = a.startTime.slice(0, 10);
    if (!isRun(a)) {
      crossByDay.add(day);
      continue;
    }
    kmByDay.set(day, (kmByDay.get(day) ?? 0) + a.distanceM / 1000);
  }
  const bestDayKm = Math.max(0, ...kmByDay.values());
  const totalElevM = runs.reduce((s, a) => s + (a.elevationGainM ?? 0), 0);
  const maxElevM = Math.max(0, ...runs.map((a) => a.elevationGainM ?? 0));
  // Best distance-weighted average pace among real runs (>= 5 km), s/km.
  const paced = runs.filter((a) => a.avgPaceSKm != null && a.distanceM >= 5_000);
  const bestPaceSKm = paced.length ? Math.min(...paced.map((a) => a.avgPaceSKm as number)) : Infinity;
  const maxHr = Math.max(0, ...runs.map((a) => a.maxHr ?? 0));
  const zone2Run = runs.some(
    (a) => a.distanceM >= 10_000 && a.avgHr != null && a.avgHr > 0 && a.avgHr <= 140,
  );
  const trackedRuns = runs.filter((a) => a.hasTrack).length;
  const runsPerDay = new Map<string, number>();
  for (const a of runs) {
    const day = a.startTime.slice(0, 10);
    runsPerDay.set(day, (runsPerDay.get(day) ?? 0) + 1);
  }
  const doubleDay = Math.max(0, ...runsPerDay.values());
  const earlyRuns = runs.filter((a) => hourStart(a) < 7).length;
  const dawnRuns = runs.filter((a) => hourStart(a) >= 3 && hourStart(a) < 6).length;
  const nightRuns = runs.filter((a) => hourStart(a) >= 21).length;
  const lunchRuns = runs.filter((a) => hourStart(a) >= 11 && hourStart(a) < 14).length;
  const mondayRuns = runs.filter((a) => localDow(a) === 1).length;
  const fridayRuns = runs.filter((a) => localDow(a) === 5).length;
  const sundayLong = runs.some((a) => localDow(a) === 0 && a.distanceM >= 15_000);
  const satWeeks = new Set(runs.filter((a) => localDow(a) === 6).map((a) => isoWeek(new Date(a.startTime))));
  const weekendWarrior = runs.some(
    (a) => localDow(a) === 0 && satWeeks.has(isoWeek(new Date(a.startTime))),
  );
  const ranNewYear = runs.some((a) => {
    const d = new Date(a.startTime);
    return d.getMonth() === 0 && d.getDate() === 1;
  });
  const ranChristmas = runs.some((a) => {
    const d = new Date(a.startTime);
    return d.getMonth() === 11 && d.getDate() === 25;
  });
  const form30 = avgPaceBetween(activities, 30, 0);
  const form90 = avgPaceBetween(activities, 90, 0);
  const months = activeMonths(activities);
  const breakDays = longestBreakDays(activities);

  // ---- the badge catalog ----------------------------------------------------
  // Count-based badge: progress bar towards `target`.
  const count = (
    id: string,
    name: string,
    desc: string,
    icon: string,
    value: number,
    target: number,
    unit: string,
  ): Achievement => ({
    id,
    name,
    desc,
    icon,
    earned: value >= target,
    progress: Math.min(value, target),
    target,
    unit,
  });
  // All-or-nothing badge: no progress bar.
  const done = (id: string, name: string, desc: string, icon: string, earned: boolean): Achievement => ({
    id,
    name,
    desc,
    icon,
    earned,
  });
  // Distance ladder over all-time km, with a hand-picked name per rung.
  const kmLadder: [number, string][] = [
    [5, "Warm-up crew"],
    [10, "Ten and holding"],
    [21.1, "Half marathoner"],
    [42.2, "Marathon worth"],
    [50, "Fifty fiver"],
    [150, "Century club runner-up"],
    [200, "Two hundred strong"],
    [300, "Three-figure grinder"],
    [400, "Four hundred hard"],
    [750, "Ultra committed"],
    [1000, "Metric marathoner"],
    [1500, "Long hauler"],
    [2000, "Two-tonne engine"],
    [2500, "Quarter myriad"],
    [3000, "Road warrior"],
    [4000, "Endurance legend"],
    [5000, "Runalytics hall of fame"],
    [10000, "Ten-thousand club"],
    [20000, "Twice around Britain"],
    [40075, "Earth's circumference"],
    [100000, "Three laps of the globe"],
    [384400, "To the moon"],
  ];
  const streakLadder: [number, string][] = [
    [3, "Hat-trick"],
    [14, "Fortnight on fire"],
    [21, "Three weeks strong"],
    [50, "Half-century streak"],
    [60, "Sixty days solid"],
    [100, "Hundred days on"],
    [150, "Streak marathoner"],
    [200, "Streak ultra"],
    [365, "A full lap of the sun"],
    [730, "Two years unbroken"],
    [1000, "One thousand days"],
    [3650, "A decade every day"],
  ];
  const sessionLadder: [number, string][] = [
    [10, "Double digits"],
    [25, "Quarter grand"],
    [50, "Half a hundred outs"],
    [100, "Century of outs"],
    [250, "Quarter-thousand club"],
    [500, "Five hundred strong"],
    [1000, "Thousand sessions"],
    [2000, "Two thousand outs"],
    [5000, "Five thousand outs"],
    [10000, "Ten thousand mornings"],
  ];
  const hourLadder: [number, string][] = [
    [10, "Ten hours logged"],
    [25, "A work week done"],
    [50, "Fifty hours"],
    [100, "Time lord"],
    [250, "Quarter thousand hours"],
    [500, "Five hundred hours"],
    [1000, "Millennium runner"],
    [8760, "A whole year moving"],
    [17520, "Two years of daylight"],
  ];
  const longLadder: [number, string][] = [
    [5, "First five"],
    [10, "Ten-kilometre club"],
    [15, "Fifteen-mile mindset"],
    [25, "Twenty-five deep"],
    [30, "Thirty-klick wall"],
    [42.2, "Full distance"],
    [50, "Ultra distance"],
  ];
  const weekLadder: [number, string][] = [
    [10, "Ten in a week"],
    [21.1, "Half per week"],
    [42.2, "Marathon week"],
    [50, "Fifty-a-week"],
    [75, "Seventy-five a week"],
    [100, "One-hundred-k week"],
  ];
  const monthLadder: [number, string][] = [
    [100, "Hundred-a-month"],
    [150, "Month of miles"],
    [200, "Two-hundred month"],
    [300, "Monster month"],
    [400, "God-tier month"],
  ];
  const elevLadder: [number, string][] = [
    [100, "Hill curious"],
    [500, "Stair climber"],
    [1000, "Everest-ish"],
    [2500, "Four Everests"],
    [5000, "Two Everests… twice"],
    [10000, "Everest xii"],
    [88490, "Summit Everest in climbing"],
  ];
  const paceLadder: [number, string][] = [
    [360, "Sub 6:00"],
    [330, "Sub 5:30"],
    [300, "Sub 5:00"],
    [285, "Sub 4:45"],
    [270, "Sub 4:30"],
    [240, "Sub 4:00"],
  ];

  const badgeList: Achievement[] = [
    // The original eight, kept first for continuity.
    {
      id: "first-run",
      name: "First mile",
      desc: "Log your first run",
      icon: icons.flag,
      earned: runs.length > 0,
    },
    {
      id: "streak-7",
      name: "Week warrior",
      desc: "7-day activity streak",
      icon: icons.flame,
      earned: streak >= 7,
      progress: Math.min(streak, 7),
      target: 7,
      unit: "days",
    },
    {
      id: "km-100",
      name: "Century",
      desc: "100 km run",
      icon: icons.target,
      earned: totalKm >= 100,
      progress: Math.min(totalKm, 100),
      target: 100,
      unit: "km",
    },
    {
      id: "streak-30",
      name: "Month of momentum",
      desc: "30-day activity streak",
      icon: icons.flame,
      earned: streak >= 30,
      progress: Math.min(streak, 30),
      target: 30,
      unit: "days",
    },
    {
      id: "half-marathon",
      name: "Halfway to glory",
      desc: "One run of 21.1 km",
      icon: icons.run,
      earned: longestKm >= 21.1,
      progress: Math.min(longestKm, 21.1),
      target: 21.1,
      unit: "km",
    },
    {
      id: "km-500",
      name: "Iron legs",
      desc: "500 km run",
      icon: icons.trophy,
      earned: totalKm >= 500,
      progress: Math.min(totalKm, 500),
      target: 500,
      unit: "km",
    },
    {
      id: "plan",
      name: "Raceday bound",
      desc: "Create a training plan",
      icon: icons.calendar,
      earned: plan != null,
    },
    {
      id: "pushed",
      name: "On the watch",
      desc: "Plan synced to COROS",
      icon: icons.watch,
      earned: pushed,
    },
    // Distance ladders (all-time running km).
    ...kmLadder.map(([km, name]) =>
      count(`km-${km}`, name, `${km} km run`, icons.run, totalKm, km, "km"),
    ),
    // Streak ladders.
    ...streakLadder.map(([days, name]) =>
      count(`streak-${days}`, name, `${days}-day activity streak`, icons.flame, streak, days, "days"),
    ),
    // Session counts — every sport, so the unit says sessions and not runs.
    // A stack of cards, not a bar chart: these badges count events, they do
    // not plot a quantity.
    ...sessionLadder.map(([n, name]) =>
      count(`sessions-${n}`, name, `${n} sessions synced`, icons.stack, sessions, n, "sessions"),
    ),
    // Time on your feet.
    ...hourLadder.map(([h, name]) =>
      count(`hours-${h}`, name, `${h} hours moving`, icons.clock, totalHours, h, "hours"),
    ),
    // Longest single run — a record for one effort, so it wears the medal the
    // other single-effort records wear. `star` was shared with New Year and
    // Weekend warrior, which made it mean nothing in particular.
    ...longLadder.map(([km, name]) =>
      count(`long-${km}`, name, `One run of ${km} km`, icons.medal, longestKm, km, "km"),
    ),
    // Volume in a rolling week / month. A bar chart is literally what this is
    // — distance piled up across a window — so it wears the chart glyph and
    // leaves `bolt` for the two things that are actually about intensity.
    ...weekLadder.map(([km, name]) =>
      count(`week-${km}`, name, `${km} km inside one week`, icons.chart, kmWithin(activities, 7), km, "km"),
    ),
    ...monthLadder.map(([km, name]) =>
      count(`month-${km}`, name, `${km} km inside one month`, icons.chart, kmWithin(activities, 30), km, "km"),
    ),
    // One huge day — same single-effort record family as the longest run.
    count("day-21", "One-day half", "21.1 km in a single day", icons.medal, bestDayKm, 21.1, "km"),
    count("day-42", "One-day marathon", "42.2 km in a single day", icons.medal, bestDayKm, 42.2, "km"),
    // Pace — best average pace on any run of 5 km or more. The bolt is speed,
    // which is the one thing this ladder is about.
    ...paceLadder.map(([sec, name]) =>
      done(
        `pace-${sec}`,
        name,
        `Average ${paceLabel(sec)} /km or faster on a 5 km+ run`,
        icons.bolt,
        bestPaceSKm <= sec,
      ),
    ),
    // Recent form — distance-weighted average pace over the window. A rising
    // line: this is the one badge family that measures a trend, so it gets the
    // trend glyph rather than the bar chart it used to borrow.
    done(
      "form-30",
      "Sharp month",
      `Average under ${paceLabel(330)} /km over the last 30 days`,
      icons.trend,
      form30 != null && form30 < 330,
    ),
    done(
      "form-90",
      "Season of speed",
      `Average under ${paceLabel(300)} /km over the last 90 days`,
      icons.trend,
      form90 != null && form90 < 300,
    ),
    // Elevation (gained on foot — a ride's altitude is not climbing for a
    // runner's legs).
    ...elevLadder.map(([m, name]) =>
      count(`elev-${m}`, name, `${m.toLocaleString()} m of climbing on runs`, icons.mountain, totalElevM, m, "m"),
    ),
    count("climb-100", "Tough ascent", "100 m climbed in one run", icons.mountain, maxElevM, 100, "m"),
    count("climb-250", "Mountain goat", "250 m climbed in one run", icons.mountain, maxElevM, 250, "m"),
    count("climb-500", "Peak bagger", "500 m climbed in one run", icons.mountain, maxElevM, 500, "m"),
    count("climb-1000", "Alpinist", "1,000 m climbed in one run", icons.mountain, maxElevM, 1000, "m"),
    // Time of day.
    done("early-bird", "Early bird", "A run before 7:00", icons.sunrise, earlyRuns > 0),
    count("early-x25", "Dawn patrol", "25 runs before 7:00", icons.sunrise, earlyRuns, 25, "runs"),
    done("dawn", "Golden hour", "A run between 3:00 and 6:00", icons.sunrise, dawnRuns > 0),
    done("night-owl", "Night owl", "A run after 21:00", icons.moon, nightRuns > 0),
    done("lunch-break", "Lunch lap", "A run between 11:00 and 14:00", icons.clock, lunchRuns > 0),
    done("monday-club", "Monday club", "A run on a Monday", icons.calendar, mondayRuns > 0),
    done("friday-fever", "Friday fever", "A run on a Friday", icons.calendar, fridayRuns > 0),
    // A Sunday long run is a distance achievement, so it wears the same glyph
    // the distance ladders wear — a heart said nothing about running.
    done("sunday-long", "Sunday long run", "A 15 km+ run on a Sunday", icons.run, sundayLong),
    done("weekend-warrior", "Weekend warrior", "Runs on both days of one weekend", icons.calendar, weekendWarrior),
    // Half a filled disc: the two ends of the clock. A globe read as
    // "worldwide", which has nothing to do with 5am and 9pm.
    done("night-and-day", "Night & day", "Runs before 6:00 and after 21:00", icons.contrast, dawnRuns > 0 && nightRuns > 0),
    // Calendar oddities. A flag means race day everywhere else in the app, so
    // the festive and superstitious days get glyphs that say what they are.
    done("new-year", "New year, new miles", "A run on January 1st", icons.star, ranNewYear),
    done("christmas-run", "Santa strides", "A run on December 25th", icons.gift, ranChristmas),
    done("friday-13", "Unlucky charm", "A run on Friday the 13th", icons.alert, ranFriday13(activities)),
    done("month-end", "Month closer", "A run on the last day of a month", icons.calendar, ranMonthEnd(activities)),
    done("bookends", "Month bookends", "First and last day of one month", icons.calendar, ranBookends(activities)),
    count("months-6", "Half-year habit", "Active in 6 different months", icons.calendar, months, 6, "months"),
    // A calendar showing all its days, not a globe: the achievement is twelve
    // months of training, not travel.
    count("months-12", "Year-round runner", "Active in 12 different months", icons.calendarYear, months, 12, "months"),
    // Comebacks and records. `sync` means "data moved to the watch"; a
    // comeback is the runner repeating the habit, so it gets the repeat loop.
    done("comeback-7", "Back in the saddle", "Returned after a 7-day break", icons.repeat, cameBackFrom(activities, 7)),
    done("comeback-30", "The long way back", "Returned after a 30-day break", icons.repeat, cameBackFrom(activities, 30)),
    done(
      "record-equal",
      "Personal best streak",
      "Your current streak is your best ever (7+)",
      icons.trophy,
      streak >= 7 && streak >= bestStreak(activities),
    ),
    // A bed, not a moon: the moon already means "night run", and a fortnight
    // off is rest rather than night-time.
    done("rest-day", "Even legends rest", "A break of 14+ days somewhere", icons.bed, breakDays >= 14),
    // Doubles and triples — two or three sessions stacked into one day.
    done("double-day", "Double day", "Two runs in one day", icons.stack, doubleDay >= 2),
    done("triple-day", "Triple threat", "Three runs in one day", icons.stack, doubleDay >= 3),
    // Heart-rate flavour.
    done("redline", "Redline", "A run that touched 180 bpm", icons.heart, maxHr >= 180),
    done("zone-two", "Zone two hero", "A 10 km+ run averaging under 140 bpm", icons.heart, zone2Run),
    // GPS.
    count("tracked-100", "Breadcrumb trail", "100 runs with a GPS track", icons.map, trackedRuns, 100, "runs"),
    // Cross-training earns its own badges. Without these the honest split
    // above would read as a punishment: a week of cycling used to buy running
    // distance badges, and now it must buy its own.
    count("cross-10", "Cross-trainer", "10 sessions away from the road", icons.bike, crossSessions, 10, "sessions"),
    count("cross-50", "Versatile engine", "50 sessions away from the road", icons.bike, crossSessions, 50, "sessions"),
    count("cross-100h", "All-round athlete", "100 h of cross-training", icons.clock, crossHours, 100, "hours"),
  ];
  // Per-day distances for the last 12 weeks (heatmap columns), oldest first.
  // kmByDay (built with the badge facts above) already holds every day's km;
  // `cross` marks a day trained some other way, so a rested-from-running day
  // the runner actually cycled is not drawn as a blank.
  const recent: { date: string; km: number; cross: boolean }[] = [];
  for (let i = 83; i >= 0; i--) {
    const d = new Date();
    d.setDate(d.getDate() - i);
    const key = ymdLocal(d);
    recent.push({ date: key, km: kmByDay.get(key) ?? 0, cross: crossByDay.has(key) });
  }
  return {
    streak,
    bestStreak: bestStreak(activities),
    totalKm,
    weekKm: kmWithin(activities, 7),
    monthKm: kmWithin(activities, 30),
    sessions,
    totalHours,
    longestKm,
    crossHours,
    crossSessions,
    achievements: badgeList,
    recent,
  };
}
