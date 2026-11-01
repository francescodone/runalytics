import { Fragment, lazy, Suspense, useDeferredValue, useState, type KeyboardEvent, type ReactNode } from "react";
import { IconArrowUpRight, IconBarbell, IconBike, IconRun, IconSearch } from "@tabler/icons-react";
import { tauri, fmtDateTime, fmtDuration, fmtKm, fmtPace, ymdLocal } from "../lib/ipc";
import type { Activity, LapSlice, RunScore, TimeRange } from "../lib/types";
import { isBikeSport, isReadableSport, isStrengthSport } from "../lib/sport";
import { navigate, useRemote, useRoute } from "./data";
import { Button, Empty, ErrorNotice, Inspector, Loading, Metric } from "./ui";
import { Change, Presence, SelectionGroup, SelectionMarker } from "./interaction";

const RouteMap = lazy(() => import("./map"));

/// The four faces of a recorded session. Stacked one under the other they made
/// the review a 4 000 px scroll in which the map, the laps and the splits could
/// never be on screen together, so each now owns a tab and the route lives in
/// the third hash segment (#journal/1/splits) the way the run itself does.
const REVIEW_TABS = [["overview", "Overview"], ["execution", "Session execution"], ["splits", "Splits"], ["context", "Training context"]] as const;
type ReviewTab = (typeof REVIEW_TABS)[number][0];
const isReviewTab = (value: string): value is ReviewTab => REVIEW_TABS.some(([id]) => id === value);

/// Buttons rather than links: switching a tab re-reads the same record, it does
/// not navigate anywhere a runner would want to copy or open in a new tab. The
/// hash still carries the choice so a review survives a reload and a back.
function ReviewTabs({ activityId, tab, onSelect }: { activityId: number; tab: ReviewTab; onSelect: (tab: ReviewTab) => void }) {
  const select = (index: number) => {
    const [id] = REVIEW_TABS[(index + REVIEW_TABS.length) % REVIEW_TABS.length];
    onSelect(id);
    document.getElementById(`review-tab-${activityId}-${id}`)?.focus();
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const index = REVIEW_TABS.findIndex(([id]) => id === tab);
    if (event.key === "ArrowRight" || event.key === "ArrowDown") { event.preventDefault(); select(index + 1); }
    else if (event.key === "ArrowLeft" || event.key === "ArrowUp") { event.preventDefault(); select(index - 1); }
    else if (event.key === "Home") { event.preventDefault(); select(0); }
    else if (event.key === "End") { event.preventDefault(); select(REVIEW_TABS.length - 1); }
  };
  return <SelectionGroup><div className="review-tabs" role="tablist" aria-label="Activity detail" onKeyDown={onKeyDown}>
    {REVIEW_TABS.map(([id, label]) => <button key={id} id={`review-tab-${activityId}-${id}`} type="button" role="tab" aria-selected={tab === id} aria-controls={`review-panel-${activityId}-${id}`} tabIndex={tab === id ? 0 : -1} onClick={() => onSelect(id)}>{tab === id && <SelectionMarker />}{label}</button>)}
  </div></SelectionGroup>;
}

