import { useId, useState } from "react";
import { IconPlus, IconTrophy, IconCheck, IconMessage, IconTrash, IconChevronDown, IconAdjustments } from "@tabler/icons-react";
import { tauri, fmtDateDMY, fmtDuration, fmtPace } from "../lib/ipc";
import { gamificationStats } from "../lib/flow";
import { BODY_REGIONS } from "../lib/body-regions";
import type { PainEntry, TimeRange } from "../lib/types";
import { dateValue, navigate, shiftDate, shortDate, today, useCommand, useRemote, useRoute } from "./data";
import { Button, Dialog, Empty, ErrorNotice, InfoDialog, Loading, Metric } from "./ui";
import { PlanAdjustments } from "./proposals";
import { PaceInsightsView } from "./pace-insights";
import { RaceEstimates } from "./race-estimates";
import { Change, Disclosure, Presence, SelectionGroup, SelectionMarker } from "./interaction";

export function Health() {
  const route = useRoute();
  const [range, setRange] = useState<TimeRange>("7d");
  const [logging, setLogging] = useState(false);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const dashboard = useRemote(() => tauri.dashboard(range), [range]);
  const currentPlan = useRemote(tauri.currentPlan);
  const mode = route.section === "pace" ? "pace" : route.section === "milestones" ? "milestones" : route.section === "journal" ? "journal" : "overview";
  const data = dashboard.data;
  const readiness = (data?.scoreHistory ?? []).map((point, index) => ({ date: point.date, value: index === data!.scoreHistory.length - 1 ? data!.scores.globalScore : point.readiness ?? Math.round(point.training * .45 + point.recovery * .35 + (100 - point.risk) * .2) })).filter(point => Number.isFinite(point.value)).sort((first, second) => first.date.localeCompare(second.date));
  const readinessChange = readiness.length > 1 && readiness[0].date !== readiness[readiness.length - 1].date ? Math.round(readiness[readiness.length - 1].value - readiness[0].value) : null;
  return <>
    <div className="training-context"><div><span className="muted">Training and recovery</span><h2>{mode === "pace" ? "Pace insights" : mode === "milestones" ? "The work adds up" : mode === "journal" ? "Listen to your body" : "Your training, in balance"}</h2></div></div>
    <div className="desk-toolbar health-toolbar"><SelectionGroup><nav className="horizon-switch" aria-label="Health view">{[["overview", "Overview"], ["pace", "Pace insights"], ["journal", "Pain journal"], ["milestones", "Milestones"]].map(([value, label]) => <a key={value} href={`#health/${value}`} aria-current={value === mode ? "page" : undefined}>{value === mode && <SelectionMarker />}{label}</a>)}</nav></SelectionGroup>{mode === "overview" && <label><span className="sr-only">Health period</span><select value={range} onChange={event => setRange(event.target.value as TimeRange)}><option value="1d">Today</option><option value="7d">7 days</option><option value="1mo">30 days</option></select></label>}</div>
    <Change identity={mode}>
    {mode === "pace" ? <PaceInsightsView /> : mode === "milestones" ? <Milestones /> : mode === "journal" ? <PainJournal onLog={() => setLogging(true)} /> : <>
      <ErrorNotice error={dashboard.error} retry={dashboard.retry} />{dashboard.loading ? <Loading label="Loading recovery and fitness" /> : data && <>
        <section className="health-verdict"><div><span className="muted">Training status</span><h2>{data.trainingStatus?.label ?? "Building your baseline"}</h2><p>{data.trainingStatus?.reason ?? "More synced training history is needed for a reliable assessment."}</p>{data.loadRecommendation && <p>{data.loadRecommendation.note}</p>}</div><dl className="metrics-line"><Metric label="Readiness" value={data.scores.globalScore} unit="%" /><Metric label="Recovery" value={data.recoveryBand?.label ?? `${data.scores.recoveryScore}%`} /><Metric label="Injury risk" value={data.scores.injuryRisk} unit="%" /></dl></section>
        {data.activePain && <div className="proposal-prompt"><div><strong>{BODY_REGIONS[data.activePain.bodyPart] ?? data.activePain.bodyPart}: {data.activePain.intensity}/10 pain</strong><p>Unresolved since {fmtDateDMY(data.activePain.startedOn)}.</p></div><Button onClick={() => navigate("health", "journal")}>Review episode</Button></div>}
        <section className="health-summary" aria-label="Readiness trend"><div className="section-heading"><h3>{readinessChange == null ? "Building your readiness history" : readinessChange > 0 ? "Readiness is improving" : readinessChange < 0 ? "Readiness is declining" : "Readiness is steady"}</h3><span className="muted">{readinessChange == null ? "Not enough history" : `${readinessChange > 0 ? "+" : ""}${readinessChange} points this period`}</span></div><TrendChart label="Readiness" points={readiness} max={100} percent /><p className="context-note">Training indicators are not a medical diagnosis. Persistent or severe pain should be assessed by a qualified clinician.</p></section>
        <Button className="health-detail-toggle" aria-expanded={detailsOpen} aria-controls="health-details" onClick={() => setDetailsOpen(!detailsOpen)}>{detailsOpen ? "Hide health details" : "Show health details"}<IconChevronDown className="disclosure-indicator" size={18} aria-hidden="true" /></Button>
        <Disclosure id="health-details" open={detailsOpen}>
        <div className="health-analysis"><section className="health-evidence"><div className="section-heading"><h3>Recovery and training</h3><span className="muted">{data.recoveryBand?.label}</span></div><TrendChart label="Recovery score" points={data.scoreHistory.map(point => ({date:point.date,value:point.recovery}))} max={100} percent /><TrendChart label="Training score" points={data.scoreHistory.map(point => ({date:point.date,value:point.training}))} max={100} percent blue />
          <div className="review-section"><h3>Training load</h3><dl className="metrics-line"><Metric label="Short term" value={data.load?.shortTerm?.toFixed(0) ?? "Not available"} /><Metric label="Long term" value={data.load?.longTerm?.toFixed(0) ?? "Not available"} /><Metric label="Load ratio" value={data.load?.ratio?.toFixed(2) ?? "Not available"} /></dl><TrendChart label="Training load (normalized)" points={data.loadHistory.map(point => ({date:point.date,value:point.ratio != null && Number.isFinite(point.ratio) && data.thresholds.ratioOptimizedHi > 0 ? Math.max(0,Math.min(100,point.ratio / data.thresholds.ratioOptimizedHi * 100)) : null}))} max={100} percent /><p className="context-note">100% marks the configured optimized-load ceiling ({data.thresholds.ratioOptimizedHi.toFixed(2)} times usual load). Values above this ceiling stay at 100%; the unscaled load ratio is shown above.</p>{data.loadRecommendation && <p className="context-note">{data.loadRecommendation.downWeek ? "Recovery week. " : ""}Target weekly load: {Math.round(data.loadRecommendation.target)}. {data.loadRecommendation.note}</p>}</div>
          <div className="review-section"><h3>Fitness and volume</h3><dl className="metrics-line"><Metric label="Distance" value={(data.stats.distanceM / 1000).toFixed(1)} unit="km" /><Metric label="Time" value={fmtDuration(data.stats.durationS)} /><Metric label="Sessions" value={data.stats.sessions} /><Metric label="VO2 max" value={data.fitness?.vo2max?.toFixed(1) ?? "Not available"} /><Metric label="Threshold" value={fmtPace(data.fitness?.thresholdSecKm)} unit="/km" /></dl><dl className="definition-list">{[["5 km", data.fitness?.predict5kS], ["10 km", data.fitness?.predict10kS], ["Half marathon", data.fitness?.predictHalfS], ["Marathon", data.fitness?.predictMarathonS]].map(([label, value]) => <div key={label as string}><dt>{label} prediction</dt><dd>{typeof value === "number" ? fmtDuration(value) : "Not available"}</dd></div>)}</dl></div>
          {currentPlan.data?.[0] && <div className="review-section"><RaceEstimates plan={currentPlan.data[0]} /></div>}
        </section><aside className="health-context"><h3>Risk contributors</h3>{data.riskFactors.map(factor => <div key={factor.label} className="risk-factor"><strong>{factor.label}</strong><meter min="0" max="1" value={factor.contribution} aria-label={`${factor.label} contribution`} /><p>{factor.detail}</p></div>)}<TrendChart label="Injury risk" points={data.scoreHistory.map(point => ({date:point.date,value:point.risk}))} max={100} percent />
          <h3>Daily health</h3><dl className="definition-list">{data.health.length ? (() => {
            const latest = [...data.health].sort((first, second) => second.date.localeCompare(first.date))[0];
            return [["Resting heart rate", latest.restingHr == null ? "Not recorded" : `${latest.restingHr} bpm`], ["HRV", latest.hrvAvg == null ? "Not recorded" : `${latest.hrvAvg} ms`], ["Sleep", latest.sleepMin == null ? "Not recorded" : fmtDuration(latest.sleepMin * 60)], ["Sleep score", latest.sleepScore ?? "Not recorded"], ["Steps", latest.steps?.toLocaleString() ?? "Not recorded"], ["Stress", latest.stressAvg ?? "Not recorded"]].map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{value}</dd></div>);
          })() : <p className="muted">No daily health measurements available.</p>}</dl><p className="muted">Training indicators are not a medical diagnosis. Persistent or severe pain should be assessed by a qualified clinician.</p></aside></div>
        <section className="review-section">
          <h3>Daily health history</h3>
          <div className="health-history"><TrendChart label="Sleep duration (hours)" points={data.health.map(day => ({date:day.date,value:day.sleepMin == null ? null : day.sleepMin / 60}))} /><TrendChart label="Heart rate variability (ms)" points={data.health.map(day => ({date:day.date,value:day.hrvAvg}))} blue /><TrendChart label="Resting heart rate (bpm)" points={data.health.map(day => ({date:day.date,value:day.restingHr}))} /></div>
          <div className="data-table-scroll" tabIndex={0} role="region" aria-label="Daily wearable measurements"><table><caption className="sr-only">Daily wearable measurements</caption><thead><tr><th>Date</th><th>Sleep</th><th>Deep sleep</th><th>REM sleep</th><th>Sleep score</th><th>HRV</th><th>Resting HR</th><th>Steps</th><th>Stress</th></tr></thead><tbody>{data.health.map(day => <tr key={day.date}><th>{fmtDateDMY(day.date)}</th><td>{day.sleepMin == null ? "Not recorded" : fmtDuration(day.sleepMin*60)}</td><td>{day.deepMin == null ? "Not recorded" : `${day.deepMin} min`}</td><td>{day.remMin == null ? "Not recorded" : `${day.remMin} min`}</td><td>{day.sleepScore ?? "Not recorded"}</td><td>{day.hrvAvg ?? "Not recorded"}</td><td>{day.restingHr ?? "Not recorded"}</td><td>{day.steps ?? "Not recorded"}</td><td>{day.stressAvg ?? "Not recorded"}</td></tr>)}</tbody></table></div>
            <InfoDialog title="Training boundaries"><dl className="definition-list">{[["Decreasing ceiling",data.thresholds.ratioDecreasingHi],["Resuming ceiling",data.thresholds.ratioResumingHi],["Maintaining ceiling",data.thresholds.ratioMaintainingHi],["Optimized ceiling",data.thresholds.ratioOptimizedHi],["Exhausted ceiling",data.thresholds.recoveryExhausted],["Fatigued ceiling",data.thresholds.recoveryFatigued],["Normal ceiling",data.thresholds.recoveryNormal]].map(([label,value]) => <div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}</dl><Button onClick={() => navigate("settings", "thresholds")}><IconAdjustments size={18} aria-hidden="true" />Edit training boundaries</Button></InfoDialog>
        </section>
        </Disclosure>
      </>}
    </>}
    </Change>
    <Presence>{logging && <LogPain onClose={() => setLogging(false)} />}</Presence>
  </>;
}

