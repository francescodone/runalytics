import { useEffect, useRef, useState, type FormEvent } from "react";
import { IconArrowLeft, IconArrowRight, IconPlus, IconDeviceWatch, IconRotate, IconCopy, IconCalendarEvent, IconCheck, IconX } from "@tabler/icons-react";
import { tauri, fmtDateDMY, fmtDuration, fmtKm, fmtPace, parsePace, estimatedTimeRange, fmtTimeRange } from "../lib/ipc";
import type { CalendarEvent, PlanDefaults, PlanKind, PlanRequest, TrainingPlan } from "../lib/types";
import { cleanTitle, dateValue, dayName, kindName, monday, navigate, shiftDate, shortDate, today, useCommand, useRemote, useRoute, useWorkspace } from "./data";
import { Button, Dialog, Empty, ErrorNotice, IconButton, Inspector, Loading, Metric } from "./ui";
import { PlanAdjustments } from "./proposals";
import { RaceEstimates } from "./race-estimates";
import { Change, Presence, SelectionGroup, SelectionMarker } from "./interaction";

type Horizon = "week" | "month" | "plan";

export function Training() {
  const route = useRoute();
  const [cursor, setCursor] = useState(today);
  const [selectedDay, setSelectedDay] = useState(today);
  const [selected, setSelected] = useState<CalendarEvent | null>(null);
  const [create, setCreate] = useState(false);
  const [benchmark, setBenchmark] = useState(false);
  const [reset, setReset] = useState(false);
  const [cancel, setCancel] = useState(false);
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const horizon: Horizon = ["week", "month", "plan"].includes(route.section) ? route.section as Horizon : "week";
  const current = useRemote(tauri.currentPlan);
  const providers = useRemote(tauri.providerStatus);
  const defaults = useRemote(tauri.planDefaults);
  const receipt = useRemote(tauri.planReceipt);
  const downWeeks = useRemote(tauri.downWeeks);
  const remotePlan = useRemote(tauri.planSyncCheck);
  const command = useCommand();
  const { refresh, notify, planProgress } = useWorkspace();
  const first = horizon === "month" ? monday(`${cursor.slice(0, 7)}-01`) : monday(cursor);
  const days = Array.from({ length: horizon === "month" ? 42 : 7 }, (_, index) => shiftDate(first, index));
  const events = useRemote(() => tauri.calendarEvents(first, days[days.length - 1]), [first, horizon]);
  const plan = current.data?.[0];
  const workouts = current.data?.[1] ?? [];
  const connected = providers.data?.some(provider => provider.connected);
  const synced = providers.data?.some(provider => provider.lastSync);
  const weekKeys = [...new Set(workouts.map(workout => monday(workout.date)))].sort();
  const completed = workouts.filter(workout => workout.done).length;
  useEffect(() => { setSelected(null); }, [horizon, cursor]);
  const selectDate = (date: string) => { setSelectedDay(date); setSelected(null); };
  const moveHorizon = (direction: number) => {
    const next = horizon === "month" ? new Date(dateValue(cursor).getFullYear(), dateValue(cursor).getMonth() + direction, 1, 12) : dateValue(shiftDate(cursor, direction * 7));
    const value = `${next.getFullYear()}-${String(next.getMonth() + 1).padStart(2, "0")}-${String(next.getDate()).padStart(2, "0")}`;
    setCursor(value); setSelectedDay(value);
  };
  return <>
    <div className="training-context">
      <div><span className="muted">{plan ? `${plan.kind === "maintenance" ? "Maintenance" : plan.kind === "improvement" ? "Improvement" : "Race preparation"} / ${plan.distance}` : "Your training"}</span>
        <h2>{horizon === "plan" ? plan ? `Building toward ${shortDate(plan.targetDate)}` : "A plan that fits your life" : horizon === "month" ? dateValue(cursor).toLocaleDateString("en-GB", { month: "long", year: "numeric" }) : `${shortDate(first)} - ${shortDate(shiftDate(first, 6))}`}</h2></div>
      <div className="training-actions">{plan ? <><Button onClick={() => setBenchmark(true)}><IconPlus size={18} />Schedule race or test</Button><Button disabled={!connected} busy={command.busy} onClick={() => void command.run(() => tauri.pushPlan("coros"), "Plan sent to COROS.")}><IconDeviceWatch size={18} />{plan.pushedId ? "Update watch" : "Send to watch"}</Button></> : <Button tone="primary" onClick={() => setCreate(true)} disabled={current.loading || Boolean(current.error) || !connected || !synced}><IconPlus size={18} />Create plan</Button>}</div>
    </div>
    {command.busy && planProgress && <div className="native-progress" role="status"><span>{planProgress.stage}</span><progress max={100} value={planProgress.percent} aria-label="Watch publishing progress" /></div>}
    <ErrorNotice error={command.error} />
    <ErrorNotice error={current.error} retry={current.retry} />
    {!providers.loading && !connected && <div className="setup-band"><div><strong>Connect your watch</strong><p>Bring your runs and recovery data into your training.</p></div><Button tone="primary" onClick={() => navigate("settings", "setup")}>Connect COROS</Button></div>}
    {connected && !synced && <div className="setup-band"><div><strong>Your watch is connected</strong><p>Sync your training history to establish your starting point.</p></div><Button busy={command.busy} tone="primary" onClick={() => void command.run(() => tauri.syncNow("coros"), "Training history synced.")}>Sync now</Button></div>}
    <div className="desk-toolbar">
      <SelectionGroup><nav className="horizon-switch" aria-label="Training horizon">{(["week", "month", "plan"] as const).map(value => <a key={value} href={`#training/${value}`} aria-current={horizon === value ? "page" : undefined}>{horizon === value && <SelectionMarker />}{value[0].toUpperCase() + value.slice(1)}</a>)}</nav></SelectionGroup>
      {horizon !== "plan" && <div className="date-controls"><IconButton label={`Previous ${horizon}`} onClick={() => moveHorizon(-1)}><IconArrowLeft size={18} /></IconButton><Button onClick={() => { setCursor(today()); setSelectedDay(today()); }}>Today</Button><IconButton label={`Next ${horizon}`} onClick={() => moveHorizon(1)}><IconArrowRight size={18} /></IconButton></div>}
      {horizon === "week" && plan && <IconButton label="Reset this week" onClick={() => setReset(true)}><IconRotate size={20} /></IconButton>}
    </div>
    <Change identity={horizon}>
    {horizon === "plan" ? <div className="plan-studio">
      <section><h3>Training horizon</h3>{current.loading ? <Loading /> : !plan ? <Empty title="Give your training a direction" action={<Button tone="primary" disabled={!connected || !synced || Boolean(current.error)} onClick={() => setCreate(true)}>Create plan</Button>}>Prepare for a race, build fitness or maintain your routine.</Empty> : <>
        <dl className="metrics-line"><Metric label="Sessions complete" value={`${completed}/${workouts.length}`} /><Metric label="Runs per week" value={plan.weeklyFrequency} /><Metric label="Threshold pace" value={fmtPace(plan.thresholdSecKm)} unit="/km" /></dl>
        <div className="plan-roadmap">{weekKeys.map((week, index) => {
          const sessions = workouts.filter(workout => monday(workout.date) === week);
          const distance = sessions.reduce((sum, workout) => sum + (workout.distanceM ?? 0), 0);
          return <button key={week} className="roadmap-row" onClick={() => { setCursor(week); setSelectedDay(week); navigate("training", "week"); }}><span>Week {index + 1}<small>{shortDate(week)}{downWeeks.data?.includes(week) ? " / Recovery" : ""}</small></span><span className="roadmap-track"><i style={{ width: `${Math.min(100, distance / Math.max(...weekKeys.map(key => workouts.filter(workout => monday(workout.date) === key).reduce((sum, workout) => sum + (workout.distanceM ?? 0), 0)), 1) * 100)}%` }} /></span><strong>{fmtKm(distance)}<small>{sessions.filter(workout => workout.done).length}/{sessions.length} complete</small></strong><IconArrowRight size={18} /></button>;
        })}</div>
      </>}</section>
      <aside className="plan-basis"><h3>Plan basis</h3>{plan ? <>
        <dl className="definition-list"><div><dt>Target date</dt><dd>{fmtDateDMY(plan.targetDate)}</dd></div><div><dt>Starting volume</dt><dd>{plan.baselineWeeklyKm == null ? "From training history" : `${plan.baselineWeeklyKm.toFixed(1)} km/week`}</dd></div><div><dt>Progression</dt><dd>{plan.increase}</dd></div><div><dt>Watch</dt><dd>{plan.pushedId ? "Published" : "Not sent yet"}</dd></div></dl>
        {receipt.data?.map((line, index) => <p key={index}><strong>{line.input}</strong><br />{line.sentence}</p>)}
        {defaults.data && <Predictions defaults={defaults.data} />}
        <RaceEstimates plan={plan} />
        {remotePlan.data && <p className="context-note">Device plan: {({upToDate:"up to date",unpublished:"not yet published",updateAvailable:"update available",otherPlan:"different plan on the other device",importAvailable:"ready to import"})[remotePlan.data.status]}. <a href="#settings/calendar">Manage device sync</a></p>}
        <Button tone="danger" onClick={() => setCancel(true)}>End plan</Button>
      </> : <p className="muted">Your recent mileage and threshold pace establish the starting load. Plan generation remains local.</p>}</aside>
    </div> : <>
      <ErrorNotice error={events.error} retry={events.retry} />
      {events.loading ? <Loading label="Loading schedule" /> : <div className={`schedule-workspace ${selected ? "has-inspector" : ""}`}>
        <div className="schedule-canvas">
          <Change identity={first}>
          {horizon === "week" ? <>
            <SelectionGroup><div className="mobile-date-ribbon" aria-label="Days this week">{days.map(date => <button key={date} onClick={() => selectDate(date)} aria-pressed={date === selectedDay} aria-label={`${dayName(date)} ${fmtDateDMY(date)}`}>{date === selectedDay && <SelectionMarker />}<span>{dayName(date)}</span><strong>{dateValue(date).getDate()}</strong><i className={(events.data ?? []).some(event => event.date === date) ? "has-session" : ""} /></button>)}</div></SelectionGroup>
            <div className="week-board" onDragEnd={() => setDropTarget(null)}>{days.map(date => <section className={`day-lane ${date === today() ? "is-today" : ""}`} key={date} aria-label={`${dayName(date)} ${fmtDateDMY(date)}`}>
              <header><span>{dayName(date)}</span><strong>{dateValue(date).getDate()}</strong>{date === today() && <small>Today</small>}</header>
              <div className="day-sessions" data-drop-target={dropTarget === date || undefined} onDragOver={event => { if (event.dataTransfer.types.includes("text/plain")) { event.preventDefault(); event.dataTransfer.dropEffect = "move"; setDropTarget(date); } }} onDragLeave={event => { if (!event.currentTarget.contains(event.relatedTarget as Node)) setDropTarget(null); }} onDrop={event => {
                event.preventDefault(); setDropTarget(null);
                const identifier = event.dataTransfer.getData("text/plain");
                const session = events.data?.find(item => item.workoutId === identifier);
                if (session) { setSelected(session); setSelectedDay(date); }
              }}>{(events.data ?? []).filter(event => event.date === date).map(event => <SessionTile key={event.id} event={event} active={event.id === selected?.id} onSelect={() => { setSelected(event); setSelectedDay(event.date); }} />)}
              {!(events.data ?? []).some(event => event.date === date) && <p className="rest-label">Rest day</p>}</div>
            </section>)}</div>
          </> : <div className="month-board"><div className="month-labels">{["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map(day => <span key={day}>{day}</span>)}</div><div className="month-dates">{days.map(date => <div className={`month-day ${date.slice(0, 7) !== cursor.slice(0, 7) ? "outside-month" : ""}`} key={date}>
            <button className="month-number" aria-label={`Select ${fmtDateDMY(date)}`} aria-pressed={date === selectedDay} aria-current={date === today() ? "date" : undefined} onClick={() => selectDate(date)}>{dateValue(date).getDate()}<i>{(events.data ?? []).filter(event => event.date === date).length || ""}</i></button>
            <div className="month-sessions">{(events.data ?? []).filter(event => event.date === date).map(event => <SessionTile key={event.id} event={event} active={selected?.id === event.id} onSelect={() => { setSelected(event); setSelectedDay(event.date); }} />)}</div>
          </div>)}</div></div>}
          <Change identity={selectedDay} className="day-itinerary"><h3>{dayName(selectedDay)}, {shortDate(selectedDay)}</h3>{(events.data ?? []).filter(event => event.date === selectedDay).map(event => <SessionTile key={event.id} event={event} active={selected?.id === event.id} onSelect={() => setSelected(event)} />)}{!(events.data ?? []).some(event => event.date === selectedDay) && <p className="muted">No sessions scheduled. Time to recover.</p>}</Change>
          <div className="schedule-foot"><span>{(events.data ?? []).length} sessions in this {horizon}</span><span>{fmtKm((events.data ?? []).reduce((total, event) => total + (event.distanceM ?? 0), 0))}</span></div>
          </Change>
        </div>
        <Presence>{selected && <Inspector key="session" title="Session" onClose={() => setSelected(null)}><Change identity={`${selected.id}:${selectedDay}`}><SessionDetail event={selected} plan={plan} proposedDate={selectedDay} onDone={() => setSelected(null)} /></Change></Inspector>}</Presence>
      </div>}
    </>}
    </Change>
    {horizon === "plan" && plan && <PlanAdjustments source="pacing" />}
    <Presence>
    {benchmark && plan && <ScheduleBenchmark key="benchmark" plan={plan} initialDate={selectedDay} onClose={() => setBenchmark(false)} onScheduled={date => { setBenchmark(false); setCursor(date); setSelectedDay(date); setSelected(null); navigate("training", "week"); }} />}
    {create && <CreatePlan key="create" defaults={defaults.data} onClose={() => setCreate(false)} />}
    {reset && <ResetWeek key="reset" week={first} onClose={() => setReset(false)} />}
    {cancel && <Dialog key="cancel" title="End this plan?" onClose={() => setCancel(false)}><p>Future sessions will be removed. Completed activities stay in your journal. Runalytics will also cancel the plan on your watch.</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setCancel(false)}>Keep plan</Button><Button tone="danger" busy={command.busy} onClick={() => void command.run(async () => {
      try { await tauri.abortPlan(); } catch (error) { refresh(); notify(String(error)); throw error; }
      setCancel(false);
    }, "Plan ended. Your completed runs are retained.")}>End plan</Button></div></Dialog>}
    </Presence>
  </>;
}

