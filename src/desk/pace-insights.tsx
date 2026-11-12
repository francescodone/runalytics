import { t, text, number, date as fmtDateDMY, distance as fmtKm } from "./i18n";
import { useState } from "react";
import { IconArrowUpRight } from "@tabler/icons-react";
import { tauri, displayTitle as storedTitle, fmtPace } from "../lib/ipc";
import { dateValue, navigate, shortDate, today, useRemote } from "./data";
import { Button, CountUp, ErrorNotice, InfoDialog, Loading } from "./ui";
import { PlanAdjustments } from "./proposals";
import { RaceEstimates } from "./race-estimates";
import { Change, SelectionGroup, SelectionMarker } from "./interaction";

export function PaceInsightsView() {
  const plan = useRemote(tauri.currentPlan);
  const watch = useRemote(tauri.planDefaults);
  return <>
    <ErrorNotice error={plan.error} retry={plan.retry} />
    {plan.loading ? <Loading /> : plan.data?.[0] ? <RaceEstimates plan={plan.data[0]} workouts={plan.data[1]} watch={watch.data} /> : !plan.error && <p className="context-note">{t("Create a training plan and link completed runs to see pace insights.")}</p>}
    <PaceHistory />
  </>;
}

function PaceHistory() {
  const remote = useRemote(tauri.paceInsights);
  const [mode, setMode] = useState<"speed" | "long">("speed");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  if (remote.loading) return <Loading label={t("Loading pace insights")} />;
  if (remote.error) return <ErrorNotice error={remote.error} retry={remote.retry} />;
  const sessions = (remote.data?.[mode] ?? [])
    .filter(session => session.reps.length > 0)
    .sort((previous, next) => previous.date.localeCompare(next.date))
    .slice(-5);
  const next = mode === "speed" ? remote.data?.nextSpeed : remote.data?.nextLong;
  const selected = sessions.find(session => session.workoutId === selectedId) ?? sessions[sessions.length - 1];
  const extent = Math.max(3, ...sessions.flatMap(session => session.reps.map(rep => Math.abs(rep.relativePace))));
  const vertical = (value: number) => 140 - value / extent * 108;
  const current = sessions.filter(session => !session.outdated);
  const onTarget = current.filter(session => session.reps.every(rep => Math.abs(rep.relativePace) <= 1)).length;
  const daysUntil = next ? Math.round((dateValue(next.date).getTime() - dateValue(today()).getTime()) / 86_400_000) : 0;
  return <section className="pace-insights" aria-label={t("Pace insights")}>
    <SelectionGroup><div className="pace-mode" role="group" aria-label={t("Pace workout type")}>{(["speed", "long"] as const).map(value => <button key={value} type="button" aria-pressed={mode === value} onClick={() => { setMode(value); setSelectedId(null); }}>{mode === value && <SelectionMarker />}{value === "speed" ? t("Speed") : t("Long")}</button>)}</div></SelectionGroup>
    {next && <div className="pace-next"><div><strong>{daysUntil === 0 ? t(mode === "speed" ? "Next speed workout today" : "Next long run today") : t(mode === "speed" ? "nextSpeed" : "nextLong", { count: daysUntil })}</strong><p>{fmtDateDMY(next.date)} / {displayTitle(next.title)}{next.distanceM != null ? ` / ${fmtKm(next.distanceM)}` : ""}</p></div><Button onClick={() => navigate("training", "plan")}><IconArrowUpRight size={18} />{t("View plan")}</Button></div>}
    <div className="section-heading pace-history-heading"><div><h3>{t("Pace on point")}</h3><p className="muted">{sessions.length ? `${shortDate(sessions[0].date)} - ${shortDate(sessions[sessions.length - 1].date)}` : t("No eligible sessions yet")}</p></div>{current.length > 0 && <span className="pace-summary"><CountUp format={progress => t("{{value0}}/{{value1}} sessions within target", { value0: number(onTarget * progress), value1: current.length })} /></span>}</div>
    {!remote.data ? <p className="context-note">{t("Create a training plan and link completed runs to see pace insights.")}</p> : !sessions.length ? <p className="context-note">{mode === "speed" ? t("Completed tempo and interval sessions with a synced GPS track and measurable work reps will appear here.") : t("Completed long runs with explicitly pace-targeted work segments will appear here. Conversational runs are not graded.")}</p> : <>
      <figure className="pace-insights-figure">
        <div className="pace-chart-frame"><svg viewBox="0 0 600 280" role="img" aria-label={t("{{value0}} pace insights for {{value1}} sessions. Higher means faster relative to each rep's target. Select a dated session below for its values.", { value0: mode === "speed" ? t("Speed") : t("Long run"), value1: sessions.length })}>
          <rect className="pace-point-band" x="32" y={vertical(1)} width="536" height={vertical(-1) - vertical(1)} />
          {[-1, 0, 1].map(value => <line key={value} className="pace-point-grid" x1="32" x2="568" y1={vertical(value)} y2={vertical(value)} />)}
          {sessions.map((session, index) => {
            const values = session.reps.map(rep => rep.relativePace);
            const fastest = Math.max(...values);
            const slowest = Math.min(...values);
            const horizontal = 60 + index * 480 / Math.max(sessions.length - 1, 1) + (sessions.length === 1 ? 240 : 0);
            return <g key={session.workoutId} className={`pace-point-session${session.outdated ? " outdated" : ""}${session.workoutId === selected.workoutId ? " selected" : ""}`} data-workout={session.workoutId}>
              <title>{t("{{value0}}: {{value1}}. Fastest relative to target {{value2}}; slowest {{value3}}.{{value4}}", { value0: fmtDateDMY(session.date), value1: displayTitle(session.title), value2: number(fastest, 2), value3: number(slowest, 2), value4: session.outdated ? t(" Outdated.") : "" })}</title>
              <line className="pace-point-guide" x1={horizontal} x2={horizontal} y1="20" y2="260" />
              <line className="pace-point-range" x1={horizontal} x2={horizontal} y1={vertical(fastest)} y2={vertical(slowest)} />
              <circle cx={horizontal} cy={vertical(fastest)} r="5" />{fastest !== slowest && <circle cx={horizontal} cy={vertical(slowest)} r="5" />}
            </g>;
          })}
        </svg><div className="pace-axis-labels" aria-hidden="true"><span>{t("Faster")}</span><span>{t("Slower")}</span></div></div>
        <figcaption><span><i className="pace-band-key" />{t("Pace on point")}</span><span><i className="pace-rep-key" />{t("Fastest / slowest relative rep")}</span></figcaption>
      </figure>
      <SelectionGroup><div className="pace-session-picker" role="group" aria-label={t("Inspect pace session")}>{sessions.map(session => <button key={session.workoutId} type="button" aria-pressed={selected.workoutId === session.workoutId} aria-label={`${fmtDateDMY(session.date)}: ${displayTitle(session.title)}${session.outdated ? t(", Outdated") : ""}`} onClick={() => setSelectedId(session.workoutId)}>{selected.workoutId === session.workoutId && <SelectionMarker />}{shortDate(session.date)}{session.outdated && <small>{t("Outdated")}</small>}</button>)}</div></SelectionGroup>
      <section className="pace-session-detail" aria-label={t("Selected pace session")}><header><Change identity={selected.workoutId}><div aria-live="polite"><h3>{displayTitle(selected.title)}</h3><p className="muted">{fmtDateDMY(selected.date)} / {selected.reps.length} {selected.reps.length === 1 ? t("measured rep") : t("measured reps")}{selected.outdated ? t(" / Outdated: excluded from current pace status") : ""}</p></div></Change><Button onClick={() => navigate("journal", String(selected.activityId))}><IconArrowUpRight size={18} />{t("Open activity review")}</Button></header>
        <div className="data-table-scroll" tabIndex={0} role="region" aria-label={t("Pace insight repetitions")}><table><caption className="sr-only">{t("Measured work reps relative to their own prescribed target")}</caption><thead><tr><th>{t("Rep")}</th><th>{t("Pace /km")}</th><th>{t("Target /km")}</th><th>{t("Relative pace")}</th></tr></thead><tbody>{selected.reps.map(rep => <tr key={rep.index}><th scope="row">{rep.index}</th><td>{fmtPace(rep.actualSKm)}</td><td>{fmtPace(rep.bandLoSKm)} - {fmtPace(rep.bandHiSKm)}</td><td>{rep.relativePace > 1 ? t("Faster than target") : rep.relativePace < -1 ? t("Slower than target") : t("Within target")}</td></tr>)}</tbody></table></div>
      </section>
    </>}
    <InfoDialog title={t("About pace insights")} className="pace-method"><p>{t("Up to five recent eligible sessions from your current plan. Rep boundaries are inferred from the planned distances along your GPS track, not imported watch laps. Only work reps with at least 98% of their prescribed distance measured are included; warm-ups, recoveries, missing reps, and invalid GPS measurements are excluded.")}</p><p>{t("Each rep is compared with its own target midpoint and half-range, so different rep distances and prescribed speeds can be compared. The shaded band marks the prescribed limits; points closer together mean more consistent target-relative pacing. This is Runalytics' transparent calculation, not Runna's proprietary pace-status algorithm.")}</p><p>{t("Targets are reconstructed from the current plan, not immutable historical snapshots. Speed sessions before the latest accepted pace adjustment are marked outdated and excluded from the current count. Long runs do not trigger quality-pace adjustments.")}</p></InfoDialog>
    {mode === "speed" && <PlanAdjustments source="pacing" />}
  </section>;
}
const displayTitle = (value: string) => text(storedTitle(value));
