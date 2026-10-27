import type { Execution, PlanWorkout, TrainingPlan } from "./types";

/// How a session landed against its range, in the words the app uses
/// everywhere. Lives next to [`qualityBand`] so the plan page, the activity
/// sheet and any later surface cannot each invent their own verdict wording.
export const EXECUTION_LABEL: Record<Execution, string> = {
  ahead: "too fast",
  in: "in range",
  behind: "too slow",
};

/// The pace band a session actually asked for, offset included — the range the
/// runner's reps are graded against, and the one every surface should draw.
///
/// A plan-level pacing adjustment (`quality_offset_sec_km`, accepted from the
/// pacing panel) moves only the tempo and interval ranges, and only from the
/// day it took effect. Sessions dated before that keep the range they were run
/// against, which is what lets a completed session be scored honestly and lets
/// an easy day never be touched by a decision about interval pace.
///
/// Mirrors `session::quality_band` in the Rust plan engine; the two must not
/// drift, or the app shows one range and grades against another.
export function qualityBand(
  plan: TrainingPlan,
  w: PlanWorkout,
): [number, number] | null {
  if (w.paceLoSKm == null || w.paceHiSKm == null) return null;
  if (w.kind !== "intervals" && w.kind !== "tempo") return [w.paceLoSKm, w.paceHiSKm];
  const from = plan.qualityOffsetFrom;
  const offset = from != null && w.date >= from ? plan.qualityOffsetSecKm : 0;
  if (offset === 0) return [w.paceLoSKm, w.paceHiSKm];
  const shift = (p: number) => Math.max(1, Math.round(p + offset));
  return [shift(w.paceLoSKm), shift(w.paceHiSKm)];
}