function TrendChart({ label, points, max, blue = false, percent = false }: { label: string; points: {date:string;value:number | null}[]; max?: number; blue?: boolean; percent?: boolean }) {
  const gradient = useId().replace(/[^a-zA-Z0-9_-]/g, "");
  const valid = points.filter((point): point is {date:string;value:number} => point.value != null && Number.isFinite(point.value)).sort((first, second) => first.date.localeCompare(second.date));
  const ceiling = max ?? Math.max(1, ...valid.map(point => point.value)) * 1.15;
  const firstDay = valid.length ? dateValue(valid[0].date).getTime() : 0;
  const span = valid.length ? Math.max(1, dateValue(valid[valid.length - 1].date).getTime() - firstDay) : 1;
  const coordinates = valid.map(point => ({horizontal:42 + (dateValue(point.date).getTime() - firstDay) / span * 538, vertical:128 - Math.max(0,Math.min(ceiling,point.value)) / ceiling * 108}));
  const color = blue ? "var(--desk-chart-secondary)" : "var(--desk-chart)";
  const latest = valid[valid.length - 1]?.value;
  return <figure className={`trend-chart ${blue ? "comparison" : ""}`} data-percent={percent || undefined}><figcaption><span>{label}</span><strong>{latest == null ? "No data" : `${latest.toFixed(percent || max === 100 ? 0 : 2)}${percent ? "%" : ""}`}</strong></figcaption>{valid.length ? <svg viewBox="0 0 600 150" role="img" aria-label={`${label}, ${valid.length} measurements; latest ${latest}${percent ? " percent; fixed scale 0 to 100 percent" : ""}`}>
    <defs><linearGradient id={gradient} x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stopColor={color} stopOpacity=".28" /><stop offset="100%" stopColor={color} stopOpacity="0" /></linearGradient></defs>
    <path d="M42 20H580 M42 74H580 M42 128H580" stroke="var(--desk-line)" fill="none" />
    {percent && <><text x="0" y="24">100%</text><text x="8" y="78">50%</text><text x="16" y="132">0%</text></>}
    <path className="chart-area" d={`M${coordinates[0].horizontal} 128 L${coordinates.map(point=>`${point.horizontal} ${point.vertical}`).join(" L")} L${coordinates[coordinates.length-1].horizontal} 128 Z`} fill={`url(#${gradient})`} />
    <polyline points={coordinates.map(point => `${point.horizontal},${point.vertical}`).join(" ")} fill="none" stroke={color} strokeWidth="2.5" strokeLinejoin="round" strokeLinecap="round" vectorEffect="non-scaling-stroke" />{coordinates.map((point,index) => <circle key={index} cx={point.horizontal} cy={point.vertical} r="3" fill={color}><title>{fmtDateDMY(valid[index].date)}: {valid[index].value.toFixed(percent ? 0 : 2)}{percent ? "%" : ""}</title></circle>)}<text x="42" y="146">{shortDate(valid[0].date)}</text><text x="580" y="146" textAnchor="end">{shortDate(valid[valid.length - 1].date)}</text></svg> : <p className="muted">No measurements in this period.</p>}<InfoDialog title={`Data for ${label.toLowerCase()}`}><div className="data-table-scroll" role="region" tabIndex={0} aria-label={`${label} measurements`}><table><caption className="sr-only">{label} measurements</caption><thead><tr><th>Date</th><th>{label}</th></tr></thead><tbody>{points.map((point, index) => <tr key={index}><th>{fmtDateDMY(point.date)}</th><td>{point.value == null || !Number.isFinite(point.value) ? "Not recorded" : `${point.value.toFixed(percent ? 0 : 2)}${percent ? "%" : ""}`}</td></tr>)}</tbody></table></div></InfoDialog></figure>;
}

