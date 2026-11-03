import { useEffect, useLayoutEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import { IconArrowLeft, IconArrowRight, IconPlus, IconCopy, IconCalendarEvent, IconCheck, IconX, IconRestore } from "@tabler/icons-react";
import { tauri, fmtDateDMY, fmtDuration, fmtKm, fmtPace, parsePace, estimatedTimeRange, fmtTimeRange } from "../lib/ipc";
import type { CalendarEvent, PlanDefaults, PlanKind, PlanRequest, TrainingPlan, WeekDeviation } from "../lib/types";
import { displayPlanName } from "../lib/types";
import { capitalizeFirst, dateValue, dayName, kindName, monday, navigate, sessionTitle, shiftDate, shortDate, today, useCommand, useRemote, useRoute, useWorkspace } from "./data";
import { Button, Dialog, Empty, ErrorNotice, IconButton, Inspector, Loading, Metric } from "./ui";
import { PlanAdjustments } from "./proposals";
import { RaceEstimates } from "./race-estimates";
import { Change, Presence, SelectionGroup, SelectionMarker, useCompactLayout } from "./interaction";

type Horizon = "week" | "month" | "plan";

/// When a pointer-drag released. The click the browser fires after the
/// gesture must not open the session the runner just moved — same guard the
/// legacy calendar uses. Module-level because the tile that receives the
/// click is rendered apart from the board that owned the drag.
let dragEndedAt = 0;

export function Training() {
  const route = useRoute();
  const compact = useCompactLayout();
  const [cursor, setCursor] = useState(today);
  const [selectedDay, setSelectedDay] = useState(today);
  const [selected, setSelected] = useState<CalendarEvent | null>(null);
  const [create, setCreate] = useState(false);
  const [benchmark, setBenchmark] = useState(false);
  const [cancel, setCancel] = useState(false);
  const [reset, setReset] = useState(false);
  const canvasRef = useRef<HTMLDivElement>(null);
  const horizon: Horizon = ["week", "month", "plan"].includes(route.section) ? route.section as Horizon : "week";
  const current = useRemote(tauri.currentPlan);
  const providers = useRemote(tauri.providerStatus);
  const defaults = useRemote(tauri.planDefaults);
  const receipt = useRemote(tauri.planReceipt);
  const downWeeks = useRemote(tauri.downWeeks);
  const weekProgress = useRemote(tauri.planWeekProgress);
  const remotePlan = useRemote(tauri.planSyncCheck);
  const command = useCommand();
  const { refresh, notify, planProgress } = useWorkspace();
  const first = horizon === "month" ? monday(`${cursor.slice(0, 7)}-01`) : monday(cursor);
  // A month board shows whole weeks, but only the weeks the month actually
  // touches: a leading or trailing week with no day inside the selected month
  // is another month's training and used to render as a full ghost row.
  const monthWeeks = (() => {
    if (horizon !== "month") return 1;
    const firstOfMonth = dateValue(`${cursor.slice(0, 7)}-01`);
    const lead = firstOfMonth.getDay() === 0 ? 6 : firstOfMonth.getDay() - 1;
    const daysInMonth = new Date(firstOfMonth.getFullYear(), firstOfMonth.getMonth() + 1, 0).getDate();
    return Math.ceil((lead + daysInMonth) / 7);
  })();
  const days = Array.from({ length: monthWeeks * 7 }, (_, index) => shiftDate(first, index));
  const events = useRemote(() => tauri.calendarEvents(first, days[days.length - 1]), [first, horizon]);
  const plan = current.data?.[0];
  const workouts = current.data?.[1] ?? [];
  const connected = providers.data?.some(provider => provider.connected);
  const synced = providers.data?.some(provider => provider.lastSync);
  const weekKeys = [...new Set(workouts.map(workout => monday(workout.date)))].sort();
  const completed = workouts.filter(workout => workout.done).length;
  useEffect(() => { setSelected(null); }, [horizon, cursor]);
  // The plan lives on the watch without being asked for it: every schedule
  // edit is re-pushed by the backend on its own, so the only moment that
  // needs a push from here is a plan that has never reached the watch —
  // created while offline, or edited before it was ever sent. One attempt
  // per plan: a failure stays quiet (the watch is not the plan's only home)
  // and the next edit or the next visit tries again.
  const [syncingPlan, setSyncingPlan] = useState(false);
  const watchAttempted = useRef<string | null>(null);
  useEffect(() => {
    if (!plan || !connected || plan.status === "pushed") return;
    if (watchAttempted.current === plan.id) return;
    watchAttempted.current = plan.id;
    setSyncingPlan(true);
    tauri.pushPlan("coros").then(
      () => { notify("Plan sent to COROS."); refresh(); },
      error => console.warn("auto plan push failed", error),
    ).finally(() => setSyncingPlan(false));
  }, [plan?.id, plan?.status, connected]);
  // Drag-to-reschedule, pointer-driven rather than HTML5 drag-and-drop: the
  // app renders in WKWebView, where `draggable="true"` on a button never
  // starts a drag session — the gesture died silently and the click that
  // followed opened the session instead, which is exactly what it looked
  // like. A pointerdown/move/up gesture works in WebKit and Chromium alike;
  // the legacy calendar already drives drag this way (calendar.ts).
  // Touch drags horizontally (tiles opt out of vertical panning via
  // touch-action), so scrolling the page still wins the vertical axis.
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let tile: HTMLElement | null = null;
    let ghost: HTMLElement | null = null;
    let activeId = -1;
    let startX = 0;
    let startY = 0;
    let moving = false;
    const clearTargets = () => canvas.querySelectorAll("[data-drop-target]").forEach(node => node.removeAttribute("data-drop-target"));
    const targetAt = (x: number, y: number) => document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-drop-day]") ?? null;
    const reset = () => {
      tile = null;
      activeId = -1;
      moving = false;
      clearTargets();
      canvas.querySelectorAll(".is-dragging").forEach(node => node.classList.remove("is-dragging"));
      ghost?.remove();
      ghost = null;
    };
    const onPointerDown = (event: PointerEvent) => {
      if (event.button !== 0) return;
      const target = (event.target as Element).closest<HTMLElement>(".session-tile.movable");
      if (!target) return;
      tile = target;
      activeId = event.pointerId;
      startX = event.clientX;
      startY = event.clientY;
      moving = false;
    };
    // Move and release are watched on the window: once the pointer leaves the
    // canvas the canvas stops seeing events, and a drag ending there would
    // never be cleaned up. Pointer capture is deliberately not used — it
    // retargets the click that follows, which would stop a plain tap on a
    // tile from opening the session.
    const onPointerMove = (event: PointerEvent) => {
      if (!tile || event.pointerId !== activeId) return;
      const dx = event.clientX - startX;
      const dy = event.clientY - startY;
      if (!moving) {
        // Small jitter stays a click; touch must travel sideways, because a
        // vertical pull is the reader scrolling the page, not rescheduling.
        const touch = event.pointerType === "touch";
        if (touch ? Math.abs(dx) < 10 || Math.abs(dx) < Math.abs(dy) : Math.abs(dx) < 5 && Math.abs(dy) < 5) return;
        moving = true;
        tile.classList.add("is-dragging");
        ghost = tile.cloneNode(true) as HTMLElement;
        ghost.classList.remove("is-dragging");
        ghost.classList.add("tile-ghost");
        ghost.style.width = `${tile.getBoundingClientRect().width}px`;
        document.body.append(ghost);
      }
      event.preventDefault();
      ghost?.style.setProperty("transform", `translate(${event.clientX + 10}px, ${event.clientY + 10}px)`);
      const drop = targetAt(event.clientX, event.clientY);
      if (drop?.hasAttribute("data-drop-target")) return;
      clearTargets();
      drop?.setAttribute("data-drop-target", "");
    };
    const onPointerUp = (event: PointerEvent) => {
      if (!tile || event.pointerId !== activeId) return;
      if (moving) {
        // The browser fires a click after the release; it was a reschedule,
        // not a request to open the session.
        dragEndedAt = performance.now();
        const drop = targetAt(event.clientX, event.clientY);
        const date = drop?.dataset.dropDay;
        const workoutId = tile.dataset.workoutId;
        const from = tile.dataset.sessionDate;
        if (workoutId && date && date !== from) {
          void tauri.moveWorkout(workoutId, date).then(() => {
            refresh();
            notify(`Moved to ${fmtDateDMY(date)}.`, { label: "Undo", run: () => tauri.moveWorkout(workoutId, from!) });
          }, error => notify(String(error)));
        }
      }
      reset();
    };
    canvas.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    window.addEventListener("pointercancel", reset);
    return () => {
      canvas.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
      window.removeEventListener("pointercancel", reset);
    };
    // The canvas only exists once the schedule has loaded, so the effect has
    // to run again when loading finishes.
  }, [horizon, events.loading]);
  const selectDate = (date: string) => { setSelectedDay(date); setSelected(null); };
  const moveHorizon = (direction: number) => {
    const next = horizon === "month" ? new Date(dateValue(cursor).getFullYear(), dateValue(cursor).getMonth() + direction, 1, 12) : dateValue(shiftDate(cursor, direction * 7));
    const value = `${next.getFullYear()}-${String(next.getMonth() + 1).padStart(2, "0")}-${String(next.getDate()).padStart(2, "0")}`;
    setCursor(value); setSelectedDay(value);
  };
  const shown = events.data ?? [];
  const done = shown.filter(event => event.activityId != null);
  const visibleWeeks = new Set(days.map(date => monday(date)));
  const plannedM = (weekProgress.data ?? []).filter(entry => visibleWeeks.has(entry.weekStart)).reduce((total, entry) => total + entry.plannedM, 0);
  const ranM = done.reduce((total, event) => total + (event.distanceM ?? 0), 0);
  const scheduleSummary = horizon !== "plan" && events.data && <div className="schedule-summary"><span>{done.length}/{shown.length} sessions done</span><span>{fmtKm(ranM)} / {fmtKm(plannedM)} done</span>{!compact && horizon === "week" && plan && <Button onClick={() => setReset(true)}><IconRestore size={18} aria-hidden="true" />Reset week</Button>}</div>;
  return <>
    <div className="training-context">
      <div><span className="muted">{plan ? displayPlanName(plan) : "Your training"}</span>
        <h2>{horizon === "plan" ? plan ? `Building toward ${shortDate(plan.targetDate)}` : "A plan that fits your life" : horizon === "month" ? dateValue(cursor).toLocaleDateString("en-GB", { month: "long", year: "numeric" }) : compact ? `${dateValue(first).toLocaleDateString("en-GB", { day: "numeric", month: "short" })} - ${dateValue(shiftDate(first, 6)).toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" })}` : `${shortDate(first)} - ${shortDate(shiftDate(first, 6))}`}</h2></div>
      {(!compact || !plan) && <div className="training-actions">{plan ? <Button onClick={() => setBenchmark(true)}><IconPlus size={18} />Schedule race or test</Button> : <Button tone="primary" onClick={() => setCreate(true)} disabled={current.loading || Boolean(current.error) || !connected || !synced}><IconPlus size={18} />Create plan</Button>}</div>}
    </div>
    {syncingPlan && planProgress && <div className="native-progress" role="status"><span>{planProgress.stage}</span><progress max={100} value={planProgress.percent} aria-label="Watch publishing progress" /></div>}
    <ErrorNotice error={command.error} />
    <ErrorNotice error={current.error} retry={current.retry} />
    {!providers.loading && !connected && <div className="setup-band"><div><strong>Connect your watch</strong><p>Bring your runs and recovery data into your training.</p></div><Button tone="primary" onClick={() => navigate("settings", "setup")}>Connect COROS</Button></div>}
    {connected && !synced && <div className="setup-band"><div><strong>Your watch is connected</strong><p>Sync your training history to establish your starting point.</p></div><Button busy={command.busy} tone="primary" onClick={() => void command.run(() => tauri.syncNow("coros"), "Training history synced.")}>Sync now</Button></div>}
    <div className="desk-toolbar training-toolbar">
      <SelectionGroup><nav className="horizon-switch" aria-label="Training horizon">{(["week", "month", "plan"] as const).map(value => <a key={value} href={`#training/${value}`} aria-current={horizon === value ? "page" : undefined}>{horizon === value && <SelectionMarker />}{value[0].toUpperCase() + value.slice(1)}</a>)}</nav></SelectionGroup>
      {compact && horizon === "plan" && plan && <Button aria-label="Schedule race or test" onClick={() => setBenchmark(true)}><IconPlus size={18} aria-hidden="true" />Race or test</Button>}
      {horizon !== "plan" && <div className="date-controls"><IconButton label={`Previous ${horizon}`} onClick={() => moveHorizon(-1)}><IconArrowLeft size={18} /></IconButton><Button onClick={() => { setCursor(today()); setSelectedDay(today()); }}>Today</Button><IconButton label={`Next ${horizon}`} onClick={() => moveHorizon(1)}><IconArrowRight size={18} /></IconButton></div>}
    </div>
    {!compact && scheduleSummary}
    <Change identity={horizon}>
    {horizon === "plan" ? <div className="plan-studio">
      <section className="plan-overview"><h3>Training horizon</h3>{current.loading ? <Loading /> : !plan ? <Empty title="Give your training a direction" action={<Button tone="primary" disabled={!connected || !synced || Boolean(current.error)} onClick={() => setCreate(true)}>Create plan</Button>}>Prepare for a race, build fitness or maintain your routine.</Empty> : <>
        <dl className="metrics-line"><Metric label="Training method" value={plan.trainingMethod === "runna_like" ? "Runna-like" : "Norwegian method"} /><Metric label="Sessions complete" value={`${completed}/${workouts.length}`} /><Metric label="Runs per week" value={plan.weeklyFrequency} /><Metric label="Threshold pace" value={fmtPace(plan.thresholdSecKm)} unit="/km" /></dl>
        <PlanRoadmap identity={weekKeys.join("|")}>{weekKeys.map((week, index) => {
          const sessions = workouts.filter(workout => monday(workout.date) === week);
          const distance = sessions.reduce((sum, workout) => sum + (workout.distanceM ?? 0), 0);
          // Done/current/upcoming comes from the calendar, not from how many
          // sessions happen to be ticked: a week is past once its Sunday has
          // gone, current while it holds today, upcoming after that. The ran
          // km is the banked mileage the backend measured for that week, so
          // the roadmap and the dashboard can never disagree.
          const progress = weekProgress.data?.find(entry => entry.weekStart === week);
          const state = shiftDate(week, 6) < today() ? "past" : week <= today() ? "current" : "future";
          const ran = progress?.ranM ?? 0;
          const planned = progress?.plannedM ?? distance;
          const weekDate = (date: string) => dateValue(date).toLocaleDateString("en-GB", { day: "numeric", month: "short" });
          return <button key={week} className={`roadmap-row is-${state}`} aria-current={state === "current" ? "date" : undefined} onClick={() => { setCursor(week); setSelectedDay(state === "current" ? today() : week); navigate("training", "week"); }}><span><span className="roadmap-week">Week {index + 1}<em>{state === "past" ? "Done" : state === "current" ? "This week" : "Upcoming"}</em></span><small><time dateTime={week}>{weekDate(week)}</time> - <time dateTime={shiftDate(week, 6)}>{weekDate(shiftDate(week, 6))}</time>{downWeeks.data?.includes(week) ? " / Recovery" : ""}</small></span><span className="roadmap-track" role="img" aria-label={`${fmtKm(ran)} run of ${fmtKm(planned)} planned`}><i style={{ width: `${planned > 0 ? Math.min(100, ran / planned * 100) : 0}%` }} /></span><strong>{fmtKm(planned)}<small>planned</small></strong><strong>{fmtKm(ran)}<small>done</small></strong><IconArrowRight size={18} /></button>;
        })}</PlanRoadmap>
      </>}</section>
      {plan && <PlanAdjustments source="pacing" />}
      <details className="plan-basis" open={!compact}><summary>Plan details</summary><div className="plan-basis-content"><h3>Plan basis</h3>{plan ? <>
        <div className="data-table-scroll" tabIndex={0} role="region" aria-label="Plan basis"><table><caption className="sr-only">What the plan was built from</caption><tbody><tr><th scope="row">Target date</th><td>{fmtDateDMY(plan.targetDate)}</td></tr><tr><th scope="row">Starting volume</th><td>{plan.baselineWeeklyKm == null ? "From training history" : `${plan.baselineWeeklyKm.toFixed(1)} km/week`}</td></tr><tr><th scope="row">Progression</th><td>{capitalizeFirst(plan.increase)}</td></tr><tr><th scope="row">Watch</th><td>{plan.pushedId ? "Published" : "Not sent yet"}</td></tr></tbody></table></div>
        {receipt.data?.length ? <div className="data-table-scroll" tabIndex={0} role="region" aria-label="Plan receipt"><table><caption className="sr-only">How your answers shaped the plan</caption><thead><tr><th>Your input</th><th>What it did</th></tr></thead><tbody>{receipt.data.map((line, index) => <tr key={index}><th scope="row">{line.input}</th><td>{line.sentence}</td></tr>)}</tbody></table></div> : null}
        {defaults.data && <Predictions defaults={defaults.data} />}
        <RaceEstimates plan={plan} />
        {remotePlan.data && <div className="data-table-scroll" tabIndex={0} role="region" aria-label="Device plan"><table><caption className="sr-only">The plan on your other device</caption><tbody><tr><th scope="row">Device plan</th><td>{capitalizeFirst(({upToDate:"up to date",unpublished:"not yet published",updateAvailable:"update available",otherPlan:"different plan on the other device",importAvailable:"ready to import"})[remotePlan.data.status])} <a className="desk-button normal context-link" href="#settings/calendar">Manage device sync</a></td></tr></tbody></table></div>}
        <Button tone="danger" onClick={() => setCancel(true)}>End current plan</Button>
      </> : <p className="muted">Your recent mileage and threshold pace establish the starting load. Plan generation remains local.</p>}</div></details>
    </div> : <>
      <ErrorNotice error={events.error} retry={events.retry} />
      {events.loading ? <Loading label="Loading schedule" /> : <div className={`schedule-workspace ${selected ? "has-inspector" : ""}`}>
        <div className="schedule-canvas" ref={canvasRef}>
          <Change identity={first}>
          {horizon === "week" ? <>
            <SelectionGroup><div className="mobile-date-ribbon" role="region" tabIndex={0} aria-label="Days this week">{days.map(date => <button key={date} data-drop-day={date} onClick={() => selectDate(date)} aria-pressed={date === selectedDay} aria-label={`${dayName(date)} ${fmtDateDMY(date)}`}>{date === selectedDay && <SelectionMarker />}<span>{dayName(date)}</span><strong>{dateValue(date).getDate()}</strong><i className={(events.data ?? []).some(event => event.date === date) ? "has-session" : ""} /></button>)}</div></SelectionGroup>
            <div className="week-board">{days.map(date => <section className={`day-lane ${date === today() ? "is-today" : ""}`} data-drop-day={date} key={date} aria-label={`${dayName(date)} ${fmtDateDMY(date)}`}>
              <header><span>{dayName(date)}</span><strong>{dateValue(date).getDate()}</strong>{date === today() && <small>Today</small>}</header>
              <div className="day-sessions">{(events.data ?? []).filter(event => event.date === date).map(event => <SessionTile key={event.id} event={event} active={event.id === selected?.id} onSelect={() => { setSelected(event); setSelectedDay(event.date); }} />)}
              {!(events.data ?? []).some(event => event.date === date) && <p className="rest-label">Rest day</p>}</div>
            </section>)}</div>
          </> : <div className="month-board" role="region" tabIndex={0} aria-label="Month dates"><div className="month-labels">{["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map(day => <span key={day}>{day}</span>)}</div><div className="month-dates">{days.map(date => <div className={`month-day ${date.slice(0, 7) !== cursor.slice(0, 7) ? "outside-month" : ""}`} data-drop-day={date} key={date}>
            {compact ? <button type="button" className="month-number" data-date={date} onClick={() => selectDate(date)} aria-label={`${dayName(date)} ${fmtDateDMY(date)}`} aria-pressed={date === selectedDay} aria-current={date === today() ? "date" : undefined}>{dateValue(date).getDate()}<i aria-hidden="true">{(events.data ?? []).filter(event => event.date === date).length || ""}</i></button> : <time className="month-number" data-date={date} dateTime={date} aria-current={date === today() ? "date" : undefined}>{dateValue(date).getDate()}</time>}
            <div className="month-sessions">{(events.data ?? []).filter(event => event.date === date).map(event => <SessionTile key={event.id} event={event} active={selected?.id === event.id} onSelect={() => { setSelected(event); setSelectedDay(event.date); }} />)}</div>
          </div>)}</div></div>}
          <Change identity={selectedDay} className="day-itinerary"><h3>{selectedDay === today() ? "Today" : dayName(selectedDay)}, {shortDate(selectedDay)}</h3>{(events.data ?? []).filter(event => event.date === selectedDay).map(event => <SessionTile key={event.id} event={event} active={selected?.id === event.id} onSelect={() => setSelected(event)} />)}{!(events.data ?? []).some(event => event.date === selectedDay) && <p className="muted">No sessions scheduled. Time to recover.</p>}</Change>
          </Change>
        </div>
        <Presence>{selected && <Inspector key="session" title="Session" modal onClose={() => setSelected(null)}><Change identity={`${selected.id}:${selectedDay}`}><SessionDetail event={selected} plan={plan} proposedDate={selectedDay} onDone={() => setSelected(null)} /></Change></Inspector>}</Presence>
      </div>}
    </>}
    </Change>
    {compact && scheduleSummary}
    {compact && plan && horizon !== "plan" && <div className="training-actions training-secondary-actions"><Button onClick={() => setBenchmark(true)}><IconPlus size={18} />Schedule race or test</Button>{horizon === "week" && <Button onClick={() => setReset(true)}><IconRestore size={18} aria-hidden="true" />Reset week</Button>}</div>}
    <Presence>
    {benchmark && plan && <ScheduleBenchmark key="benchmark" plan={plan} initialDate={selectedDay} onClose={() => setBenchmark(false)} onScheduled={date => { setBenchmark(false); setCursor(date); setSelectedDay(date); setSelected(null); navigate("training", "week"); }} />}
    {reset && <ResetWeek key="reset" weekStart={first} onClose={() => setReset(false)} />}
    {create && <CreatePlan key="create" defaults={defaults.data} onClose={() => setCreate(false)} />}
    {cancel && <Dialog key="cancel" title="End current plan?" onClose={() => setCancel(false)}><p>This removes the current plan and its scheduled sessions. Other saved plans are kept. Recorded activities, health history and pain entries are not deleted.</p><p>Runalytics will attempt to cancel this plan on COROS and update your calendar and connected devices. To remove every saved plan instead, use Training data in Settings.</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setCancel(false)}>Keep plan</Button><Button tone="danger" busy={command.busy} onClick={() => void command.run(async () => {
      try { await tauri.abortPlan(); } catch (error) { refresh(); notify(String(error)); throw error; }
      setCancel(false);
    }, "Current plan ended. Your completed runs are retained.")}>End current plan</Button></div></Dialog>}
    </Presence>
  </>;
}