function SessionTile({ event, active, onSelect }: { event: CalendarEvent; active: boolean; onSelect: () => void }) {
  return <button className={`session-tile kind-${event.kind} ${active ? "selected" : ""}`} aria-pressed={active} onClick={onSelect} draggable={Boolean(event.workoutId && event.activityId == null && event.kind !== "race" && event.status !== "skipped")} onDragStart={drag => { drag.dataTransfer.setData("text/plain", event.workoutId ?? ""); drag.dataTransfer.effectAllowed = "move"; drag.currentTarget.classList.add("is-dragging"); }} onDragEnd={drag => drag.currentTarget.classList.remove("is-dragging")}>
    <span className="session-type">{kindName(event.kind)}{event.activityId != null && <IconCheck size={15} aria-label="Completed" />}{event.status === "skipped" && <IconX size={15} aria-label="Skipped" />}</span>
    <strong>{cleanTitle(event.title)}</strong><span>{event.distanceM != null ? fmtKm(event.distanceM) : fmtTimeRange(estimatedTimeRange(event)) ?? (event.durationMin != null ? `${Math.round(event.durationMin)} min` : "Recovery")}</span>
    {event.missed && <small>Missed</small>}{event.adjusted && <small>Adjusted for recovery</small>}{event.moved && <small>Rescheduled</small>}
  </button>;
}