function ReviewPanel({ activityId, tab, children }: { activityId: number; tab: ReviewTab; children: ReactNode }) {
  return <div role="tabpanel" id={`review-panel-${activityId}-${tab}`} className="review-panel" aria-labelledby={`review-tab-${activityId}-${tab}`}>{children}</div>;
}

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
  // An unknown or missing third segment is the overview: the review always has
  // exactly one tab open, and the plain #journal/1 link stays the shortest one.
  const tab: ReviewTab = isReviewTab(route.detail) ? route.detail : "overview";
  // A sport label with no letters in it is a raw provider code (COROS answers
  // some record lists with "1"/"2"/"10"); it names no sport, so it is not a
  // journal entry.
  const filtered = (activities.data ?? []).filter(activity => isReadableSport(activity.sport)).filter(activity => {
    const date = ymdLocal(new Date(activity.startTime));
    return (!sport || activity.sport === sport) && (!provider || activity.provider === provider) && (!from || date >= from) && (!until || date <= until) && (!search || `${activity.sport} ${activity.provider} ${fmtDateTime(activity.startTime)} ${fmtKm(activity.distanceM)}`.toLowerCase().includes(search));
  }).sort((first, second) => second.startTime.localeCompare(first.startTime));
  return <>
    <div className="desk-toolbar"><SelectionGroup><nav className="horizon-switch" aria-label="Journal view"><a href="#journal" aria-current={route.section !== "map" ? "page" : undefined}>{route.section !== "map" && <SelectionMarker />}Activities</a><a href="#journal/map" aria-current={route.section === "map" ? "page" : undefined}>{route.section === "map" && <SelectionMarker />}Training map</a></nav></SelectionGroup></div>
    {route.section === "map" ? <TrainingMap /> : <>
    <div className="journal-tools"><label className="search-field"><IconSearch size={18} aria-hidden="true" /><input type="search" aria-label="Search activities" value={query} placeholder="Search your training" onChange={event => setQuery(event.target.value)} /></label><label>Sport<select value={sport} onChange={event => setSport(event.target.value)}><option value="">All sports</option>{[...new Set((activities.data ?? []).filter(activity => isReadableSport(activity.sport)).map(activity => activity.sport))].map(value => <option key={value}>{value}</option>)}</select></label><label>Source<select value={provider} onChange={event => setProvider(event.target.value)}><option value="">All sources</option><option value="coros">COROS</option><option value="garmin">Garmin</option></select></label><label>From<input type="date" value={from} onChange={event => setFrom(event.target.value)} /></label><label>Until<input type="date" value={until} onChange={event => setUntil(event.target.value)} /></label><Button onClick={() => { setQuery(""); setSport(""); setProvider(""); setFrom(""); setUntil(""); }}>Reset filters</Button></div>
    <ErrorNotice error={activities.error} retry={activities.retry} />
    {activities.loading ? <Loading label="Loading training history" /> : <div className={`journal-workspace ${selected ? "has-inspector" : ""}`}>
      <section className="activity-library" aria-label="Activity history" aria-busy={query.toLowerCase().trim() !== search}><div className="list-heading"><span aria-live="polite">{filtered.length} activities</span><span>Most recent first</span></div>
        <Change identity={`${search}:${sport}:${provider}:${from}:${until}`}>
        {!filtered.length && <Empty title={activities.data?.length ? "No matching activities" : "Your training story starts here"}>{activities.data?.length ? "Try another date or clear the filters." : "Sync your watch to bring your completed activities into the journal."}</Empty>}
        {filtered.map(activity => <button key={activity.id} className="activity-row" aria-pressed={selected?.id === activity.id} onClick={() => navigate("journal", String(activity.id))}>{isStrengthSport(activity.sport) ? <IconBarbell size={23} aria-hidden="true" /> : isBikeSport(activity.sport) ? <IconBike size={23} aria-hidden="true" /> : <IconRun size={23} aria-hidden="true" />}<span><strong>{activity.sport}</strong><small>{fmtDateTime(activity.startTime)}</small></span><span className="activity-distance"><strong>{fmtKm(activity.distanceM)}</strong><small>{fmtDuration(activity.durationS)}</small></span><span className="activity-pace">{activity.avgPaceSKm ? `${fmtPace(activity.avgPaceSKm)}/km` : ""}</span><IconArrowUpRight size={18} aria-hidden="true" /></button>)}
        </Change>
      </section>
      <Presence>{selected && <Inspector key="review" title="Activity review" onClose={() => navigate("journal")}><ActivityReview activity={selected} score={scores.data?.find(score => score.activityId === selected.id)} tab={tab} onTab={next => navigate("journal", String(selected.id), next === "overview" ? "" : next)} /></Inspector>}</Presence>
    </div>}</>}
  </>;
}

function TrainingMap() {
  const data = useRemote(() => Promise.all([tauri.heatPoints(), tauri.getSettings()]));
  return <section className="training-map"><div className="section-heading"><h2>Where you train</h2><span className="muted">All recorded routes</span></div><ErrorNotice error={data.error} retry={data.retry} />{data.loading ? <Loading label="Loading route history" /> : data.data?.[0].length ? <Suspense fallback={<Loading label="Loading training map" />}><RouteMap track={[]} heat={data.data[0]} settings={data.data[1]} /></Suspense> : <Empty title="No routes recorded yet">GPS activities appear here after syncing.</Empty>}</section>;
}

/// Below this width a lap column is too thin to aim at, so the chart is widened
/// and scrolled instead of squeezing it further.
const MIN_LAP_COLUMN_PX = 20;
/// How far the chart is allowed to grow past its own panel to keep the shortest
/// lap clickable — beyond this a sliver is the honest answer.
const LAP_COLUMN_ZOOM_CAP = 1200;

