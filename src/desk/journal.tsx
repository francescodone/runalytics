import { lazy, Suspense, useDeferredValue, useState } from "react";
import { IconArrowUpRight, IconRun, IconSearch } from "@tabler/icons-react";
import { tauri, fmtDateTime, fmtDuration, fmtKm, fmtPace, ymdLocal } from "../lib/ipc";
import type { Activity, LapSlice, TimeRange } from "../lib/types";
import { navigate, useRemote, useRoute } from "./data";
import { Button, Empty, ErrorNotice, Inspector, Loading, Metric } from "./ui";
import { Change, Presence, SelectionGroup, SelectionMarker } from "./interaction";

const RouteMap = lazy(() => import("./map"));

export function Journal() {
  const route = useRoute();
  const activities = useRemote(tauri.allActivities);
  const scores = useRemote(tauri.runScores);
  const [query, setQuery] = useState("");
  const [sport, setSport] = useState("");
  const [provider, setProvider] = useState("");
  const [from, setFrom] = useState("");
  const [until, setUntil] = useState("");
  const search = useDeferredValue(query.toLowerCase().trim());
  const selected = activities.data?.find(activity => String(activity.id) === route.section);
  const filtered = (activities.data ?? []).filter(activity => {
    const date = ymdLocal(new Date(activity.startTime));
    return (!sport || activity.sport === sport) && (!provider || activity.provider === provider) && (!from || date >= from) && (!until || date <= until) && (!search || `${activity.sport} ${activity.provider} ${fmtDateTime(activity.startTime)} ${fmtKm(activity.distanceM)}`.toLowerCase().includes(search));
  }).sort((first, second) => second.startTime.localeCompare(first.startTime));
  return <>
    <div className="desk-toolbar"><SelectionGroup><nav className="horizon-switch" aria-label="Journal view"><a href="#journal" aria-current={route.section !== "map" ? "page" : undefined}>{route.section !== "map" && <SelectionMarker />}Activities</a><a href="#journal/map" aria-current={route.section === "map" ? "page" : undefined}>{route.section === "map" && <SelectionMarker />}Training map</a></nav></SelectionGroup></div>
    {route.section === "map" ? <TrainingMap /> : <>
    <div className="journal-tools"><label className="search-field"><IconSearch size={18} aria-hidden="true" /><input type="search" aria-label="Search activities" value={query} placeholder="Search your training" onChange={event => setQuery(event.target.value)} /></label><label>Sport<select value={sport} onChange={event => setSport(event.target.value)}><option value="">All sports</option>{[...new Set(activities.data?.map(activity => activity.sport))].map(value => <option key={value}>{value}</option>)}</select></label><label>Source<select value={provider} onChange={event => setProvider(event.target.value)}><option value="">All sources</option><option value="coros">COROS</option><option value="garmin">Garmin</option></select></label><label>From<input type="date" value={from} onChange={event => setFrom(event.target.value)} /></label><label>Until<input type="date" value={until} onChange={event => setUntil(event.target.value)} /></label><Button onClick={() => { setQuery(""); setSport(""); setProvider(""); setFrom(""); setUntil(""); }}>Reset filters</Button></div>
    <ErrorNotice error={activities.error} retry={activities.retry} />
    {activities.loading ? <Loading label="Loading training history" /> : <div className={`journal-workspace ${selected ? "has-inspector" : ""}`}>
      <section className="activity-library" aria-label="Activity history" aria-busy={query.toLowerCase().trim() !== search}><div className="list-heading"><span aria-live="polite">{filtered.length} activities</span><span>Most recent first</span></div>
        <Change identity={`${search}:${sport}:${provider}:${from}:${until}`}>
        {!filtered.length && <Empty title={activities.data?.length ? "No matching activities" : "Your training story starts here"}>{activities.data?.length ? "Try another date or clear the filters." : "Sync your watch to bring your completed activities into the journal."}</Empty>}
        {filtered.map(activity => <button key={activity.id} className="activity-row" aria-pressed={selected?.id === activity.id} onClick={() => navigate("journal", String(activity.id))}><IconRun size={23} aria-hidden="true" /><span><strong>{activity.sport}</strong><small>{fmtDateTime(activity.startTime)}</small></span><span className="activity-distance"><strong>{fmtKm(activity.distanceM)}</strong><small>{fmtDuration(activity.durationS)}</small></span><span className="activity-pace">{activity.avgPaceSKm ? `${fmtPace(activity.avgPaceSKm)}/km` : ""}</span><IconArrowUpRight size={18} aria-hidden="true" /></button>)}
        </Change>
      </section>
      <Presence>{selected && <Inspector key="review" title="Activity review" onClose={() => navigate("journal")}><ActivityReview activity={selected} />{scores.data?.filter(score => score.activityId === selected.id).map(score => <section className="review-section" key={score.activityId}><h3>Recorded session impact</h3><dl className="metrics-line"><Metric label="Training load" value={score.load ?? "Not recorded"} /><Metric label="Risk at completion" value={score.risk} unit="/100" /><Metric label="Risk change" value={`${score.riskDelta > 0 ? "+" : ""}${score.riskDelta.toFixed(1)}`} unit="points" /></dl></section>)}</Inspector>}</Presence>
    </div>}</>}
  </>;
}