export function SessionDetail({ event, plan, proposedDate, onDone }: { event: CalendarEvent; plan?: TrainingPlan; proposedDate?: string; onDone: () => void }) {
  const command = useCommand();
  const { notify } = useWorkspace();
  const [date, setDate] = useState(proposedDate ?? event.date);
  const [match, setMatch] = useState("");
  const candidates = useRemote(() => event.workoutId && event.activityId == null ? tauri.calendarEvents(shiftDate(event.date, -7), shiftDate(event.date, 7)) : Promise.resolve([]), [event.id]);
  const review = useRemote(() => event.activityId != null ? tauri.activityReview(event.activityId) : Promise.resolve(null), [event.activityId]);
  const editable = event.workoutId && event.activityId == null && event.kind !== "race";
  return <div className="session-detail"><span className="session-type">{kindName(event.kind)}</span><h2>{cleanTitle(event.title)}</h2><p className="muted">{fmtDateDMY(event.date)}{event.moved && event.plannedDate ? ` / Originally ${fmtDateDMY(event.plannedDate)}` : ""}</p>
    {event.adjusted && <p className="context-note">Adjusted for you: {event.adjusted}</p>}
    <dl className="metrics-line"><Metric label="Distance" value={event.distanceM != null ? (event.distanceM / 1000).toFixed(1) : "Not specified"} unit={event.distanceM != null ? "km" : undefined} />{event.activityId != null ? event.durationMin != null && <Metric label="Duration" value={Math.round(event.durationMin)} unit="min" /> : fmtTimeRange(estimatedTimeRange(event)) && <Metric label="Estimated time" value={fmtTimeRange(estimatedTimeRange(event))} />}</dl>
    <div className="prescription">{event.notes.split(/\n\s*\n/).filter(Boolean).map((paragraph, index) => <p key={index}>{paragraph}</p>)}</div>
    <Button onClick={() => void command.run(() => navigator.clipboard.writeText(event.notes), "Session copied.")}><IconCopy size={18} />Copy prescription</Button>
    {review.data && <section className="session-recording"><h3>Recorded run</h3><p>{fmtDuration(review.data.activity.durationS)} at {fmtPace(review.data.activity.avgPaceSKm)}/km</p><Button onClick={() => navigate("journal", String(event.activityId))}>Open activity review</Button></section>}
    <ErrorNotice error={command.error ?? review.error} />
    {editable && <section className="session-management"><h3>Schedule</h3><form onSubmit={async submit => {
      submit.preventDefault();
      if (await command.run(() => tauri.moveWorkout(event.workoutId!, date))) {
        notify(`Moved to ${fmtDateDMY(date)}.`, { label: "Undo", run: () => tauri.moveWorkout(event.workoutId!, event.date) }); onDone();
      }
    }}><label>Run on<input type="date" required min={today()} max={plan?.targetDate} value={date} onChange={change => setDate(change.target.value)} /></label><Button type="submit" busy={command.busy} disabled={date === event.date || event.status === "skipped"}><IconCalendarEvent size={18} />Move session</Button></form>
      <Button busy={command.busy} onClick={async () => {
        const skipped = event.status !== "skipped";
        if (await command.run(() => tauri.setWorkoutSkipped(event.workoutId!, skipped))) {
          notify(skipped ? "Session skipped. It will not count as missed." : "Session restored.", { label: "Undo", run: () => tauri.setWorkoutSkipped(event.workoutId!, !skipped) }); onDone();
        }
      }}>{event.status === "skipped" ? "Restore session" : "Skip session"}</Button>
    </section>}
    {event.workoutId && event.activityId == null && <section className="session-management"><h3>Already ran this session?</h3><ErrorNotice error={candidates.error} retry={candidates.retry} /><form onSubmit={async submit => {
      submit.preventDefault();
      if (await command.run(() => tauri.matchCalendarActivity(event.workoutId!, Number(match)), "Recorded run matched to this session.")) onDone();
    }}><label>Recorded activity<select required value={match} onChange={change => setMatch(change.target.value)}><option value="">Choose a nearby run</option>{candidates.data?.filter(candidate => candidate.activityId != null && candidate.workoutId == null).map(candidate => <option key={candidate.id} value={candidate.activityId!}>{fmtDateDMY(candidate.date)} / {fmtKm(candidate.distanceM ?? 0)}</option>)}</select></label><Button type="submit" disabled={!match} busy={command.busy}>Match run</Button></form></section>}
  </div>;
}