function LapExecution({ laps, prescribed }: { laps: LapSlice[]; prescribed: boolean }) {
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const validPace = (pace: number | null): pace is number => pace != null && Number.isFinite(pace) && pace > 0;
  const targetFor = (lap: LapSlice): [number, number] | null => prescribed && validPace(lap.bandLoSKm) && validPace(lap.bandHiSKm) && lap.bandLoSKm <= lap.bandHiSKm ? [lap.bandLoSKm, lap.bandHiSKm] : null;
  const paces = laps.flatMap(lap => [lap.avgPaceSKm, ...(targetFor(lap) ?? [])]).filter(validPace);
  // The pace axis is linear over everything the chart has to draw, padded at
  // both ends so no bar sits exactly on an edge. Scaling by 1/pace instead
  // crammed every bar into the top third of the plot, which left a 10 s/km
  // target range roughly five pixels tall and made four very different laps
  // look like four identical bars.
  const paceFloor = paces.length ? Math.min(...paces) : 0;
  const paceCeil = paces.length ? Math.max(...paces) : 1;
  const pad = Math.max((paceCeil - paceFloor) * 0.15, 15);
  const position = (pace: number) => {
    const span = paceCeil + pad - (paceFloor - pad);
    return span <= 0 ? 50 : ((paceCeil + pad - pace) / span) * 100;
  };
  // Each column is as wide as its lap is long, measured against the whole
  // session, so the shape of the run — long warm-up, short reps, long cool-down
  // — is readable in the columns and not only in their labels. When the shortest
  // lap would fall below a clickable width the grid is widened and the region
  // scrolled: clamping single columns would break the proportion silently,
  // drawing a 200 m recovery as though it were a kilometre.
  const weights = laps.map(lap => Number.isFinite(lap.meters) && lap.meters > 0 ? lap.meters : 1);
  const totalWeight = weights.reduce((sum, weight) => sum + weight, 0);
  const minWeight = weights.length ? Math.min(...weights) : 1;
  const columnGaps = 4 * Math.max(0, laps.length - 1);
  const columnsMinWidth = Math.min(LAP_COLUMN_ZOOM_CAP, Math.ceil((MIN_LAP_COLUMN_PX * totalWeight) / minWeight) + columnGaps);
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
          <div className="lap-pace-columns" style={{gridTemplateColumns: weights.map(weight => `${weight}fr`).join(" "), minWidth: `${columnsMinWidth}px`}}>
            {laps.map(lap => {
              const target = targetFor(lap);
              // A pace caption needs about this much room before it collides with
              // its neighbour; a proportionally short lap keeps its bar and loses
              // the text, which the selection line and the table still say.
              const columnPx = (Number.isFinite(lap.meters) && lap.meters > 0 ? lap.meters : 1) / totalWeight * (columnsMinWidth - columnGaps);
              return <button key={lap.index} type="button" className={`lap-pace-column${columnPx < 46 ? " lap-col-narrow" : ""}`} aria-label={description(lap)} aria-pressed={selected.index === lap.index} title={description(lap)} onClick={() => setSelectedIndex(lap.index)}>
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
      <div className="data-table-scroll" tabIndex={0} role="region" aria-label="Recorded laps"><table className={`lap-execution-table${hasTargets ? " has-targets" : ""}`}><caption className="sr-only">Recorded laps and prescribed pace</caption><thead>{hasTargets ? <tr><th>Lap</th><th>Distance</th><th>Time</th><th>Pace</th></tr> : <tr><th>Lap</th><th>Distance</th><th>Time</th><th>Avg. pace</th><th>Target</th><th>Result</th></tr>}</thead><tbody>{laps.map(lap => {
        // A graded lap gets a row for what it ran and a row for what was asked,
        // so the two are never stacked inside one cell; an ungraded lap has no
        // range to give a second row, so it stays a single row.
        if (hasTargets) {
          const isSelected = selected.index === lap.index;
          return <Fragment key={lap.index}>
            <tr className="lap-avg-row" data-selected={isSelected ? "true" : undefined}>
              <th scope="row" rowSpan={2}><button type="button" className="lap-row-button" title={lap.label} aria-label={`Lap ${lap.index}: ${lap.label}`} aria-pressed={isSelected} onClick={() => setSelectedIndex(lap.index)}>{lap.index}</button></th>
              <td rowSpan={2}>{fmtKm(lap.meters)}</td><td rowSpan={2}>{fmtDuration(lap.durationS)}</td>
              <td><span className="lap-row-label">Avg</span><span className="lap-row-pace">{paceLabel(lap)}</span></td>
            </tr>
            <tr className="lap-range-row" data-selected={isSelected ? "true" : undefined}>
              <td><span className="lap-row-label">Target</span><span className={`lap-row-target${lap.verdict === "in" && validPace(lap.avgPaceSKm) ? " on-target" : ""}`}>{targetLabel(lap)}</span><span className="lap-row-result">{resultLabel(lap)}</span></td>
            </tr>
          </Fragment>;
        }
        return <tr key={lap.index}>
          <th scope="row"><span className="lap-row-index">{lap.index}</span>{lap.label}</th>
          <td>{fmtKm(lap.meters)}</td><td>{fmtDuration(lap.durationS)}</td>
          <td><span className="lap-row-pace">{paceLabel(lap)}</span></td>
          <td>{targetLabel(lap)}</td><td>{resultLabel(lap)}</td>
        </tr>;
      })}</tbody></table></div>
    </>}
  </section>;
}

function ActivityReview({ activity, score, tab, onTab }: { activity: Activity; score?: RunScore; tab: ReviewTab; onTab: (tab: ReviewTab) => void }) {
  const [range, setRange] = useState<TimeRange>("7d");
  const review = useRemote(() => tauri.activityReview(activity.id), [activity.id]);
  const detail = useRemote(() => tauri.activityDetail(activity.id, range), [activity.id, range]);
  const settings = useRemote(tauri.getSettings);
  return <article className="activity-review"><div className="review-heading"><span className="muted">{activity.provider.toUpperCase()} / {fmtDateTime(activity.startTime)}</span><h2>{activity.sport}</h2></div>
    <ReviewTabs activityId={activity.id} tab={tab} onSelect={onTab} />
    <ErrorNotice error={review.error} retry={review.retry} />
    {review.loading ? <Loading label="Loading recorded route and splits" /> : review.data && <Change identity={tab}>
      {tab === "overview" && <ReviewPanel activityId={activity.id} tab="overview">
        <dl className="metrics-line"><Metric label="Distance" value={(activity.distanceM / 1000).toFixed(2)} unit="km" /><Metric label="Moving time" value={fmtDuration(activity.durationS)} /><Metric label="Average pace" value={fmtPace(activity.avgPaceSKm)} unit="/km" /><Metric label="Heart rate" value={activity.avgHr ?? "Not recorded"} unit={activity.avgHr ? "bpm" : undefined} /><Metric label="Elevation gain" value={activity.elevationGainM ?? "Not recorded"} unit={activity.elevationGainM != null ? "m" : undefined} /></dl>
        {review.data.track.length > 1 && settings.data ? <Suspense fallback={<Loading label="Loading map" />}><RouteMap track={review.data.track} settings={settings.data} /></Suspense> : <p className="context-note">No GPS track was recorded for this activity.</p>}
        {score && <section className="review-section"><h3>Recorded session impact</h3><dl className="metrics-line"><Metric label="Training load" value={score.load ?? "Not recorded"} /><Metric label="Risk at completion" value={score.risk} unit="/100" /><Metric label="Risk change" value={`${score.riskDelta > 0 ? "+" : ""}${score.riskDelta.toFixed(1)}`} unit="points" /></dl></section>}
      </ReviewPanel>}
      {tab === "execution" && <ReviewPanel activityId={activity.id} tab="execution"><LapExecution key={activity.id} laps={review.data.laps} prescribed={review.data.prescribed} /></ReviewPanel>}
      {tab === "splits" && <ReviewPanel activityId={activity.id} tab="splits"><section className="review-section"><h3>Kilometre splits</h3><div className="data-table-scroll" tabIndex={0} role="region" aria-label="Kilometre splits"><table><caption className="sr-only">Kilometre splits, with pace change compared with the previous kilometre</caption><thead><tr><th>Km</th><th>Pace /km</th><th>vs previous km</th><th>HR</th></tr></thead><tbody>{review.data.splits.map(split => <tr key={split.km}><th scope="row">{split.km}{split.meters < 990 ? ` (${Math.round(split.meters)} m)` : ""}</th><td>{fmtPace(split.avgPaceSKm)}</td><td>{split.deltaPrevSKm == null ? "Not available" : `${split.deltaPrevSKm > 0 ? "+" : ""}${Math.round(split.deltaPrevSKm)} s`}</td><td>{split.avgHr ?? "Not recorded"}</td></tr>)}</tbody></table></div></section></ReviewPanel>}
      {tab === "context" && <ReviewPanel activityId={activity.id} tab="context"><section className="review-section"><div className="section-heading"><h3>Training context</h3><label><span className="sr-only">Context range</span><select value={range} onChange={event => setRange(event.target.value as TimeRange)}><option value="1d">Day</option><option value="7d">Week</option><option value="1mo">Month</option></select></label></div><ErrorNotice error={detail.error} retry={detail.retry} />{detail.data && <><dl className="metrics-line"><Metric label="Load ratio" value={detail.data.load?.ratio?.toFixed(2) ?? "Not available"} /><Metric label="Injury risk" value={detail.data.injuryRisk} unit="/100" /><Metric label="VO2 max" value={detail.data.fitness?.vo2max?.toFixed(1) ?? "Not available"} /></dl>{detail.data.riskFactors.map(factor => <p key={factor.label}><strong>{factor.label}</strong><br />{factor.detail}</p>)}</>}</section></ReviewPanel>}
    </Change>}
  </article>;
}