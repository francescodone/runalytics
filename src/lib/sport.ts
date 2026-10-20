// Sport classification, shared by every screen that counts distance.
//
// The backend keeps running mileage separate from everything else: `RUN_SQL`
// in store.rs filters the plan baseline, weekly volumes and load targets down
// to running sessions, because a 90 km spin must not set a running plan's
// baseline. The frontend used to receive the *whole* activity list and sum it
// as if it were running, so a bike day silently inflated the mileage chart,
// the streak and the coach's brief while the plan engine ignored it. These
// predicates are the frontend half of that same rule — keep them in step with
// RUN_SQL, or the two halves disagree about what a running kilometre is.

import type { Activity } from "./types";

/// Mirrors `RUN_SQL` in src-tauri/src/store.rs. COROS labels a run "run", a
/// FIT session carries the Garmin enum "running", and trail/track sessions
/// arrive as e.g. "trail run" — hence the substring fallback.
export function isRunSport(sport: string): boolean {
  const s = sport.toLowerCase();
  return s === "run" || s === "running" || s === "jog" || s === "jogging" || s.includes("run");
}

/// Cycling in any of the labels the ingest paths produce ("ride" from COROS,
/// "cycling" from a FIT enum, or a raw "bike" from a backfilled row).
export function isBikeSport(sport: string): boolean {
  const s = sport.toLowerCase();
  return s.includes("cycl") || s.includes("ride") || s.includes("bike");
}

export const isRun = (a: Activity): boolean => isRunSport(a.sport);

/// Training that counts as effort but not as running mileage: cycling, swim,
/// strength, hike — anything the runner did that the plan cannot bank.
export const isCrossTraining = (a: Activity): boolean => !isRun(a);

/// Running distance in a list, in km. The only distance that may be presented
/// as mileage.
export function runKm(activities: Activity[]): number {
  return activities.filter(isRun).reduce((s, a) => s + a.distanceM, 0) / 1000;
}

/// One non-running sport, with only its own numbers attached.
export interface CrossSport {
  /// The sport as a noun, for "3 h of cycling".
  activity: string;
  km: number;
  hours: number;
  sessions: number;
  /// Distinct local calendar days THIS sport happened on.
  days: number;
  /// Whether distance means anything for this sport. A gym session reports
  /// sets, not distance, so "0 km of strength work" would be a lie.
  hasDistance: boolean;
}

/// What the runner did instead of running, ready to be credited in prose.
/// The per-sport breakdown is not decoration: a summary that pooled the
/// figures and then named one sport would claim "90 km of cycling over 2
/// days" when the second day was a gym session.
export interface CrossTraining {
  sessions: number;
  hours: number;
  /// Distinct local calendar days with any cross-training session.
  days: number;
  /// Every sport, biggest first.
  bySport: CrossSport[];
}

/// A sport label a runner would recognise, from whatever the device sent.
function sportNoun(raw: string): { noun: string; hasDistance: boolean } {
  const s = raw.toLowerCase();
  const strength = s.includes("strength") || s.includes("gym") || s.includes("yoga");
  const noun = isBikeSport(s)
    ? "cycling"
    : s.includes("swim")
      ? "swimming"
      : s.includes("hik")
        ? "hiking"
        : s.includes("row")
          ? "rowing"
          : s.includes("walk")
            ? "walking"
            : strength
              ? "strength work"
              : raw;
  return { noun, hasDistance: !strength };
}

/// Summarise the non-running sessions. Returns null when there is nothing to
/// credit, so callers drop the whole clause instead of rendering a zero.
export function crossTraining(activities: Activity[]): CrossTraining | null {
  const cross = activities.filter(isCrossTraining);
  if (cross.length === 0) return null;
  const tally = new Map<string, { km: number; hours: number; n: number; days: Set<string> }>();
  for (const a of cross) {
    const cur = tally.get(a.sport) ?? { km: 0, hours: 0, n: 0, days: new Set<string>() };
    cur.km += a.distanceM / 1000;
    cur.hours += a.durationS / 3600;
    cur.n += 1;
    cur.days.add(a.startTime.slice(0, 10));
    tally.set(a.sport, cur);
  }
  const bySport = [...tally.entries()]
    .map(([raw, v]) => {
      const { noun, hasDistance } = sportNoun(raw);
      return {
        activity: noun,
        hasDistance,
        km: v.km,
        hours: v.hours,
        sessions: v.n,
        days: v.days.size,
      };
    })
    .sort((x, y) => y.hours - x.hours || y.km - x.km || y.sessions - x.sessions);
  return {
    sessions: cross.length,
    hours: cross.reduce((x, a) => x + a.durationS, 0) / 3600,
    days: new Set(cross.map((a) => a.startTime.slice(0, 10))).size,
    bySport,
  };
}

/// One sport's contribution, in the unit that sport actually reports.
function sportPhrase(s: CrossSport): string {
  if (s.hasDistance && s.km >= 1) return `${s.km.toFixed(0)} km ${s.activity}`;
  return s.hours >= 1
    ? `${s.hours.toFixed(1)} h ${s.activity}`
    : `${Math.round(s.hours * 60)} min ${s.activity}`;
}

/// Everything that was cross-trained, each sport with its own figure. Two
/// sports are named in full; beyond that the largest leads and the rest is
/// counted, so the clause stays a sentence and not a table.
export function crossPhrase(c: CrossTraining): string {
  const parts = c.bySport.map(sportPhrase);
  if (parts.length <= 1) return parts[0];
  if (parts.length === 2) return `${parts[0]} + ${parts[1]}`;
  return `${parts[0]} + ${parts.length - 1} more`;
}
