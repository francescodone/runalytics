import { useState } from "react";
import { IconArrowUpRight } from "@tabler/icons-react";
import { tauri, fmtDateDMY, fmtKm, fmtPace } from "../lib/ipc";
import { dateValue, navigate, shortDate, today, useRemote } from "./data";
import { Button, ErrorNotice, Loading } from "./ui";
import { PlanAdjustments } from "./proposals";
import { Change, SelectionGroup, SelectionMarker } from "./interaction";

export function PaceInsightsView() {
  const remote = useRemote(tauri.paceInsights);
  const [mode, setMode] = useState<"speed" | "long">("speed");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  if (remote.loading) return <Loading label="Loading pace insights" />;
  if (remote.error) return <ErrorNotice error={remote.error} retry={remote.retry} />;
  const sessions = (remote.data?.[mode] ?? []).filter(session => session.reps.length > 0);
  const next = mode === "speed" ? remote.data?.nextSpeed : remote.data?.nextLong;
  const selected = sessions.find(session => session.workoutId === selectedId) ?? sessions[sessions.length - 1];
  const extent = Math.max(3, ...sessions.flatMap(session => session.reps.map(rep => Math.abs(rep.relativePace))));
  const vertical = (value: number) => 140 - value / extent * 108;
  const current = sessions.filter(session => !session.outdated);
  const onTarget = current.filter(session => session.reps.every(rep => Math.abs(rep.relativePace) <= 1)).length;
  const daysUntil = next ? Math.round((dateValue(next.date).getTime() - dateValue(today()).getTime()) / 86_400_000) : 0;
  return <section className="pace-insights" aria-label="Pace insights">
    <SelectionGroup><div className="pace-mode" role="group" aria-label="Pace workout type">{(["speed", "long"] as const).map(value => <button key={value} type="button" aria-pressed={mode === value} onClick={() => { setMode(value); setSelectedId(null); }}>{mode === value && <SelectionMarker />}{value === "speed" ? "Speed" : "Long"}</button>)}</div></SelectionGroup>
    {next && <div className="pace-next"><div><strong>Next {mode === "speed" ? "speed workout" : "long run"}{daysUntil === 0 ? " today" : ` in ${daysUntil} ${daysUntil === 1 ? "day" : "days"}`}</strong><p>{fmtDateDMY(next.date)} / {next.title}{next.distanceM != null ? ` / ${fmtKm(next.distanceM)}` : ""}</p></div><Button onClick={() => navigate("training", "plan")}><IconArrowUpRight size={18} />View plan</Button></div>}
    <div className="section-heading"><div><h3>Pace on point</h3><p className="muted">{sessions.length ? `${shortDate(sessions[0].date)} - ${shortDate(sessions[sessions.length - 1].date)}` : "No eligible sessions yet"}</p></div>{current.length > 0 && <span className="pace-summary">{onTarget}/{current.length} sessions within target</span>}</div>
    {!remote.data ? <p className="context-note">Create a training plan and link completed runs to see pace insights.</p> : !sessions.length ? <p className="context-note">{mode === "speed" ? "Completed tempo and interval sessions with a synced GPS track and measurable work reps will appear here." : "Completed long runs with explicitly pace-targeted work segments will appear here. Conversational runs are not graded."}</p> : <>
      <figure className="pace-insights-figure">
        <div className="pace-chart-frame"><svg viewBox="0 0 600 280" role="img" aria-label={`${mode === "speed" ? "Speed" : "Long run"} pace insights for ${sessions.length} sessions. Higher means faster relative to each rep's target. Select a dated session below for its values.`}>
          <rect className="pace-point-band" x="32" y={vertical(1)} width="536" height={vertical(-1) - vertical(1)} />
          {[-1, 0, 1].map(value => <line key={value} className="pace-point-grid" x1="32" x2="568" y1={vertical(value)} y2={vertical(value)} />)}
          {sessions.map((session, index) => {
            const values = session.reps.map(rep => rep.relativePace);
            const fastest = Math.max(...values);
            const slowest = Math.min(...values);
            const horizontal = 60 + index * 480 / Math.max(sessions.length - 1, 1) + (sessions.length === 1 ? 240 : 0);
            return <g key={session.workoutId} className={`pace-point-session${session.outdated ? " outdated" : ""}${session.workoutId === selected.workoutId ? " selected" : ""}`} data-workout={session.workoutId}>
              <title>{fmtDateDMY(session.date)}: {session.title}. Fastest relative to target {fastest.toFixed(2)}; slowest {slowest.toFixed(2)}.{session.outdated ? " Outdated." : ""}</title>
              <line className="pace-point-guide" x1={horizontal} x2={horizontal} y1="20" y2="260" />
              <line className="pace-point-range" x1={horizontal} x2={horizontal} y1={vertical(fastest)} y2={vertical(slowest)} />
              <circle cx={horizontal} cy={vertical(fastest)} r="5" />{fastest !== slowest && <circle cx={horizontal} cy={vertical(slowest)} r="5" />}
            </g>;
          })}
        </svg><div className="pace-axis-labels" aria-hidden="true"><span>Faster</span><span>Slower</span></div></div>
        <figcaption><span><i className="pace-band-key" />Pace on point</span><span><i className="pace-rep-key" />Fastest / slowest relative rep</span></figcaption>
      </figure>
      <SelectionGroup><div className="pace-session-picker" role="group" aria-label="Inspect pace session">{sessions.map(session => <button key={session.workoutId} type="button" aria-pressed={selected.workoutId === session.workoutId} aria-label={`${fmtDateDMY(session.date)}: ${session.title}${session.outdated ? ", Outdated" : ""}`} onClick={() => setSelectedId(session.workoutId)}>{selected.workoutId === session.workoutId && <SelectionMarker />}{shortDate(session.date)}{session.outdated && <small>Outdated</small>}</button>)}</div></SelectionGroup>
      <section className="pace-session-detail" aria-label="Selected pace session"><Change identity={selected.workoutId}><div aria-live="polite"><strong>{selected.title}</strong><p className="muted">{fmtDateDMY(selected.date)} / {selected.reps.length} {selected.reps.length === 1 ? "measured rep" : "measured reps"}{selected.outdated ? " / Outdated: excluded from current pace status" : ""}</p></div></Change><Button onClick={() => navigate("journal", String(selected.activityId))}><IconArrowUpRight size={18} />Open activity review</Button>
        <div className="data-table-scroll" tabIndex={0} role="region" aria-label="Pace insight repetitions"><table><caption className="sr-only">Measured work reps relative to their own prescribed target</caption><thead><tr><th>Rep</th><th>Pace /km</th><th>Target /km</th><th>Relative pace</th></tr></thead><tbody>{selected.reps.map(rep => <tr key={rep.index}><th scope="row">{rep.index}</th><td>{fmtPace(rep.actualSKm)}</td><td>{fmtPace(rep.bandLoSKm)} - {fmtPace(rep.bandHiSKm)}</td><td>{rep.relativePace > 1 ? "Faster than target" : rep.relativePace < -1 ? "Slower than target" : "Within target"}</td></tr>)}</tbody></table></div>
      </section>
    </>}
    <details className="pace-method"><summary>About pace insights</summary><p>Up to five recent eligible sessions from your current plan. Rep boundaries are inferred from the planned distances along your GPS track, not imported watch laps. Only work reps with at least 98% of their prescribed distance measured are included; warm-ups, recoveries, missing reps, and invalid GPS measurements are excluded.</p><p>Each rep is compared with its own target midpoint and half-range, so different rep distances and prescribed speeds can be compared. The shaded band marks the prescribed limits; points closer together mean more consistent target-relative pacing. This is Runalytics' transparent calculation, not Runna's proprietary pace-status algorithm.</p><p>Targets are reconstructed from the current plan, not immutable historical snapshots. Speed sessions before the latest accepted pace adjustment are marked outdated and excluded from the current count. Long runs do not trigger quality-pace adjustments.</p></details>
    {mode === "speed" && <div className="review-section"><PlanAdjustments source="pacing" /></div>}
  </section>;
}