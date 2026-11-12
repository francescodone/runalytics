import { t, duration as fmtDuration, date as fmtDateDMY } from "./i18n";
import type { PlanDefaults, PlanWorkout, TrainingPlan } from "../lib/types";
import { dateValue, monday, today } from "./data";
import { InfoDialog } from "./ui";

export function trainingRaceTime(threshold: number, offset: number, kilometers: number): number {
  const effective = threshold + offset;
  return Number.isFinite(effective) && effective > 0 ? Math.round(3600 * Math.pow(kilometers / (3600 / effective), 1.06)) : 0;
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

export function RaceEstimates({ plan, proposedOffset, watch, workouts }: { plan: TrainingPlan; proposedOffset?: number; watch?: Pick<PlanDefaults, "predict5kS" | "predict10kS" | "predictHalfS" | "predictMarathonS"> | null; workouts?: PlanWorkout[] }) {
  const current = plan.qualityOffsetSecKm ?? 0;
  const preview = proposedOffset != null;
  const projectedThreshold = raceDayThreshold(plan, workouts ?? [], today());
  return <section className="race-estimates" aria-label={preview ? t("Proposed race estimates") : t("Training race estimates")}>
    <h3>{preview ? t("Race estimate changes") : t("Training race estimates")}</h3>
    <p className="muted">{plan.qualityOffsetFrom ? t(" / Last adjustment {{value0}}", { value0: fmtDateDMY(plan.qualityOffsetFrom) }).replace(/^\s*\/\s*/, "") : t(" / Plan baseline").replace(/^\s*\/\s*/, "")}</p>
    <div className="data-table-scroll" tabIndex={0} role="region" aria-label={t("Race estimates from the plan threshold and accepted pace adjustment")}><table><caption className="sr-only">{t("Race estimates from the plan threshold and accepted pace adjustment")}</caption><thead><tr><th>{t("Distance")}</th>{!preview && <th>{t("Watch")} · {t("Today")}</th>}<th>{preview ? t("Current") : <>{t("Plan")} · {t("Today")}</>}</th>{preview ? <th>{t("Proposed")}</th> : <th>{t("Race day projection")}</th>}</tr></thead><tbody>{[["5 km",5,watch?.predict5kS,"5K"],["10 km",10,watch?.predict10kS,"10K"],[t("Half marathon"),21.0975,watch?.predictHalfS,"21K"],[t("Marathon"),42.195,watch?.predictMarathonS,"42K"]].map(([label,kilometers,watchSeconds,distance]) => {
      const seconds = trainingRaceTime(plan.thresholdSecKm,current,Number(kilometers));
      const proposed = trainingRaceTime(plan.thresholdSecKm,proposedOffset ?? current,Number(kilometers));
      const projection = projectedThreshold != null && distance === plan.distance ? trainingRaceTime(projectedThreshold, 0, Number(kilometers)) : 0;
      return <tr key={String(label)}><th scope="row">{label}</th>{!preview && <td>{typeof watchSeconds === "number" && Number.isFinite(watchSeconds) && watchSeconds > 0 ? fmtDuration(watchSeconds, Number(kilometers) <= 10) : t("Not available")}</td>}<td data-seconds={seconds}>{seconds ? fmtDuration(seconds, Number(kilometers) <= 10) : t("Not available")}</td>{preview ? <td data-seconds={proposed}>{proposed ? fmtDuration(proposed, Number(kilometers) <= 10) : t("Not available")}</td> : <td data-projection={projection || undefined}>{projection ? fmtDuration(projection, Number(kilometers) <= 10) : "—"}</td>}</tr>;
    })}</tbody></table></div>
    {!preview && <p className="muted">{t("Watch predictions reflect the latest sync and may not be from today.")}</p>}
    <InfoDialog title={t("Estimate basis")}><p>{t("Riegel model (exponent 1.06), treating the plan threshold plus accepted quality-pace offset as a one-hour effort pace. Training performance is not a race result; course, endurance, weather and fatigue can change outcomes.")}</p>{!preview && <p>{t("Race day is an estimate based on the plan's peak weekly distance and remaining time, not a guarantee. It is shown only for an active race plan with a known starting volume.")}</p>}</InfoDialog>
  </section>;
}