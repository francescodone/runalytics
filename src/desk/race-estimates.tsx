import { t, duration as fmtDuration, date as fmtDateDMY } from "./i18n";
import type { PlanDefaults, PlanWorkout, TrainingPlan } from "../lib/types";
import { dateValue, monday, today } from "./data";
import { CountUp, InfoDialog } from "./ui";
import { fmtPace } from "../lib/ipc";

export function trainingRaceTime(threshold: number, offset: number, kilometers: number): number {
  const effective = threshold + offset;
  return Number.isFinite(effective) && effective > 0 ? Math.round(3600 * Math.pow(kilometers / (3600 / effective), 1.06)) : 0;
}

export function currentRaceTime(heuristicSeconds: number, watchSeconds?: number | null): number {
  const heuristic = Number.isFinite(heuristicSeconds) && heuristicSeconds > 0 ? heuristicSeconds : 0;
  const watch = typeof watchSeconds === "number" && Number.isFinite(watchSeconds) && watchSeconds > 0 ? watchSeconds : 0;
  return Math.round(heuristic && watch ? (heuristic + watch) / 2 : heuristic || watch);
}

export function raceDayThreshold(plan: TrainingPlan, workouts: PlanWorkout[], currentDate: string): number | null {
  const baseline = plan.baselineWeeklyKm;
  const weeksLeft = Math.ceil((dateValue(plan.targetDate).getTime() - dateValue(currentDate).getTime()) / 604800000);
  if (plan.kind !== "race" || (plan.status !== "active" && plan.status !== "pushed") || weeksLeft <= 0 || baseline == null || !Number.isFinite(baseline) || baseline <= 0) return null;
  const weekly = new Map<string, number>();
  for (const workout of workouts) {
    if (workout.kind === "race" || workout.date < monday(currentDate) || workout.date >= plan.targetDate || workout.distanceM == null || !Number.isFinite(workout.distanceM) || workout.distanceM <= 0) continue;
    const week = monday(workout.date);
    weekly.set(week, (weekly.get(week) ?? 0) + workout.distanceM / 1000);
  }
  if (!weekly.size) return null;
  const gain = Math.max(0, Math.max(...weekly.values()) / baseline - 1);
  return (plan.thresholdSecKm + (plan.qualityOffsetSecKm ?? 0)) * (1 - Math.min(0.04, gain * 0.5, weeksLeft * 0.005));
}

export function RaceEstimates({ plan, proposedOffset, watch, workouts, compact = false }: { plan: TrainingPlan; proposedOffset?: number; watch?: Pick<PlanDefaults, "predict5kS" | "predict10kS" | "predictHalfS" | "predictMarathonS"> | null; workouts?: PlanWorkout[]; compact?: boolean }) {
  const current = plan.qualityOffsetSecKm ?? 0;
  const preview = proposedOffset != null;
  const stacked = compact && !preview;
  const description = preview ? t("Race estimates from the plan threshold and accepted pace adjustment") : t("Current race estimates from the plan heuristic and watch predictions");
  const projectedThreshold = raceDayThreshold(plan, workouts ?? [], today());
  return <section className={`race-estimates${stacked ? " pace-forecast-estimates" : ""}`} aria-label={preview ? t("Proposed race estimates") : t("Training race estimates")}>
    <h3>{preview ? t("Race estimate changes") : t("Training race estimates")}</h3>
    <p className="muted">{plan.qualityOffsetFrom ? t(" / Last adjustment {{value0}}", { value0: fmtDateDMY(plan.qualityOffsetFrom) }).replace(/^\s*\/\s*/, "") : t(" / Plan baseline").replace(/^\s*\/\s*/, "")}</p>
    <div className="data-table-scroll" tabIndex={0} role="region" aria-label={description}><table role={stacked ? "table" : undefined}><caption className="sr-only">{description}</caption><thead className={stacked ? "sr-only" : undefined} role={stacked ? "rowgroup" : undefined}><tr role={stacked ? "row" : undefined}><th scope="col" role={stacked ? "columnheader" : undefined}>{t("Distance")}</th><th scope="col" role={stacked ? "columnheader" : undefined}>{preview ? t("Current") : t("Today")}</th>{preview ? <th scope="col">{t("Proposed")}</th> : <th scope="col" role={stacked ? "columnheader" : undefined}>{t("Race day projection")}</th>}</tr></thead><tbody role={stacked ? "rowgroup" : undefined}>{([["5 km",5,watch?.predict5kS],["10 km",10,watch?.predict10kS],[t("Half marathon"),21.0975,watch?.predictHalfS],[t("Marathon"),42.195,watch?.predictMarathonS]] as const).map(([label,kilometers,watchSeconds]) => {
      const heuristic = trainingRaceTime(plan.thresholdSecKm,current,Number(kilometers));
      const seconds = preview ? heuristic : currentRaceTime(heuristic, watchSeconds);
      const proposed = trainingRaceTime(plan.thresholdSecKm,proposedOffset ?? current,Number(kilometers));
      const projection = projectedThreshold != null ? trainingRaceTime(projectedThreshold, 0, Number(kilometers)) : 0;
      return <tr key={String(label)} role={stacked ? "row" : undefined}><th scope="row" role={stacked ? "rowheader" : undefined}>{label}</th><td data-seconds={seconds} role={stacked ? "cell" : undefined}>{stacked && <span className="pace-estimate-label" aria-hidden="true">{t("Today")}</span>}{seconds ? <CountUp format={progress => fmtDuration(seconds * progress, Number(kilometers) <= 10)} /> : t("Not available")}</td>{preview ? <td data-seconds={proposed}>{proposed ? <CountUp format={progress => fmtDuration(proposed * progress, Number(kilometers) <= 10)} /> : t("Not available")}</td> : <td data-projection={projection || undefined} role={stacked ? "cell" : undefined}>{stacked && <span className="pace-estimate-label" aria-hidden="true">{t("Race day projection")}</span>}{projection ? <CountUp format={progress => fmtDuration(projection * progress, Number(kilometers) <= 10)} /> : "—"}</td>}</tr>;
    })}</tbody></table></div>
    <InfoDialog title={t("Estimate basis")}>
      {!preview && <><p>{t("Today blends the plan heuristic and latest synced watch prediction equally (50% each), rounded to the nearest second. If one is unavailable or invalid, only the other is used.")}</p><p>{t("Watch predictions reflect the latest sync and may not be from today.")}</p><p>{t("The heuristic uses the saved plan threshold plus accepted pace adjustments ({{value0}}/km), not the latest watch threshold. It does not model your individual marathon endurance and can be optimistic.", { value0: fmtPace(plan.thresholdSecKm + current) })}</p></>}
      <p>{t("Riegel model (exponent 1.06), treating the plan threshold plus accepted quality-pace offset as a one-hour effort pace. Training performance is not a race result; course, endurance, weather and fatigue can change outcomes.")}</p>
      {!preview && <p>{t("Race day is an estimate based on the plan's peak weekly distance and remaining time, not a guarantee. It is shown only for an active race plan with a known starting volume.")}</p>}
    </InfoDialog>
  </section>;
}