function TrainingMap() {
  const data = useRemote(() => Promise.all([tauri.heatPoints(), tauri.getSettings()]));
  return <section className="training-map"><div className="section-heading"><h2>Where you train</h2><span className="muted">All recorded routes</span></div><ErrorNotice error={data.error} retry={data.retry} />{data.loading ? <Loading label="Loading route history" /> : data.data?.[0].length ? <Suspense fallback={<Loading label="Loading training map" />}><RouteMap track={[]} heat={data.data[0]} settings={data.data[1]} /></Suspense> : <Empty title="No routes recorded yet">GPS activities appear here after syncing.</Empty>}</section>;
}

function LapExecution({ laps, prescribed }: { laps: LapSlice[]; prescribed: boolean }) {
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const validPace = (pace: number | null): pace is number => pace != null && Number.isFinite(pace) && pace > 0;
  const targetFor = (lap: LapSlice): [number, number] | null => prescribed && validPace(lap.bandLoSKm) && validPace(lap.bandHiSKm) && lap.bandLoSKm <= lap.bandHiSKm ? [lap.bandLoSKm, lap.bandHiSKm] : null;
  const paces = laps.flatMap(lap => [lap.avgPaceSKm, ...(targetFor(lap) ?? [])]).filter(validPace);
  const fastest = paces.length ? Math.min(...paces) : 300;
  const position = (pace: number) => 88 * fastest / pace;
  const hasTargets = laps.some(lap => targetFor(lap) != null);
  const selected = laps.find(lap => lap.index === selectedIndex) ?? laps.find(lap => targetFor(lap)) ?? laps[0];
  const paceLabel = (lap: LapSlice) => validPace(lap.avgPaceSKm) ? `${fmtPace(lap.avgPaceSKm)}/km` : "Not recorded";
  const targetLabel = (lap: LapSlice) => {
    const target = targetFor(lap);
    return target ? `${fmtPace(target[0])} - ${fmtPace(target[1])}/km` : "Not prescribed";
  };
  const resultLabel = (lap: LapSlice) => !targetFor(lap) || !validPace(lap.avgPaceSKm) ? "Not graded" : lap.verdict === "in" ? "On target" : lap.verdict === "ahead" ? "Faster than target" : lap.verdict === "behind" ? "Slower than target" : "Not graded";
  const description = (lap: LapSlice) => `Lap ${lap.index}: ${lap.label}. ${fmtKm(lap.meters)} in ${fmtDuration(lap.durationS)}. Pace ${paceLabel(lap)}. Target: ${targetLabel(lap)}. ${resultLabel(lap)}.`;
  return <section className="review-section lap-execution"><h3>{prescribed ? "Session execution" : "Recorded laps"}</h3>
    {!laps.length ? <p className="muted">No lap data available.</p> : <>
      {hasTargets && <figure className="lap-pace-chart">
        <figcaption><strong>Lap pace /km</strong><span className="lap-pace-legend"><span><i className="lap-recorded-key" />Recorded</span><span><i className="lap-target-key" />Target range</span></span></figcaption>
        <div className="lap-chart-scroll" tabIndex={0} role="region" aria-label="Lap pace chart">
          <div className="lap-pace-columns" style={{gridTemplateColumns: laps.map(lap => `minmax(44px, ${Number.isFinite(lap.meters) && lap.meters > 0 ? lap.meters : 1}fr)`).join(" ")}}>
            {laps.map(lap => {
              const target = targetFor(lap);
              return <button key={lap.index} type="button" className="lap-pace-column" aria-label={description(lap)} aria-pressed={selected.index === lap.index} title={description(lap)} onClick={() => setSelectedIndex(lap.index)}>
                <span className="lap-pace-plot" aria-hidden="true">
                  {validPace(lap.avgPaceSKm) ? <span className="lap-pace-actual" style={{height:`${position(lap.avgPaceSKm)}%`}}><span>{fmtPace(lap.avgPaceSKm)}</span></span> : <span className="lap-pace-missing">--</span>}
                  {target && <span className={`lap-target-band${lap.verdict === "in" && validPace(lap.avgPaceSKm) ? " on-target" : ""}`} style={{bottom:`${position(target[1])}%`, height:`${position(target[0]) - position(target[1])}%`}} />}
                </span>
                <span className="lap-pace-index" aria-hidden="true">{lap.index}</span>
              </button>;
            })}
          </div>
        </div>
        <p className="lap-pace-selection" aria-live="polite"><strong>Lap {selected.index}: {selected.label}</strong><span>{paceLabel(selected)} / Target: {targetLabel(selected)}</span><span>{resultLabel(selected)}</span></p>
      </figure>}
      <div className="data-table-scroll" tabIndex={0} role="region" aria-label="Recorded laps"><table className={`lap-execution-table${hasTargets ? " has-targets" : ""}`}><caption className="sr-only">Recorded laps and prescribed pace</caption><thead><tr><th>Lap</th><th>Distance</th><th>Time</th><th>Avg. pace{hasTargets && <span className="lap-row-target">Target range</span>}</th>{!hasTargets && <><th>Target</th><th>Result</th></>}</tr></thead><tbody>{laps.map(lap => {
        const target = targetFor(lap);
        return <tr key={lap.index} data-selected={hasTargets && selected.index === lap.index ? "true" : undefined}>
          <th scope="row">{hasTargets ? <button type="button" className="lap-row-button" title={lap.label} aria-label={`Lap ${lap.index}: ${lap.label}`} aria-pressed={selected.index === lap.index} onClick={() => setSelectedIndex(lap.index)}>{lap.index}</button> : <><span className="lap-row-index">{lap.index}</span>{lap.label}</>}</th>
          <td>{fmtKm(lap.meters)}</td><td>{fmtDuration(lap.durationS)}</td>
          <td><span className="lap-row-pace">{paceLabel(lap)}</span>{hasTargets && <>
            <span className="lap-pace-track" aria-hidden="true">{validPace(lap.avgPaceSKm) && <span className="lap-pace-fill" style={{width:`${position(lap.avgPaceSKm)}%`}} />}{target && <span className={`lap-pace-range${lap.verdict === "in" && validPace(lap.avgPaceSKm) ? " on-target" : ""}`} style={{left:`${position(target[1])}%`, width:`${position(target[0]) - position(target[1])}%`}} />}</span>
            <span className="lap-row-target">{targetLabel(lap)}</span><span className="lap-row-result">{resultLabel(lap)}</span>
          </>}</td>
          {!hasTargets && <><td>{targetLabel(lap)}</td><td>{resultLabel(lap)}</td></>}
        </tr>;
      })}</tbody></table></div>
    </>}
  </section>;
}