function Milestones() {
  const data = useRemote(() => Promise.all([tauri.allActivities(), tauri.currentPlan()]));
  const [earnedOnly, setEarnedOnly] = useState(false);
  if (data.loading) return <Loading label="Loading milestones" />;
  if (data.error || !data.data) return <ErrorNotice error={data.error} retry={data.retry} />;
  const [activities, plan] = data.data;
  const stats = gamificationStats(activities, plan?.[0] ?? null, Boolean(plan?.[0]?.pushedId));
  return <section className="milestones"><dl className="metrics-line"><Metric label="Current streak" value={stats.streak} unit="days" /><Metric label="Best streak" value={stats.bestStreak} unit="days" /><Metric label="Running distance" value={stats.totalKm.toFixed(1)} unit="km" /><Metric label="Cross-training" value={stats.crossHours.toFixed(1)} unit="hours" /><Metric label="Longest run" value={stats.longestKm.toFixed(1)} unit="km" /></dl><h3>Last 12 weeks</h3><div className="consistency-grid" aria-label="Daily training consistency">{stats.recent.map(day => <div key={day.date} title={`${fmtDateDMY(day.date)}: ${day.km.toFixed(1)} km${day.cross ? ", cross-training" : ""}`} className={day.km > 0 ? "ran" : day.cross ? "cross-trained" : ""} style={{ opacity: day.km > 0 ? .4 + Math.min(day.km / 15, 1) * .6 : 1 }}><span className="sr-only">{fmtDateDMY(day.date)}: {day.km.toFixed(1)} km{day.cross ? ", cross-training" : ""}</span></div>)}</div><div className="section-heading"><h3>{stats.achievements.filter(badge => badge.earned).length} milestones reached</h3><label><input type="checkbox" checked={earnedOnly} onChange={event => setEarnedOnly(event.target.checked)} />Earned only</label></div><div className="milestone-list">{stats.achievements.filter(badge => !earnedOnly || badge.earned).map(badge => <article key={badge.id} className={badge.earned ? "earned" : ""}><IconTrophy size={24} aria-hidden="true" /><div><h3>{badge.name}</h3><p>{badge.desc}</p>{badge.target != null && <><progress max={badge.target} value={badge.progress ?? 0} aria-label={badge.name} /><small>{badge.progress ?? 0} / {badge.target} {badge.unit}</small></>}</div>{badge.earned && <IconCheck size={18} aria-label="Earned" />}</article>)}</div></section>;
}

