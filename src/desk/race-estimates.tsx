import type { TrainingPlan } from "../lib/types";
import { fmtDuration, fmtDateDMY } from "../lib/ipc";
import { InfoDialog } from "./ui";

export function trainingRaceTime(threshold: number, offset: number, kilometers: number): number {
  const effective = threshold + offset;
  return Number.isFinite(effective) && effective > 0 ? Math.round(3600 * Math.pow(kilometers / (3600 / effective), 1.06)) : 0;
}

export function RaceEstimates({ plan, proposedOffset }: { plan: TrainingPlan; proposedOffset?: number }) {
  const current = plan.qualityOffsetSecKm ?? 0;
  const preview = proposedOffset != null;
  return <section className="race-estimates" aria-label={preview ? "Proposed race estimates" : "Training race estimates"}>
    <h3>{preview ? "Race estimate changes" : "Training race estimates"}</h3>
    <p className="muted">Plan-derived estimates{plan.qualityOffsetFrom ? ` / Last adjustment ${fmtDateDMY(plan.qualityOffsetFrom)}` : " / Plan baseline"}. Separate from watch predictions.</p>
    <table><caption className="sr-only">Race estimates from the plan threshold and accepted pace adjustment</caption><thead><tr><th>Distance</th><th>{preview ? "Current" : "Estimated time"}</th>{preview && <th>Proposed</th>}</tr></thead><tbody>{[["5 km",5],["10 km",10],["Half marathon",21.0975],["Marathon",42.195]].map(([label,kilometers]) => {
      const seconds = trainingRaceTime(plan.thresholdSecKm,current,Number(kilometers));
      const proposed = trainingRaceTime(plan.thresholdSecKm,proposedOffset ?? current,Number(kilometers));
      return <tr key={label}><th scope="row">{label}</th><td data-seconds={seconds}>{seconds ? fmtDuration(seconds, Number(kilometers) <= 10) : "Not available"}</td>{preview && <td data-seconds={proposed}>{proposed ? fmtDuration(proposed, Number(kilometers) <= 10) : "Not available"}</td>}</tr>;
    })}</tbody></table>
    <InfoDialog title="Estimate basis"><p>Riegel model (exponent 1.06), treating the plan threshold plus accepted quality-pace offset as a one-hour effort pace. Training performance is not a race result; course, endurance, weather and fatigue can change outcomes.</p></InfoDialog>
  </section>;
}