function ScheduleBenchmark({ plan, initialDate, onClose, onScheduled }: { plan: TrainingPlan; initialDate: string; onClose: () => void; onScheduled: (date: string) => void }) {
  const command = useCommand();
  const earliest = [shiftDate(today(), 1), shiftDate(plan.createdAt.slice(0, 10), 1)].sort()[1];
  const latest = shiftDate(plan.targetDate, -8);
  const [preset, setPreset] = useState("parkrun");
  const [name, setName] = useState("parkrun");
  const [distance, setDistance] = useState(5);
  const [date, setDate] = useState(initialDate >= earliest && initialDate <= latest ? initialDate : earliest);
  return <Dialog title="Schedule race or test" onClose={onClose}><form className="form-grid" onSubmit={async submit => {
    submit.preventDefault();
    if (await command.run(() => tauri.scheduleBenchmark(date, name, distance * 1000), "Benchmark scheduled. Nearby hard sessions have been eased.")) onScheduled(date);
  }}>
    <label className="full">Event<select value={preset} onChange={change => {
      const value = change.target.value; setPreset(value);
      setName(value === "parkrun" ? "parkrun" : value === "race" ? "Tune-up race" : "Time trial");
      if (value === "parkrun") setDistance(5);
    }}><option value="parkrun">parkrun / 5 km</option><option value="race">Tune-up race</option><option value="trial">Time trial</option></select></label>
    <label className="full">Event name<input required maxLength={80} value={name} onChange={change => setName(change.target.value)} /></label>
    <label>Date<input type="date" required min={earliest} max={latest} value={date} onChange={change => setDate(change.target.value)} /></label>
    <label>Distance (km)<input type="number" required min={1} max={10} step={0.1} disabled={preset === "parkrun"} value={distance} onChange={change => setDistance(Number(change.target.value))} /></label>
    <p className="context-note full">Replaces that day's session and eases adjacent hard or long runs. Your target race stays unchanged. Run this as a hard effort; match the race-only recording afterwards for a pace review.</p>
    {earliest > latest && <p className="context-note full">There is no room for a benchmark before the final week of this plan.</p>}
    <div className="full"><ErrorNotice error={command.error} /></div>
    <div className="form-actions full"><Button onClick={onClose}>Cancel</Button><Button type="submit" tone="primary" busy={command.busy} disabled={earliest > latest || !name.trim()}><IconCalendarEvent size={18} />Schedule benchmark</Button></div>
  </form></Dialog>;
}

