import { t, duration as fmtDuration, date as fmtDateDMY } from "./i18n";
import type { TrainingPlan } from "../lib/types";
import {  } from "../lib/ipc";
import { InfoDialog } from "./ui";

export function trainingRaceTime(threshold: number, offset: number, kilometers: number): number {
  const effective = threshold + offset;
  return Number.isFinite(effective) && effective > 0 ? Math.round(3600 * Math.pow(kilometers / (3600 / effective), 1.06)) : 0;
}

export function RaceEstimates({ plan, proposedOffset }: { plan: TrainingPlan; proposedOffset?: number }) {
  const current = plan.qualityOffsetSecKm ?? 0;
  const preview = proposedOffset != null;
  return <section className="race-estimates" aria-label={preview ? t("Proposed race estimates") : t("Training race estimates")}>
    <h3>{preview ? t("Race estimate changes") : t("Training race estimates")}</h3>
    <p className="muted">{t("Plan-derived estimates{{value0}}. Separate from watch predictions.", { value0: plan.qualityOffsetFrom ? t(" / Last adjustment {{value0}}", { value0: fmtDateDMY(plan.qualityOffsetFrom) }) : t(" / Plan baseline") })}</p>
    <table><caption className="sr-only">{t("Race estimates from the plan threshold and accepted pace adjustment")}</caption><thead><tr><th>{t("Distance")}</th><th>{preview ? t("Current") : t("Estimated time")}</th>{preview && <th>{t("Proposed")}</th>}</tr></thead><tbody>{[["5 km",5],["10 km",10],[t("Half marathon"),21.0975],[t("Marathon"),42.195]].map(([label,kilometers]) => {
      const seconds = trainingRaceTime(plan.thresholdSecKm,current,Number(kilometers));
      const proposed = trainingRaceTime(plan.thresholdSecKm,proposedOffset ?? current,Number(kilometers));
      return <tr key={label}><th scope="row">{label}</th><td data-seconds={seconds}>{seconds ? fmtDuration(seconds, Number(kilometers) <= 10) : t("Not available")}</td>{preview && <td data-seconds={proposed}>{proposed ? fmtDuration(proposed, Number(kilometers) <= 10) : t("Not available")}</td>}</tr>;
    })}</tbody></table>
    <InfoDialog title={t("Estimate basis")}><p>{t("Riegel model (exponent 1.06), treating the plan threshold plus accepted quality-pace offset as a one-hour effort pace. Training performance is not a race result; course, endurance, weather and fatigue can change outcomes.")}</p></InfoDialog>
  </section>;
}