function ActivityReview({ activity }: { activity: Activity }) {
  const [range, setRange] = useState<TimeRange>("7d");
  const review = useRemote(() => tauri.activityReview(activity.id), [activity.id]);
  const detail = useRemote(() => tauri.activityDetail(activity.id, range), [activity.id, range]);
  const settings = useRemote(tauri.getSettings);
  return <article className="activity-review"><div className="review-heading"><span className="muted">{activity.provider.toUpperCase()} / {fmtDateTime(activity.startTime)}</span><h2>{activity.sport}</h2></div>
    <dl className="metrics-line"><Metric label="Distance" value={(activity.distanceM / 1000).toFixed(2)} unit="km" /><Metric label="Moving time" value={fmtDuration(activity.durationS)} /><Metric label="Average pace" value={fmtPace(activity.avgPaceSKm)} unit="/km" /><Metric label="Heart rate" value={activity.avgHr ?? "Not recorded"} unit={activity.avgHr ? "bpm" : undefined} /><Metric label="Elevation gain" value={activity.elevationGainM ?? "Not recorded"} unit={activity.elevationGainM != null ? "m" : undefined} /></dl>
    <ErrorNotice error={review.error} retry={review.retry} />
    {review.loading ? <Loading label="Loading recorded route and splits" /> : review.data && <>
      {review.data.track.length > 1 && settings.data ? <Suspense fallback={<Loading label="Loading map" />}><RouteMap track={review.data.track} settings={settings.data} /></Suspense> : <p className="context-note">No GPS track was recorded for this activity.</p>}
      <LapExecution key={activity.id} laps={review.data.laps} prescribed={review.data.prescribed} />
      <section className="review-section"><h3>Kilometre splits</h3><div className="data-table-scroll" tabIndex={0} role="region" aria-label="Kilometre splits"><table><caption className="sr-only">Kilometre splits, with pace change compared with the previous kilometre</caption><thead><tr><th>Km</th><th>Pace /km</th><th>vs previous km</th><th>HR</th></tr></thead><tbody>{review.data.splits.map(split => <tr key={split.km}><th scope="row">{split.km}{split.meters < 990 ? ` (${Math.round(split.meters)} m)` : ""}</th><td>{fmtPace(split.avgPaceSKm)}</td><td>{split.deltaPrevSKm == null ? "Not available" : `${split.deltaPrevSKm > 0 ? "+" : ""}${Math.round(split.deltaPrevSKm)} s`}</td><td>{split.avgHr ?? "Not recorded"}</td></tr>)}</tbody></table></div></section>
    </>}
    <section className="review-section"><div className="section-heading"><h3>Training context</h3><label><span className="sr-only">Context range</span><select value={range} onChange={event => setRange(event.target.value as TimeRange)}><option value="1d">Day</option><option value="7d">Week</option><option value="1mo">Month</option></select></label></div><ErrorNotice error={detail.error} retry={detail.retry} />{detail.data && <><dl className="metrics-line"><Metric label="Load ratio" value={detail.data.load?.ratio?.toFixed(2) ?? "Not available"} /><Metric label="Injury risk" value={detail.data.injuryRisk} unit="/100" /><Metric label="VO2 max" value={detail.data.fitness?.vo2max?.toFixed(1) ?? "Not available"} /></dl>{detail.data.riskFactors.map(factor => <p key={factor.label}><strong>{factor.label}</strong><br />{factor.detail}</p>)}</>}</section>
  </article>;
}