function LogPain({ onClose }: { onClose: () => void }) {
  const [bodyPart, setBodyPart] = useState("");
  const [intensity, setIntensity] = useState(3);
  const [started, setStarted] = useState(today);
  const [notes, setNotes] = useState("");
  const command = useCommand();
  return <Dialog title="Log pain" onClose={onClose}><form onSubmit={async event => {
    event.preventDefault();
    if (await command.run(() => tauri.savePainEntry(bodyPart, intensity, started, notes), "Pain logged. Any proposed plan changes await your review.")) { navigate("health", "journal"); onClose(); }
  }}><div className="form-grid"><label className="full">Body region<select required autoFocus value={bodyPart} onChange={event => setBodyPart(event.target.value)}><option value="">Select a region</option>{Object.entries(BODY_REGIONS).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label><label className="full">Intensity: {intensity}/10<input type="range" min="0" max="10" step="1" value={intensity} onChange={event => setIntensity(Number(event.target.value))} /></label><label className="full">Started on<input type="date" required max={today()} value={started} onChange={event => setStarted(event.target.value)} /></label><label className="full">Notes<textarea value={notes} onChange={event => setNotes(event.target.value)} /></label></div><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={onClose}>Cancel</Button><Button type="submit" tone="primary" busy={command.busy}>Save pain entry</Button></div></form></Dialog>;
}

function PainJournal({ onLog }: { onLog: () => void }) {
  const entries = useRemote(tauri.painEntries);
  const history = useRemote(() => Promise.all([tauri.painDays(shiftDate(today(), -29), today()), tauri.painByRegion(30)]));
  const [resolved, setResolved] = useState(false);
  return <div className="pain-workspace"><section><div className="section-heading"><h3>Episodes</h3><label><input type="checkbox" checked={resolved} onChange={event => setResolved(event.target.checked)} />Include resolved</label></div><ErrorNotice error={entries.error} retry={entries.retry} />{entries.loading ? <Loading label="Loading pain journal" /> : !entries.data?.filter(entry => resolved || !entry.resolvedOn).length ? <Empty title="No active pain entries" action={<Button onClick={onLog}><IconPlus size={18} />Log pain</Button>}>Keep a record when something does not feel right.</Empty> : <><Button onClick={onLog}><IconPlus size={18} />Log pain</Button>{entries.data?.filter(entry => resolved || !entry.resolvedOn).map(entry => <PainEpisode key={entry.id} entry={entry} />)}</>}</section><aside><PlanAdjustments source="pain" /><h3 className="review-section">Last 30 days</h3><ErrorNotice error={history.error} retry={history.retry} />{history.data && <><TrendChart label="Daily pain intensity" max={10} points={history.data[0].map(([date,value]) => ({date,value}))} /><dl className="definition-list">{history.data[1].map(([region,value]) => <div key={region}><dt>{BODY_REGIONS[region] ?? region}</dt><dd>{value.toFixed(1)}</dd></div>)}</dl></>}</aside></div>;
}

function PainEpisode({ entry }: { entry: PainEntry }) {
  const command = useCommand();
  const [remove, setRemove] = useState(false);
  const [chat, setChat] = useState(false);
  const conversationId = useId();
  return <article className="pain-episode">
    <header><div><span className="muted">{fmtDateDMY(entry.startedOn)}{entry.resolvedOn ? ` / Resolved ${fmtDateDMY(entry.resolvedOn)}` : " / Active"}</span><h3>{BODY_REGIONS[entry.bodyPart] ?? entry.bodyPart}</h3></div><strong className="pain-intensity">{entry.intensity}<small>/10</small></strong></header>
    {entry.notes && <p className="episode-notes">{entry.notes}</p>}
    <div className="episode-actions"><Button busy={command.busy} onClick={() => void command.run(() => tauri.resolvePain(entry.id, entry.resolvedOn ? null : today()), entry.resolvedOn ? "Episode reopened." : "Mark resolved")}><IconCheck size={18} />{entry.resolvedOn ? "Reopen" : "Mark resolved"}</Button>{!entry.resolvedOn && !entry.followupAt && <Button busy={command.busy} onClick={() => void command.run(() => tauri.markPainFollowup(entry.id), "Follow-up recorded: still recovering.")}>Still recovering</Button>}<Button onClick={() => setChat(!chat)} aria-expanded={chat} aria-controls={conversationId}><IconMessage size={18} />Discuss episode<IconChevronDown className="disclosure-indicator" size={18} aria-hidden="true" /></Button><Button tone="danger" onClick={() => setRemove(true)}><IconTrash size={18} />Delete</Button></div>
    <ErrorNotice error={command.error} />
    <Disclosure id={conversationId} open={chat} mountOnOpen><EpisodeChat entry={entry} /></Disclosure>
    <Presence>{remove && <Dialog title="Delete this pain entry?" onClose={() => setRemove(false)}><p>This removes the entry and its associated conversation. This cannot be undone.</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setRemove(false)}>Keep entry</Button><Button tone="danger" busy={command.busy} onClick={() => void command.run(() => tauri.deletePainEntry(entry.id), "Pain entry deleted.")}>Delete entry</Button></div></Dialog>}</Presence>
  </article>;
}