function PlanRoadmap({ identity, children }: { identity: string; children: ReactNode }) {
  const roadmap = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const list = roadmap.current!;
    const center = () => {
      const current = list.querySelector<HTMLElement>(".roadmap-row.is-current");
      if (!current) return;
      list.scrollTop = Math.max(0, current.offsetTop - (list.clientHeight - current.offsetHeight) / 2);
      const workspace = list.closest<HTMLElement>(".desk-workspace");
      if (workspace) {
        const bounds = current.getBoundingClientRect();
        const viewport = workspace.getBoundingClientRect();
        const style = getComputedStyle(workspace);
        const top = viewport.top + parseFloat(style.scrollPaddingTop);
        const bottom = viewport.bottom - parseFloat(style.scrollPaddingBottom);
        if (bounds.bottom > bottom) workspace.scrollTop += bounds.bottom - bottom;
        else if (bounds.top < top) workspace.scrollTop += bounds.top - top;
      }
    };
    const resize = new ResizeObserver(center);
    resize.observe(list);
    center();
    return () => resize.disconnect();
  }, [identity]);
  return <div className="plan-roadmap" ref={roadmap} role="region" aria-label="Plan weeks" tabIndex={0}>{children}</div>;
}

function SessionTile({ event, active, onSelect }: { event: CalendarEvent; active: boolean; onSelect: () => void }) {
  const movable = Boolean(event.workoutId && event.activityId == null && event.kind !== "race" && event.status !== "skipped");
  return <button className={`session-tile kind-${event.kind} ${movable ? "movable" : ""} ${active ? "selected" : ""}`} aria-pressed={active} onClick={() => {
    // A pointer-drag that just released ends with a click on this tile — on
    // touch, a synthetic one the browser fires itself. Either way the runner
    // moved the session; they did not ask to open it.
    if (performance.now() - dragEndedAt < 250) return;
    onSelect();
  }} data-workout-id={movable ? event.workoutId : undefined} data-session-date={event.date}>
    <span className="session-type"><span className="session-kind">{kindName(event.kind)}</span>{event.activityId != null && <IconCheck size={15} aria-label="Completed" />}{event.status === "skipped" && <IconX size={15} aria-label="Skipped" />}</span>
    <strong>{sessionTitle(event.title)}</strong><span>{event.distanceM != null ? fmtKm(event.distanceM) : fmtTimeRange(estimatedTimeRange(event)) ?? (event.durationMin != null ? `${Math.round(event.durationMin)} min` : "Recovery")}</span>
    {event.missed && <small>Missed</small>}{event.adjusted && <small>Adjusted</small>}{event.moved && <small>Rescheduled</small>}
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
  return <div className="session-detail"><span className="session-type">{kindName(event.kind)}</span><h2>{sessionTitle(event.title)}</h2><p className="muted">{fmtDateDMY(event.date)}{event.moved && event.plannedDate ? ` / Originally ${fmtDateDMY(event.plannedDate)}` : ""}</p>
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

/// Put the visible week back onto the plan's initial layout: sessions the
/// runner dragged elsewhere return to their generated days, and a race or test
/// scheduled over a planned session gives that session back (or disappears if
/// the plan had a rest day there). The list is the backend's own drift read, so
/// the dialog cannot promise a change the reset is going to refuse.
function ResetWeek({ weekStart, onClose }: { weekStart: string; onClose: () => void }) {
  const command = useCommand();
  const deviations = useRemote(() => tauri.weekDeviations(weekStart), [weekStart]);
  const restorable = (deviations.data ?? []).filter(deviation => deviation.restorable);
  const label = (deviation: WeekDeviation) => deviation.removes
    ? "Scheduled over a rest day — the reset removes it"
    : deviation.restoresContent && deviation.restoresDate
      ? `${dayName(deviation.to)} · back to its planned day and session`
      : deviation.restoresContent
        ? "Back to the session the plan generated"
        : `${dayName(deviation.to)} → ${dayName(deviation.from)}`;
  return <Dialog title="Reset this week" onClose={onClose}>
    <p className="context-note">Put the sessions back on the days your plan generated them, and undo a race or test scheduled over a planned session.</p>
    {deviations.loading ? <Loading label="Checking this week" /> : deviations.error ? <ErrorNotice error={deviations.error} retry={deviations.retry} /> : (deviations.data ?? []).length === 0
      ? <p className="muted">Nothing here has moved off the plan.</p>
      : <ul className="reset-list">{deviations.data!.map(deviation => <li key={deviation.workoutId} className={deviation.restorable ? "" : "not-restorable"}>
        <span className="reset-title">{sessionTitle(deviation.title)}</span>
        <span className="reset-days">{deviation.restorable ? label(deviation) : `${dayName(deviation.to)} · stays`}</span>
      </li>)}</ul>}
    <p className="context-note">Race day written by your plan, runs you have already done and sessions you skipped stay where they are.</p>
    <div className="full"><ErrorNotice error={command.error} /></div>
    <div className="form-actions full">
      <Button onClick={onClose}>Keep as it is</Button>
      <Button tone="primary" busy={command.busy} disabled={restorable.length === 0} onClick={() => void command.run(() => tauri.resetWeek(weekStart), restorable.length === 1 ? "One session is back on the plan." : `${restorable.length} sessions are back on the plan.`).then(ok => { if (ok) onClose(); })}>
        {restorable.length === 0 ? "Nothing to reset" : `Reset ${restorable.length}`}
      </Button>
    </div>
  </Dialog>;
}

function Predictions({ defaults }: { defaults: PlanDefaults }) {
  return <section className="watch-predictions"><h3>Watch predictions</h3><div className="data-table-scroll" tabIndex={0} role="region" aria-label="Watch predictions"><table><caption className="sr-only">Predicted race times from your current fitness</caption><tbody>{[["5 km", defaults.predict5kS], ["10 km", defaults.predict10kS], ["Half marathon", defaults.predictHalfS], ["Marathon", defaults.predictMarathonS]].map(([label, seconds]) => <tr key={label as string}><th scope="row">{label}</th><td>{typeof seconds === "number" ? fmtDuration(seconds) : "Not available"}</td></tr>)}</tbody></table></div></section>;
}

function CreatePlan({ defaults, onClose }: { defaults?: PlanDefaults; onClose: () => void }) {
  const command = useCommand();
  const form = useRef<HTMLFormElement>(null);
  const [step, setStep] = useState(0);
  const [kind, setKind] = useState<PlanKind>("race");
  const [trainingMethod, setTrainingMethod] = useState<NonNullable<PlanRequest["trainingMethod"]>>("norwegian");
  const [distance, setDistance] = useState<PlanRequest["distance"]>("21K");
  const [startDate, setStartDate] = useState(today());
  const [target, setTarget] = useState(shiftDate(today(), 70));
  const [weeks, setWeeks] = useState(10);
  const [threshold, setThreshold] = useState(defaults?.thresholdSecKm ? fmtPace(defaults.thresholdSecKm) : "");
  const [mileage, setMileage] = useState("");
  const [frequency, setFrequency] = useState(4);
  const [longDay, setLongDay] = useState(7);
  const [increase, setIncrease] = useState<PlanRequest["increase"]>("moderate");
  const [name, setName] = useState("");
  const [validation, setValidation] = useState<string | null>(null);
  const effectiveDate = kind === "maintenance" ? shiftDate(startDate || today(), weeks * 7) : target;
  const earliestEndDate = [shiftDate(startDate || today(), 15), shiftDate(today(), 1)].sort()[1];
  useEffect(() => { if (step > 0) form.current?.querySelector<HTMLElement>(step === 2 ? "h3" : "input,select")?.focus({ preventScroll: true }); }, [step]);
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); setValidation(null);
    if (!startDate || effectiveDate < earliestEndDate) { setValidation("Choose an end date at least 15 days after the start date and later than today."); return; }
    if (step === 1 && (parsePace(threshold) == null || parsePace(threshold)! <= 0)) { setValidation("Enter a threshold pace such as 4:45."); return; }
    if (step < 2) { setStep(step + 1); return; }
    const parsed = parsePace(threshold);
    if (parsed == null || parsed <= 0) { setValidation("Enter a threshold pace such as 4:45."); return; }
    if (await command.run(() => tauri.createPlan({ kind, trainingMethod, distance, startDate, targetDate: effectiveDate, thresholdSecKm: parsed, weeklyFrequency: frequency, longrunDay: longDay, increase, currentWeeklyKm: mileage ? Number(mileage) : null, name: name.trim() || null }), "Your plan is ready.")) { navigate("training", "plan"); onClose(); }
  }
  return <Dialog title="Create training plan" wide onClose={onClose}><div className="plan-composer"><form ref={form} onSubmit={submit}><div className="composer-progress" aria-label={`Step ${step + 1} of 3`}>{["Direction", "Your baseline", "Review"].map((label, index) => <span key={label} aria-current={index === step ? "step" : undefined}>{index + 1}. {label}</span>)}</div>
    <ErrorNotice error={validation ?? command.error} />
    <Change identity={step}>
    {step === 0 && <div className="form-grid"><fieldset className="full"><legend>What are you training for?</legend>{([ ["race", "A race"], ["improvement", "Improve my fitness"], ["maintenance", "Stay consistent"] ] as const).map(([value, label]) => <label key={value}><input type="radio" name="plan-kind" value={value} checked={kind === value} onChange={() => setKind(value)} />{label}</label>)}</fieldset>
      <fieldset className="full"><legend>Training method</legend>
        <label><input type="radio" name="training-method" value="norwegian" checked={trainingMethod === "norwegian"} aria-describedby="norwegian-description" onChange={() => setTrainingMethod("norwegian")} />Norwegian method</label>
        <p id="norwegian-description" className="muted">Controlled sub-threshold repeats, easy mileage and gradual progression, with quality work limited by your recovery.</p>
        <label><input type="radio" name="training-method" value="runna_like" checked={trainingMethod === "runna_like"} aria-describedby="runna-description" onChange={() => setTrainingMethod("runna_like")} />Runna-like</label>
        <p id="runna-description" className="muted">A varied mix of intervals, tempo and long runs, with recovery weeks and a race taper. Inspired by Runna's public guidance, not its proprietary algorithm.</p>
      </fieldset>
      <label>Distance<select value={distance} onChange={event => setDistance(event.target.value as PlanRequest["distance"])}><option value="5K">5 km</option><option value="10K">10 km</option><option value="21K">Half marathon</option><option value="42K">Marathon</option></select></label>
      <label>Start date<input type="date" required value={startDate} onChange={event => setStartDate(event.target.value)} /></label>
      {kind === "maintenance" ? <label>Block length<select value={weeks} onChange={event => setWeeks(Number(event.target.value))}>{[4, 6, 8, 10, 12, 16].map(value => <option key={value} value={value}>{value} weeks</option>)}</select></label> : <label>{kind === "race" ? "Race date" : "Time trial date"}<input type="date" required min={earliestEndDate} value={target} onChange={event => setTarget(event.target.value)} /></label>}
      <label className="full">Plan name (optional)<input value={name} maxLength={60} placeholder="Spring half build" onChange={event => setName(event.target.value)} /><span className="muted">How Runalytics calls this plan. Your watch keeps one Runalytics plan slot and shows the name inside its description.</span></label>
    </div>}
    {step === 1 && <div className="form-grid"><label>Threshold pace (min/km)<input required value={threshold} placeholder="4:45" pattern="[0-9]{1,2}:[0-5][0-9]" onChange={event => setThreshold(event.target.value)} /></label><label>Current weekly mileage (km)<input type="number" min="5" max="250" step="0.1" value={mileage} placeholder={defaults?.currentWeeklyKm ? `Leave empty for your synced average: ${defaults.currentWeeklyKm.toFixed(1)}` : "Leave empty to use your synced history"} onChange={event => setMileage(event.target.value)} /><span className="muted">Only fill this in if your recent runs do not reflect a normal week.</span></label><label>Runs per week<input type="number" min="3" max="7" required value={frequency} onChange={event => setFrequency(Number(event.target.value))} /></label><label>Long run day<select value={longDay} onChange={event => setLongDay(Number(event.target.value))}>{["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"].map((day, index) => <option key={day} value={index + 1}>{day}</option>)}</select></label>{kind !== "maintenance" && <label className="full">Mileage progression<select value={increase} onChange={event => setIncrease(event.target.value as PlanRequest["increase"])}><option value="conservative">Conservative / 5%</option><option value="moderate">Moderate / 8%</option><option value="ambitious">Ambitious / 12%</option></select></label>}</div>}
    {step === 2 && <><h3 tabIndex={-1}>Ready to build your plan</h3><p>Your schedule is generated locally from these inputs. Sending it to your watch is a separate action.</p>{defaults?.activePain && <p className="context-note">Unresolved pain: {defaults.activePain.bodyPart}, {defaults.activePain.intensity}/10. Consider reviewing it in Health before starting a new plan.</p>}</>}
    </Change>
    <div className="form-actions">{step > 0 && <Button onClick={() => setStep(step - 1)}>Back</Button>}<Button tone="primary" type="submit" busy={command.busy}>{step === 2 ? "Build plan" : "Continue"}</Button></div>
    </form><aside className="composer-summary"><h3>Your training brief</h3><dl className="definition-list"><div><dt>Goal</dt><dd>{kind}</dd></div><div><dt>Training method</dt><dd>{trainingMethod === "runna_like" ? "Runna-like" : "Norwegian method"}</dd></div><div><dt>Distance</dt><dd>{distance}</dd></div><div><dt>Start date</dt><dd>{startDate ? fmtDateDMY(startDate) : "Not entered"}</dd></div><div><dt>End date</dt><dd>{fmtDateDMY(effectiveDate)}</dd></div><div><dt>Frequency</dt><dd>{frequency} runs/week</dd></div><div><dt>Starting volume</dt><dd>{mileage ? `${mileage} km/week` : "From synced history"}</dd></div><div><dt>Threshold</dt><dd>{threshold || "Not entered"}</dd></div></dl>{defaults?.weeksObserved ? <p className="muted">Baseline from {defaults.weeksObserved} complete weeks of training.</p> : null}</aside></div></Dialog>;
}