function Predictions({ defaults }: { defaults: PlanDefaults }) {
  return <section className="watch-predictions"><h3>Watch predictions</h3><dl className="definition-list">{[["5 km", defaults.predict5kS], ["10 km", defaults.predict10kS], ["Half marathon", defaults.predictHalfS], ["Marathon", defaults.predictMarathonS]].map(([label, seconds]) => <div key={label as string}><dt>{label}</dt><dd>{typeof seconds === "number" ? fmtDuration(seconds) : "Not available"}</dd></div>)}</dl></section>;
}

function CreatePlan({ defaults, onClose }: { defaults?: PlanDefaults; onClose: () => void }) {
  const command = useCommand();
  const form = useRef<HTMLFormElement>(null);
  const [step, setStep] = useState(0);
  const [kind, setKind] = useState<PlanKind>("race");
  const [distance, setDistance] = useState<PlanRequest["distance"]>("21K");
  const [target, setTarget] = useState(shiftDate(today(), 70));
  const [weeks, setWeeks] = useState(10);
  const [threshold, setThreshold] = useState(defaults?.thresholdSecKm ? fmtPace(defaults.thresholdSecKm) : "");
  const [mileage, setMileage] = useState(defaults?.currentWeeklyKm?.toString() ?? "");
  const [frequency, setFrequency] = useState(4);
  const [longDay, setLongDay] = useState(7);
  const [increase, setIncrease] = useState<PlanRequest["increase"]>("moderate");
  const [validation, setValidation] = useState<string | null>(null);
  const effectiveDate = kind === "maintenance" ? shiftDate(today(), weeks * 7) : target;
  useEffect(() => { if (step > 0) form.current?.querySelector<HTMLElement>(step === 2 ? "h3" : "input,select")?.focus({ preventScroll: true }); }, [step]);
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); setValidation(null);
    if (step === 1 && (parsePace(threshold) == null || parsePace(threshold)! <= 0)) { setValidation("Enter a threshold pace such as 4:45."); return; }
    if (step < 2) { setStep(step + 1); return; }
    const parsed = parsePace(threshold);
    if (parsed == null || parsed <= 0 || effectiveDate < shiftDate(today(), 15)) { setValidation("Check your pace and choose an end date at least 15 days from today."); return; }
    if (await command.run(() => tauri.createPlan({ kind, distance, targetDate: effectiveDate, thresholdSecKm: parsed, weeklyFrequency: frequency, longrunDay: longDay, increase, currentWeeklyKm: mileage ? Number(mileage) : null }), "Your plan is ready.")) { navigate("training", "plan"); onClose(); }
  }
  return <Dialog title="Create training plan" wide onClose={onClose}><div className="plan-composer"><form ref={form} onSubmit={submit}><div className="composer-progress" aria-label={`Step ${step + 1} of 3`}>{["Direction", "Your baseline", "Review"].map((label, index) => <span key={label} aria-current={index === step ? "step" : undefined}>{index + 1}. {label}</span>)}</div>
    <ErrorNotice error={validation ?? command.error} />
    <Change identity={step}>
    {step === 0 && <div className="form-grid"><fieldset className="full"><legend>What are you training for?</legend>{([ ["race", "A race"], ["improvement", "Improve my fitness"], ["maintenance", "Stay consistent"] ] as const).map(([value, label]) => <label key={value}><input type="radio" name="plan-kind" value={value} checked={kind === value} onChange={() => setKind(value)} />{label}</label>)}</fieldset>
      <label>Distance<select value={distance} onChange={event => setDistance(event.target.value as PlanRequest["distance"])}><option value="5K">5 km</option><option value="10K">10 km</option><option value="21K">Half marathon</option><option value="42K">Marathon</option></select></label>
      {kind === "maintenance" ? <label>Block length<select value={weeks} onChange={event => setWeeks(Number(event.target.value))}>{[4, 6, 8, 10, 12, 16].map(value => <option key={value} value={value}>{value} weeks</option>)}</select></label> : <label>{kind === "race" ? "Race date" : "Time trial date"}<input type="date" required min={shiftDate(today(), 15)} value={target} onChange={event => setTarget(event.target.value)} /></label>}
    </div>}
    {step === 1 && <div className="form-grid"><label>Threshold pace (min/km)<input required value={threshold} placeholder="4:45" pattern="[0-9]{1,2}:[0-5][0-9]" onChange={event => setThreshold(event.target.value)} /></label><label>Weekly distance (km)<input type="number" min="5" max="250" step="0.1" value={mileage} onChange={event => setMileage(event.target.value)} /></label><label>Runs per week<input type="number" min="3" max="7" required value={frequency} onChange={event => setFrequency(Number(event.target.value))} /></label><label>Long run day<select value={longDay} onChange={event => setLongDay(Number(event.target.value))}>{["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"].map((day, index) => <option key={day} value={index + 1}>{day}</option>)}</select></label>{kind !== "maintenance" && <label className="full">Mileage progression<select value={increase} onChange={event => setIncrease(event.target.value as PlanRequest["increase"])}><option value="conservative">Conservative / 5%</option><option value="moderate">Moderate / 8%</option><option value="ambitious">Ambitious / 12%</option></select></label>}</div>}
    {step === 2 && <><h3 tabIndex={-1}>Ready to build your plan</h3><p>Your schedule is generated locally from these inputs. Sending it to your watch is a separate action.</p>{defaults?.activePain && <p className="context-note">Unresolved pain: {defaults.activePain.bodyPart}, {defaults.activePain.intensity}/10. Consider reviewing it in Health before starting a new plan.</p>}</>}
    </Change>
    <div className="form-actions">{step > 0 && <Button onClick={() => setStep(step - 1)}>Back</Button>}<Button tone="primary" type="submit" busy={command.busy}>{step === 2 ? "Build plan" : "Continue"}</Button></div>
    </form><aside className="composer-summary"><h3>Your training brief</h3><dl className="definition-list"><div><dt>Goal</dt><dd>{kind}</dd></div><div><dt>Distance</dt><dd>{distance}</dd></div><div><dt>End date</dt><dd>{fmtDateDMY(effectiveDate)}</dd></div><div><dt>Frequency</dt><dd>{frequency} runs/week</dd></div><div><dt>Starting volume</dt><dd>{mileage ? `${mileage} km/week` : "From synced history"}</dd></div><div><dt>Threshold</dt><dd>{threshold || "Not entered"}</dd></div></dl>{defaults?.weeksObserved ? <p className="muted">Baseline from {defaults.weeksObserved} complete weeks of training.</p> : null}</aside></div></Dialog>;
}

function ResetWeek({ week, onClose }: { week: string; onClose: () => void }) {
  const deviations = useRemote(() => tauri.weekDeviations(week), [week]);
  const command = useCommand();
  return <Dialog title="Restore original schedule" onClose={onClose}><p>Completed, skipped and race sessions stay where they are. Only eligible future sessions will move.</p><ErrorNotice error={deviations.error ?? command.error} retry={deviations.retry} />{deviations.loading ? <Loading /> : <ul className="plain-list">{deviations.data?.map(change => <li key={change.workoutId}><strong>{change.title}</strong><span>{fmtDateDMY(change.from)} to {fmtDateDMY(change.to)}{!change.restorable ? " / cannot move" : ""}</span></li>)}</ul>}<div className="form-actions"><Button onClick={onClose}>Cancel</Button><Button tone="primary" busy={command.busy} disabled={!deviations.data?.some(change => change.restorable)} onClick={async () => { if (await command.run(() => tauri.resetWeek(week), "Week restored to its original schedule.")) onClose(); }}>Restore week</Button></div></Dialog>;
}