function EpisodeChat({ entry }: { entry: PainEntry }) {
  const history = useRemote(() => tauri.painMessages(entry.id), [entry.id]);
  const status = useRemote(tauri.llmStatus);
  const [question, setQuestion] = useState("");
  const [consent, setConsent] = useState(false);
  const [clear, setClear] = useState(false);
  const command = useCommand();
  return <section className="episode-chat" aria-label="Episode conversation"><div className="section-heading"><h3>{status.data?.configured ? status.data.model ?? "Assistant" : "Built-in logger"}</h3><Button onClick={() => setClear(true)} disabled={!history.data?.length}>Clear conversation</Button></div><ErrorNotice error={history.error ?? status.error ?? command.error} />{history.data?.map(message => <div key={message.id} className={`chat-message ${message.role}`}><strong>{message.role === "user" ? "You" : "Assistant"}</strong><p>{message.content}</p></div>)}<form onSubmit={async event => {
    event.preventDefault();
    if (status.loading || status.error || (status.data?.configured && !consent)) return;
    if (await command.run(() => tauri.askAssistant(entry.id, question))) setQuestion("");
  }}><label>Your question<textarea required value={question} onChange={event => setQuestion(event.target.value)} /></label>{status.data?.configured && <label className="consent"><input type="checkbox" checked={consent} onChange={event => setConsent(event.target.checked)} />I agree to send this episode and conversation to my configured assistant provider.</label>}<p className="muted">General training guidance, not a diagnosis. The assistant cannot change your plan without your approval.</p><Button type="submit" tone="primary" busy={command.busy} disabled={!question.trim() || status.loading || Boolean(status.error) || Boolean(status.data?.configured && !consent)}>Send question</Button></form><Presence>{clear && <Dialog title="Clear this conversation?" onClose={() => setClear(false)}><p>The pain entry is kept. Messages will be permanently removed.</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setClear(false)}>Keep messages</Button><Button tone="danger" busy={command.busy} onClick={async () => { if (await command.run(() => tauri.clearPainMessages(entry.id), "Conversation cleared.")) setClear(false); }}>Clear messages</Button></div></Dialog>}</Presence></section>;
}