import { t, text, number, formatLocale, date as fmtDateDMY, duration as fmtDuration, distance as fmtKm, timeRange as fmtTimeRange } from "./i18n";
import { useEffect, useLayoutEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import { IconArrowLeft, IconArrowRight, IconPlus, IconCopy, IconCalendarEvent, IconCheck, IconX, IconRestore, IconChevronDown } from "@tabler/icons-react";
import { tauri, fmtPace, parsePace, estimatedTimeRange } from "../lib/ipc";
import type { CalendarEvent, PlanDefaults, PlanKind, PlanRequest, TrainingPlan, WeekDeviation } from "../lib/types";
import { planName as displayPlanName } from "./i18n";
import { capitalizeFirst, dateValue, dayName, kindName, monday, navigate, sessionTitle, shiftDate, shortDate, today, useCommand, useRemote, useRoute, useWorkspace } from "./data";
import { Button, CountUp, Dialog, Empty, ErrorNotice, IconButton, Inspector, Loading, Metric } from "./ui";
import { EditPlan, SessionFeedback } from "./feedback";
import { Change, Disclosure, Presence, SelectionGroup, SelectionMarker, useCompactLayout } from "./interaction";

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
  const sessionOrigin = useRef<HTMLButtonElement | null>(null);
  const [create, setCreate] = useState(false);
  const [benchmark, setBenchmark] = useState(false);
  const [cancel, setCancel] = useState(false);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const cancelPlanId = useRef<string | null>(null);
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
    const firstOfMonth = dateValue(`${cursor.slice(0, 7)}-01`);
    const lead = firstOfMonth.getDay() === 0 ? 6 : firstOfMonth.getDay() - 1;
    const daysInMonth = new Date(firstOfMonth.getFullYear(), firstOfMonth.getMonth() + 1, 0).getDate();
    return Math.ceil((lead + daysInMonth) / 7);
  })();
  const days = Array.from({ length: horizon === "month" ? monthWeeks * 7 : 7 }, (_, index) => shiftDate(first, index));
  const scheduleFirst = monday(`${cursor.slice(0, 7)}-01`);
  const scheduleLast = shiftDate(scheduleFirst, monthWeeks * 7 - 1);
  const events = useRemote(() => tauri.calendarEvents(scheduleFirst, scheduleLast), [scheduleFirst, scheduleLast]);
  const selectedEvent = events.data?.find(event => event.id === selected?.id);
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
      () => { notify(t("Plan sent to COROS.")); refresh(); },
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
            notify(t("Moved to {{value0}}.", { value0: fmtDateDMY(date) }), { label: t("Undo"), run: () => tauri.moveWorkout(workoutId, from!) });
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
  const shown = (events.data ?? []).filter(event => event.date >= first && event.date <= days[days.length - 1]);
  const done = shown.filter(event => event.activityId != null);
  const visibleWeeks = new Set(days.map(date => monday(date)));
  const plannedM = (weekProgress.data ?? []).filter(entry => visibleWeeks.has(entry.weekStart)).reduce((total, entry) => total + entry.plannedM, 0);
  const ranM = done.reduce((total, event) => total + (event.distanceM ?? 0), 0);
  const scheduleSummary = horizon !== "plan" && events.data && <div className="schedule-summary"><CountUp format={progress => t("{{value0}}/{{value1}} sessions done", { value0: number(done.length * progress), value1: number(shown.length * progress) })} /><CountUp format={progress => t("{{value0}} / {{value1}} done", { value0: fmtKm(ranM * progress), value1: fmtKm(plannedM * progress) })} />{!compact && horizon === "week" && plan && <Button onClick={() => setReset(true)}><IconRestore size={18} aria-hidden="true" />{t("Reset week")}</Button>}</div>;
  return <>
    <div className="training-context">
      <div><span className="muted">{plan ? displayPlanName(plan) : t("Your training")}</span>
        <h2>{horizon === "plan" ? plan ? t("Building toward {{value0}}", { value0: shortDate(plan.targetDate) }) : t("A plan that fits your life") : horizon === "month" ? dateValue(cursor).toLocaleDateString(formatLocale, { month: "long", year: "numeric" }) : compact ? `${dateValue(first).toLocaleDateString(formatLocale, { day: "numeric", month: "short" })} - ${dateValue(shiftDate(first, 6)).toLocaleDateString(formatLocale, { day: "numeric", month: "short", year: "numeric" })}` : `${shortDate(first)} - ${shortDate(shiftDate(first, 6))}`}</h2></div>
      {(!compact || !plan) && <div className="training-actions">{plan ? <Button onClick={() => setBenchmark(true)}><IconPlus size={18} />{t("Schedule race or test")}</Button> : <Button tone="primary" onClick={() => setCreate(true)} disabled={current.loading || Boolean(current.error) || !connected || !synced}><IconPlus size={18} />{t("Create plan")}</Button>}</div>}
    </div>
    {syncingPlan && planProgress && <div className="native-progress" role="status"><span>{text(planProgress.stage)}</span><progress max={100} value={planProgress.percent} aria-label={t("Watch publishing progress")} /></div>}
    <ErrorNotice error={command.error} />
    <ErrorNotice error={current.error} retry={current.retry} />
    {!providers.loading && !connected && <div className="setup-band"><div><strong>{t("Connect your watch")}</strong><p>{t("Bring your runs and recovery data into your training.")}</p></div><Button tone="primary" onClick={() => navigate("settings", "setup")}>{t("Connect COROS")}</Button></div>}
    {connected && !synced && <div className="setup-band"><div><strong>{t("Your watch is connected")}</strong><p>{t("Sync your training history to establish your starting point.")}</p></div><Button busy={command.busy} tone="primary" onClick={() => void command.run(() => tauri.syncNow("coros"), t("Training history synced."))}>{t("Sync now")}</Button></div>}
    <div className="desk-toolbar training-toolbar">
      <SelectionGroup><nav className="horizon-switch" aria-label={t("Training horizon")}>{(["week", "month", "plan"] as const).map(value => <a key={value} href={`#training/${value}`} aria-current={horizon === value ? "page" : undefined}>{horizon === value && <SelectionMarker />}{t(value[0].toUpperCase() + value.slice(1))}</a>)}</nav></SelectionGroup>
      {compact && horizon === "plan" && plan && <Button aria-label={t("Schedule race or test")} onClick={() => setBenchmark(true)}><IconPlus size={18} aria-hidden="true" />{t("Race or test")}</Button>}
      {horizon !== "plan" && <div className="date-controls"><IconButton label={horizon === "month" ? t("Previous month") : t("Previous week")} onClick={() => moveHorizon(-1)}><IconArrowLeft size={18} /></IconButton><Button onClick={() => { setCursor(today()); setSelectedDay(today()); }}>{t("Today")}</Button><IconButton label={horizon === "month" ? t("Next month") : t("Next week")} onClick={() => moveHorizon(1)}><IconArrowRight size={18} /></IconButton></div>}
    </div>
    {!compact && scheduleSummary}
    <Change identity={horizon}>
    {horizon === "plan" ? <div className="plan-studio">
      <div className="plan-main"><section className="plan-overview"><h3>{t("Training horizon")}</h3>{current.loading ? <Loading /> : !plan ? <Empty title={t("Give your training a direction")} action={<Button tone="primary" disabled={!connected || !synced || Boolean(current.error)} onClick={() => setCreate(true)}>{t("Create plan")}</Button>}>{t("Prepare for a race, build fitness or maintain your routine.")}</Empty> : <>
        <dl className="metrics-line"><Metric label={t("Training method")} value={plan.trainingMethod === "runna_like" ? "Runna-like" : "Norwegian method"} /><Metric label={t("Sessions complete")} value={<CountUp format={progress => `${number(completed * progress)}/${number(workouts.length)}`} />} /><Metric label={t("Runs per week")} value={plan.weeklyFrequency} /><Metric label={t("Threshold pace")} value={fmtPace(plan.thresholdSecKm)} count={{ value: plan.thresholdSecKm, format: fmtPace }} unit="/km" /></dl>
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
          const weekDate = (date: string) => dateValue(date).toLocaleDateString(formatLocale, { day: "numeric", month: "short" });
          return <button key={week} className={`roadmap-row is-${state}`} aria-current={state === "current" ? "date" : undefined} onClick={() => { setCursor(week); setSelectedDay(state === "current" ? today() : week); navigate("training", "week"); }}><span><span className="roadmap-week">{t("Week {{value0}}", { value0: index + 1 })}<em>{state === "past" ? t("Done") : state === "current" ? t("This week") : t("Upcoming")}</em></span><small><time dateTime={week}>{weekDate(week)}</time> - <time dateTime={shiftDate(week, 6)}>{weekDate(shiftDate(week, 6))}</time>{downWeeks.data?.includes(week) ? t(" / Recovery") : ""}</small></span><span className="roadmap-track" role="img" aria-label={t("{{value0}} run of {{value1}} planned", { value0: fmtKm(ran), value1: fmtKm(planned) })}><i style={{ width: `${planned > 0 ? Math.min(100, ran / planned * 100) : 0}%` }} /></span><strong>{fmtKm(planned)}<small>{t("planned")}</small></strong><strong>{fmtKm(ran)}<small>{t("done")}</small></strong><IconArrowRight size={18} /></button>;
        })}</PlanRoadmap>
      </>}</section>
      {plan && <EditPlan key={plan.id} planId={plan.id} />}</div>
      <section className="plan-basis" aria-labelledby="plan-details-heading"><header className="plan-basis-heading"><h3 id="plan-details-heading">{t("Plan details")}</h3>{compact && <Button aria-expanded={detailsOpen} aria-controls="plan-details" onClick={() => setDetailsOpen(open => !open)}>{detailsOpen ? t("Hide details") : t("Show details")}<IconChevronDown className="disclosure-indicator" size={18} aria-hidden="true" /></Button>}</header><Disclosure id="plan-details" className="plan-basis-content" open={!compact || detailsOpen}><h3>{t("Plan basis")}</h3>{plan ? <>
        <div className="data-table-scroll" tabIndex={0} role="region" aria-label={t("Plan basis")}><table><caption className="sr-only">{t("What the plan was built from")}</caption><tbody><tr><th scope="row">{t("Target date")}</th><td>{fmtDateDMY(plan.targetDate)}</td></tr><tr><th scope="row">{t("Starting volume")}</th><td>{plan.baselineWeeklyKm == null ? t("From training history") : t("{{value0}} km/week", { value0: number(plan.baselineWeeklyKm, 1) })}</td></tr><tr><th scope="row">{t("Progression")}</th><td>{capitalizeFirst(plan.increase)}</td></tr><tr><th scope="row">{t("Watch")}</th><td>{plan.pushedId ? t("Published") : t("Not sent yet")}</td></tr></tbody></table></div>
        {receipt.data?.length ? <div className="data-table-scroll" tabIndex={0} role="region" aria-label={t("Plan receipt")}><table><caption className="sr-only">{t("How your answers shaped the plan")}</caption><thead><tr><th>{t("Your input")}</th><th>{t("What it did")}</th></tr></thead><tbody>{receipt.data.map((line, index) => <tr key={index}><th scope="row">{text(line.input)}</th><td>{text(line.sentence)}</td></tr>)}</tbody></table></div> : null}
        {remotePlan.data && <><div className="data-table-scroll" tabIndex={0} role="region" aria-label={t("Device plan")}><table><caption className="sr-only">{t("The plan on your other device")}</caption><tbody><tr><th scope="row">{t("Device plan")}</th><td>{capitalizeFirst(({upToDate:t("up to date"),unpublished:t("not yet published"),updateAvailable:t("update available"),otherPlan:t("different plan on the other device"),importAvailable:t("ready to import")})[remotePlan.data.status])}</td></tr></tbody></table></div><a className="desk-button normal context-link" href="#settings/devices">{t("Manage device sync")}</a></>}
        <Button tone="danger" onClick={() => { cancelPlanId.current = plan.id; setCancel(true); }}>{t("End current plan")}</Button>
      </> : <p className="muted">{t("Your recent mileage and threshold pace establish the starting load. Plan generation remains local.")}</p>}</Disclosure></section>
    </div> : <>
      <ErrorNotice error={events.error} retry={events.retry} />
      {events.loading ? <Loading label={t("Loading schedule")} /> : <div className={`schedule-workspace ${selected ? "has-inspector" : ""}`}>
        <div className="schedule-canvas" ref={canvasRef}>
          <Change identity={first}>
          {horizon === "week" ? <>
            <SelectionGroup><div className="mobile-date-ribbon" role="region" tabIndex={0} aria-label={t("Days this week")}>{days.map(date => <button key={date} data-drop-day={date} onClick={() => selectDate(date)} aria-pressed={date === selectedDay} aria-label={`${dayName(date)} ${fmtDateDMY(date)}`}>{date === selectedDay && <SelectionMarker />}<span>{dayName(date)}</span><strong>{dateValue(date).getDate()}</strong><i className={(events.data ?? []).some(event => event.date === date) ? "has-session" : ""} /></button>)}</div></SelectionGroup>
            <div className="week-board">{days.map(date => <section className={`day-lane ${date === today() ? "is-today" : ""}`} data-drop-day={date} key={date} aria-label={`${dayName(date)} ${fmtDateDMY(date)}`}>
              <header><span>{dayName(date)}</span><strong>{dateValue(date).getDate()}</strong>{date === today() && <small>{t("Today")}</small>}</header>
              <div className="day-sessions">{(events.data ?? []).filter(event => event.date === date).map(event => <SessionTile key={event.id} event={event} active={event.id === selected?.id} onSelect={origin => { sessionOrigin.current = origin; setSelected(event); setSelectedDay(event.date); }} />)}
              {!(events.data ?? []).some(event => event.date === date) && <p className="rest-label" data-entrance>{t("Rest day")}</p>}</div>
            </section>)}</div>
          </> : <div className="month-board" role="region" tabIndex={0} aria-label={t("Month dates")}><div className="month-labels">{[t("Mon"), t("Tue"), t("Wed"), t("Thu"), t("Fri"), t("Sat"), t("Sun")].map(day => <span key={day}>{day}</span>)}</div><div className="month-dates">{days.map(date => <div className={`month-day ${date.slice(0, 7) !== cursor.slice(0, 7) ? "outside-month" : ""}`} data-drop-day={date} key={date}>
            {compact ? <button type="button" className="desk-button month-number" data-date={date} onClick={() => selectDate(date)} aria-label={`${dayName(date)} ${fmtDateDMY(date)}`} aria-pressed={date === selectedDay} aria-current={date === today() ? "date" : undefined}>{dateValue(date).getDate()}<i aria-hidden="true">{(events.data ?? []).filter(event => event.date === date).length || ""}</i></button> : <time className="month-number" data-date={date} dateTime={date} aria-current={date === today() ? "date" : undefined}>{dateValue(date).getDate()}</time>}
            <div className="month-sessions">{(events.data ?? []).filter(event => event.date === date).map(event => <SessionTile key={event.id} event={event} active={selected?.id === event.id} onSelect={origin => { sessionOrigin.current = origin; setSelected(event); setSelectedDay(event.date); }} />)}</div>
          </div>)}</div></div>}
          <Change identity={selectedDay} className="day-itinerary"><h3>{selectedDay === today() ? t("Today") : dayName(selectedDay)}, {shortDate(selectedDay)}</h3>{(events.data ?? []).filter(event => event.date === selectedDay).map(event => <SessionTile key={event.id} event={event} active={selected?.id === event.id} onSelect={origin => { sessionOrigin.current = origin; setSelected(event); }} />)}{!(events.data ?? []).some(event => event.date === selectedDay) && <p className="muted">{t("No sessions scheduled. Time to recover.")}</p>}</Change>
          </Change>
        </div>
        <Presence>{selected && <Inspector key="session" title={t("Session")} modal origin={sessionOrigin.current} onClose={() => setSelected(null)}><Change identity={`${selected.id}:${selectedDay}`}><SessionDetail event={selectedEvent ?? selected} stale={events.data !== undefined && !selectedEvent} plan={plan} proposedDate={selectedDay} onDone={() => setSelected(null)} /></Change></Inspector>}</Presence>
      </div>}
    </>}
    </Change>
    {compact && scheduleSummary}
    {compact && plan && horizon !== "plan" && <div className="training-actions training-secondary-actions"><Button onClick={() => setBenchmark(true)}><IconPlus size={18} />{t("Schedule race or test")}</Button>{horizon === "week" && <Button onClick={() => setReset(true)}><IconRestore size={18} aria-hidden="true" />{t("Reset week")}</Button>}</div>}
    <Presence>
    {benchmark && plan && <ScheduleBenchmark key="benchmark" plan={plan} initialDate={selectedDay} onClose={() => setBenchmark(false)} onScheduled={date => { setBenchmark(false); setCursor(date); setSelectedDay(date); setSelected(null); navigate("training", "week"); }} />}
    {reset && <ResetWeek key="reset" planId={plan?.id} weekStart={first} onClose={() => setReset(false)} />}
    {create && <CreatePlan key="create" defaults={defaults.data} onClose={() => setCreate(false)} />}
    {cancel && <Dialog key="cancel" title={t("End current plan?")} onClose={() => setCancel(false)}><p>{t("This removes the current plan and its scheduled sessions. Other saved plans are kept. Recorded activities, health history and pain entries are not deleted.")}</p><p>{t("Runalytics will attempt to cancel this plan on COROS and update your calendar and connected devices. To remove every saved plan instead, use Training data in Settings.")}</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setCancel(false)}>{t("Keep plan")}</Button><Button tone="danger" busy={command.busy} onClick={() => void command.run(async () => {
      try { await tauri.abortPlan(cancelPlanId.current ?? undefined); } catch (error) { refresh(); notify(String(error)); throw error; }
      setCancel(false);
    }, t("Current plan ended. Your completed runs are retained."))}>{t("End current plan")}</Button></div></Dialog>}
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
  return <div className="plan-roadmap" ref={roadmap} role="region" aria-label={t("Plan weeks")} tabIndex={0}>{children}</div>;
}

function SessionTile({ event, active, onSelect }: { event: CalendarEvent; active: boolean; onSelect: (origin: HTMLButtonElement) => void }) {
  const movable = Boolean(event.workoutId && event.activityId == null && event.kind !== "race" && event.status !== "skipped");
  return <button className={`session-tile kind-${event.kind} ${movable ? "movable" : ""} ${active ? "selected" : ""}`} aria-pressed={active} onClick={click => {
    // A pointer-drag that just released ends with a click on this tile — on
    // touch, a synthetic one the browser fires itself. Either way the runner
    // moved the session; they did not ask to open it.
    if (performance.now() - dragEndedAt < 250) return;
    onSelect(click.currentTarget);
  }} data-workout-id={movable ? event.workoutId : undefined} data-session-date={event.date}>
    <span className="session-type"><span className="session-kind">{kindName(event.kind)}</span>{event.activityId != null && <IconCheck size={15} aria-label={t("Completed")} />}{event.status === "skipped" && <IconX size={15} aria-label={t("Skipped")} />}</span>
    <strong>{sessionTitle(event.title)}</strong><span>{event.distanceM != null ? fmtKm(event.distanceM) : fmtTimeRange(estimatedTimeRange(event)) ?? (event.durationMin != null ? `${Math.round(event.durationMin)} min` : t("Recovery"))}</span>
    {event.missed && <small>{t("Missed")}</small>}{event.adjusted && <small>{t("Adjusted")}</small>}{event.moved && <small>{t("Rescheduled")}</small>}
  </button>;
}

export function SessionDetail({ event, plan, proposedDate, stale = false, onDone }: { event: CalendarEvent; plan?: TrainingPlan; proposedDate?: string; stale?: boolean; onDone: () => void }) {
  const command = useCommand();
  const { notify } = useWorkspace();
  const [date, setDate] = useState(proposedDate ?? event.date);
  const [match, setMatch] = useState("");
  const candidates = useRemote(() => event.workoutId && event.activityId == null ? tauri.calendarEvents(shiftDate(event.date, -7), shiftDate(event.date, 7)) : Promise.resolve([]), [event.id]);
  const review = useRemote(() => event.activityId != null ? tauri.activityReview(event.activityId) : Promise.resolve(null), [event.activityId]);
  const editable = event.workoutId && event.activityId == null && event.kind !== "race";
  return <div className="session-detail"><span className="session-type">{kindName(event.kind)}</span><h2>{sessionTitle(event.title)}</h2><p className="muted">{fmtDateDMY(event.date)}{event.moved && event.plannedDate ? t(" / Originally {{value0}}", { value0: fmtDateDMY(event.plannedDate) }) : ""}</p>
    {stale && <p role="alert">{t("This session is no longer in the latest plan. Your draft is kept, but this session cannot be changed.")}</p>}
    {event.adjusted && <p className="context-note">{t("Adjusted for you: {{value0}}", { value0: text(event.adjusted) })}</p>}
    <dl className="metrics-line"><Metric label={t("Distance")} value={event.distanceM != null ? number((event.distanceM / 1000), 1) : "Not specified"} count={event.distanceM != null ? { value: event.distanceM / 1000, format: value => number(value, 1) } : undefined} unit={event.distanceM != null ? "km" : undefined} />{event.activityId != null ? event.durationMin != null && <Metric label={t("Duration")} value={Math.round(event.durationMin)} unit="min" /> : fmtTimeRange(estimatedTimeRange(event)) && <Metric label={t("Estimated time")} value={fmtTimeRange(estimatedTimeRange(event))} />}</dl>
    <div className="prescription">{event.notes.split(/\n\s*\n/).filter(Boolean).map((paragraph, index) => <p key={index}>{text(paragraph)}</p>)}</div>
    {event.workoutId && <SessionFeedback workoutId={event.workoutId} />}
    <Button onClick={() => void command.run(() => navigator.clipboard.writeText(event.notes), t("Session copied."))}><IconCopy size={18} />{t("Copy prescription")}</Button>
    {review.data && <section className="session-recording"><h3>{t("Recorded run")}</h3><p>{t("{{value0}} at {{value1}}/km", { value0: fmtDuration(review.data.activity.durationS), value1: fmtPace(review.data.activity.avgPaceSKm) })}</p><Button onClick={() => navigate("journal", String(event.activityId))}>{t("Open activity review")}</Button></section>}
    <ErrorNotice error={command.error ?? review.error} />
    {editable && <section className="session-management"><h3>{t("Schedule")}</h3><form onSubmit={async submit => {
      submit.preventDefault();
      if (stale) return;
      if (await command.run(() => tauri.moveWorkout(event.workoutId!, date))) {
        notify(t("Moved to {{value0}}.", { value0: fmtDateDMY(date) }), { label: t("Undo"), run: () => tauri.moveWorkout(event.workoutId!, event.date) }); onDone();
      }
    }}><label>{t("Run on")}<input type="date" required min={today()} max={plan?.targetDate} value={date} onChange={change => setDate(change.target.value)} /></label><Button type="submit" busy={command.busy} disabled={stale || date === event.date || event.status === "skipped"}><IconCalendarEvent size={18} />{t("Move session")}</Button></form>
      <Button disabled={stale} busy={command.busy} onClick={async () => {
        const skipped = event.status !== "skipped";
        if (await command.run(() => tauri.setWorkoutSkipped(event.workoutId!, skipped))) {
          notify(skipped ? t("Session skipped. It will not count as missed.") : t("Session restored."), { label: t("Undo"), run: () => tauri.setWorkoutSkipped(event.workoutId!, !skipped) }); onDone();
        }
      }}>{event.status === "skipped" ? t("Restore session") : t("Skip session")}</Button>
    </section>}
    {event.workoutId && event.activityId == null && <section className="session-management"><h3>{t("Already ran this session?")}</h3><ErrorNotice error={candidates.error} retry={candidates.retry} /><form onSubmit={async submit => {
      submit.preventDefault();
      if (stale) return;
      if (await command.run(() => tauri.matchCalendarActivity(event.workoutId!, Number(match)), t("Recorded run matched to this session."))) onDone();
    }}><label>{t("Recorded activity")}<select required value={match} onChange={change => setMatch(change.target.value)}><option value="">{t("Choose a nearby run")}</option>{candidates.data?.filter(candidate => candidate.activityId != null && candidate.workoutId == null).map(candidate => <option key={candidate.id} value={candidate.activityId!}>{fmtDateDMY(candidate.date)} / {fmtKm(candidate.distanceM ?? 0)}</option>)}</select></label><Button type="submit" disabled={stale || !match} busy={command.busy}>{t("Match run")}</Button></form></section>}
  </div>;
}

function ScheduleBenchmark({ plan, initialDate, onClose, onScheduled }: { plan: TrainingPlan; initialDate: string; onClose: () => void; onScheduled: (date: string) => void }) {
  const command = useCommand();
  const [draftPlanId] = useState(plan.id);
  const stale = draftPlanId !== plan.id;
  const earliest = [shiftDate(today(), 1), shiftDate(plan.createdAt.slice(0, 10), 1)].sort()[1];
  const latest = shiftDate(plan.targetDate, -8);
  const [preset, setPreset] = useState("parkrun");
  const [name, setName] = useState("parkrun");
  const [distance, setDistance] = useState(5);
  const [reduceMileage, setReduceMileage] = useState(true);
  const [date, setDate] = useState(initialDate >= earliest && initialDate <= latest ? initialDate : earliest);
  return <Dialog title={t("Schedule race or test")} onClose={onClose}><form className="form-grid" onSubmit={async submit => {
    submit.preventDefault();
    if (stale) return;
    if (await command.run(() => tauri.scheduleBenchmark(date, name, distance * 1000, draftPlanId, reduceMileage), t("Benchmark scheduled. Nearby hard sessions have been eased."))) onScheduled(date);
  }}>
    <label className="full">{t("Event")}<select value={preset} onChange={change => {
      const value = change.target.value; setPreset(value);
      setName(value === "parkrun" ? "parkrun" : value === "race" ? t("Tune-up race") : t("Time trial"));
      if (value === "parkrun") setDistance(5);
    }}><option value="parkrun">{t("parkrun / 5 km")}</option><option value="race">{t("Tune-up race")}</option><option value="trial">{t("Time trial")}</option></select></label>
    <label className="full">{t("Event name")}<input required maxLength={80} value={name} onChange={change => setName(change.target.value)} /></label>
    <label>{t("Date")}<input type="date" required min={earliest} max={latest} value={date} onChange={change => setDate(change.target.value)} /></label>
    <label>{t("Distance (km)")}<input type="number" required min={1} max={10} step={0.1} disabled={preset === "parkrun"} value={distance} onChange={change => setDistance(Number(change.target.value))} /></label>
    <label className="toggle-setting full"><input type="checkbox" checked={reduceMileage} onChange={change => setReduceMileage(change.target.checked)} />{t("Reduce this week's planned mileage by 20%")}</label>
    <p className="context-note full">{t("Replaces that day's session and eases adjacent hard or long runs. Your target race stays unchanged. Run this as a hard effort; match the race-only recording afterwards for a pace review.")}</p>
    {earliest > latest && <p className="context-note full">{t("There is no room for a benchmark before the final week of this plan.")}</p>}
    <div className="full"><ErrorNotice error={command.error} /></div>
    {stale && <p role="alert" className="full">{t("The plan changed on another device. Close this draft and review the latest plan before scheduling.")}</p>}
    <div className="form-actions full"><Button onClick={onClose}>{t("Cancel")}</Button><Button type="submit" tone="primary" busy={command.busy} disabled={stale || earliest > latest || !name.trim()}><IconCalendarEvent size={18} />{t("Schedule benchmark")}</Button></div>
  </form></Dialog>;
}

/// Put the visible week back onto the plan's initial layout: sessions the
/// runner dragged elsewhere return to their generated days, and a race or test
/// scheduled over a planned session gives that session back (or disappears if
/// the plan had a rest day there). The list is the backend's own drift read, so
/// the dialog cannot promise a change the reset is going to refuse.
function ResetWeek({ weekStart, planId, onClose }: { weekStart: string; planId?: string; onClose: () => void }) {
  const command = useCommand();
  const [draftPlanId] = useState(planId);
  const stale = draftPlanId !== planId;
  const deviations = useRemote(() => tauri.weekDeviations(weekStart), [weekStart]);
  const restorable = (deviations.data ?? []).filter(deviation => deviation.restorable);
  const label = (deviation: WeekDeviation) => deviation.removes
    ? t("Scheduled over a rest day — the reset removes it")
    : deviation.restoresContent && deviation.restoresDate
      ? t("{{value0}} · back to its planned day and session", { value0: dayName(deviation.to) })
      : deviation.restoresContent
        ? t("Back to the session the plan generated")
        : `${dayName(deviation.to)} → ${dayName(deviation.from)}`;
  return <Dialog title={t("Reset this week")} onClose={onClose}>
    <p className="context-note">{t("Put the sessions back on the days your plan generated them, and undo a race or test scheduled over a planned session.")}</p>
    {deviations.loading ? <Loading label={t("Checking this week")} /> : deviations.error ? <ErrorNotice error={deviations.error} retry={deviations.retry} /> : (deviations.data ?? []).length === 0
      ? <p className="muted">{t("Nothing here has moved off the plan.")}</p>
      : <ul className="reset-list">{deviations.data!.map(deviation => <li key={deviation.workoutId} className={deviation.restorable ? "" : "not-restorable"}>
        <span className="reset-title">{sessionTitle(deviation.title)}</span>
        <span className="reset-days">{deviation.restorable ? label(deviation) : t("{{value0}} · stays", { value0: dayName(deviation.to) })}</span>
      </li>)}</ul>}
    <p className="context-note">{t("Race day written by your plan, runs you have already done and sessions you skipped stay where they are.")}</p>
    <div className="full"><ErrorNotice error={command.error} /></div>
    {stale && <p role="alert">{t("The plan changed on another device. Close this preview and review the latest plan.")}</p>}
    <div className="form-actions full">
      <Button onClick={onClose}>{t("Keep as it is")}</Button>
      <Button tone="primary" busy={command.busy} disabled={stale || restorable.length === 0} onClick={() => void command.run(() => tauri.resetWeek(weekStart, draftPlanId), restorable.length === 1 ? t("One session is back on the plan.") : t("{{value0}} sessions are back on the plan.", { value0: restorable.length })).then(ok => { if (ok) onClose(); })}>
        {restorable.length === 0 ? t("Nothing to reset") : t("Reset {{value0}}", { value0: restorable.length })}
      </Button>
    </div>
  </Dialog>;
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
  const [qualityDays, setQualityDays] = useState<number[]>([2, 4]);
  const [increase, setIncrease] = useState<PlanRequest["increase"]>("moderate");
  const [name, setName] = useState("");
  const [validation, setValidation] = useState<string | null>(null);
  const validQualityDays = qualityDays.filter(day => { const distance = Math.abs(day - longDay); return Math.min(distance, 7 - distance) >= 2; });
  const effectiveDate = kind === "maintenance" ? shiftDate(startDate || today(), weeks * 7) : target;
  const earliestEndDate = [shiftDate(startDate || today(), 15), shiftDate(today(), 1)].sort()[1];
  useEffect(() => { if (step > 0) form.current?.querySelector<HTMLElement>(step === 2 ? "h3" : "input,select")?.focus({ preventScroll: true }); }, [step]);
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); setValidation(null);
    if (!startDate || effectiveDate < earliestEndDate) { setValidation(t("Choose an end date at least 15 days after the start date and later than today.")); return; }
    if (step === 1 && (parsePace(threshold) == null || parsePace(threshold)! <= 0)) { setValidation(t("Enter a threshold pace such as 4:45.")); return; }
    if (step === 1 && validQualityDays.length === 0) { setValidation(t("Choose at least one quality day.")); return; }
    if (step < 2) { setStep(step + 1); return; }
    const parsed = parsePace(threshold);
    if (parsed == null || parsed <= 0) { setValidation(t("Enter a threshold pace such as 4:45.")); return; }
    if (await command.run(() => tauri.createPlan({ kind, trainingMethod, distance, startDate, targetDate: effectiveDate, thresholdSecKm: parsed, weeklyFrequency: frequency, longrunDay: longDay, qualityDays: validQualityDays, increase, currentWeeklyKm: mileage ? Number(mileage) : null, name: name.trim() || null }), t("Your plan is ready."))) { navigate("training", "plan"); onClose(); }
  }
  return <Dialog title={t("Create training plan")} wide onClose={onClose}><div className="plan-composer"><form ref={form} onSubmit={submit}><div className="composer-progress" aria-label={t("Step {{value0}} of 3", { value0: step + 1 })}>{[t("Direction"), t("Your baseline"), t("Review")].map((label, index) => <span key={label} aria-current={index === step ? "step" : undefined}>{index + 1}. {label}</span>)}</div>
    <ErrorNotice error={validation ?? command.error} />
    <Change identity={step}>
    {step === 0 && <div className="form-grid"><fieldset className="full"><legend>{t("What are you training for?")}</legend>{([ ["race", t("A race")], ["improvement", t("Improve my fitness")], ["maintenance", t("Stay consistent")] ] as const).map(([value, label]) => <label key={value}><input type="radio" name="plan-kind" value={value} checked={kind === value} onChange={() => setKind(value)} />{label}</label>)}</fieldset>
      <fieldset className="full"><legend>{t("Training method")}</legend>
        <label><input type="radio" name="training-method" value="norwegian" checked={trainingMethod === "norwegian"} aria-describedby="norwegian-description" onChange={() => setTrainingMethod("norwegian")} />{t("Norwegian method")}</label>
        <p id="norwegian-description" className="muted">{t("Controlled sub-threshold repeats, easy mileage and gradual progression, with quality work limited by your recovery.")}</p>
        <label><input type="radio" name="training-method" value="runna_like" checked={trainingMethod === "runna_like"} aria-describedby="runna-description" onChange={() => setTrainingMethod("runna_like")} />{t("Runna-like")}</label>
        <p id="runna-description" className="muted">{t("A varied mix of intervals, tempo and long runs, with recovery weeks and a race taper. Inspired by Runna's public guidance, not its proprietary algorithm.")}</p>
      </fieldset>
      <label>{t("Distance")}<select value={distance} onChange={event => setDistance(event.target.value as PlanRequest["distance"])}><option value="5K">5 km</option><option value="10K">10 km</option><option value="21K">{t("Half marathon")}</option><option value="42K">{t("Marathon")}</option></select></label>
      <label>{t("Start date")}<input type="date" required value={startDate} onChange={event => setStartDate(event.target.value)} /></label>
      {kind === "maintenance" ? <label>{t("Block length")}<select value={weeks} onChange={event => setWeeks(Number(event.target.value))}>{[4, 6, 8, 10, 12, 16].map(value => <option key={value} value={value}>{t("{{value0}} weeks", { value0: value })}</option>)}</select></label> : <label>{kind === "race" ? t("Race date") : t("Time trial date")}<input type="date" required min={earliestEndDate} value={target} onChange={event => setTarget(event.target.value)} /></label>}
      <label className="full">{t("Plan name (optional)")}<input value={name} maxLength={60} placeholder={t("Spring half build")} onChange={event => setName(event.target.value)} /><span className="muted">{t("How Runalytics calls this plan. Your watch keeps one Runalytics plan slot and shows the name inside its description.")}</span></label>
    </div>}
    {step === 1 && <div className="form-grid"><label>{t("Threshold pace (min/km)")}<input required value={threshold} placeholder="4:45" pattern="[0-9]{1,2}:[0-5][0-9]" onChange={event => setThreshold(event.target.value)} /></label><label>{t("Current weekly mileage (km)")}<input type="number" min="5" max="250" step="0.1" value={mileage} placeholder={defaults?.currentWeeklyKm ? t("Leave empty for your synced average: {{value0}}", { value0: number(defaults.currentWeeklyKm, 1) }) : t("Leave empty to use your synced history")} onChange={event => setMileage(event.target.value)} /><span className="muted">{t("Only fill this in if your recent runs do not reflect a normal week.")}</span></label><label>{t("Runs per week")}<input type="number" min="3" max="7" required value={frequency} onChange={event => setFrequency(Number(event.target.value))} /></label><label>{t("Long run day")}<select value={longDay} onChange={event => setLongDay(Number(event.target.value))}>{[t("Monday"), t("Tuesday"), t("Wednesday"), t("Thursday"), t("Friday"), t("Saturday"), t("Sunday")].map((day, index) => <option key={day} value={index + 1}>{day}</option>)}</select></label>{kind !== "maintenance" && <label className="full">{t("Mileage progression")}<select value={increase} onChange={event => setIncrease(event.target.value as PlanRequest["increase"])}><option value="conservative">{t("Conservative / 5%")}</option><option value="moderate">{t("Moderate / 8%")}</option><option value="ambitious">{t("Ambitious / 12%")}</option></select></label>}</div>}
    {step === 1 && <fieldset><legend>{t("Tempo and interval days")}</legend><p className="muted">{t("Choose available days at least two days from your long run. Some weeks may have fewer quality sessions.")}</p>{[t("Monday"), t("Tuesday"), t("Wednesday"), t("Thursday"), t("Friday"), t("Saturday"), t("Sunday")].map((day, index) => { const value = index + 1; const distance = Math.abs(value - longDay); const disabled = Math.min(distance, 7 - distance) < 2; return <label key={value}><input type="checkbox" checked={!disabled && qualityDays.includes(value)} disabled={disabled} onChange={event => setQualityDays(current => event.target.checked ? [...current.filter(selected => selected !== value), value] : current.filter(selected => selected !== value))} />{day}</label>; })}</fieldset>}
    {step === 2 && <><h3 tabIndex={-1}>{t("Ready to build your plan")}</h3><p>{t("Your schedule is generated locally from these inputs. Sending it to your watch is a separate action.")}</p>{defaults?.activePain && <p className="context-note">{t("Unresolved pain: {{value0}}, {{value1}}/10. Consider reviewing it in Health before starting a new plan.", { value0: defaults.activePain.bodyPart, value1: defaults.activePain.intensity })}</p>}</>}
    </Change>
    <div className="form-actions">{step > 0 && <Button onClick={() => setStep(step - 1)}>{t("Back")}</Button>}<Button tone="primary" type="submit" busy={command.busy}>{step === 2 ? t("Build plan") : t("Continue")}</Button></div>
    </form><aside className="composer-summary"><h3>{t("Your training brief")}</h3><dl className="definition-list"><div><dt>{t("Goal")}</dt><dd>{t(kind === "race" ? "Race" : kind === "improvement" ? "Improve my fitness" : "Stay consistent")}</dd></div><div><dt>{t("Training method")}</dt><dd>{trainingMethod === "runna_like" ? t("Runna-like") : t("Norwegian method")}</dd></div><div><dt>{t("Distance")}</dt><dd>{distance}</dd></div><div><dt>{t("Start date")}</dt><dd>{startDate ? fmtDateDMY(startDate) : t("Not entered")}</dd></div><div><dt>{t("End date")}</dt><dd>{fmtDateDMY(effectiveDate)}</dd></div><div><dt>{t("Frequency")}</dt><dd>{t("{{value0}} runs/week", { value0: frequency })}</dd></div><div><dt>{t("Starting volume")}</dt><dd>{mileage ? t("{{value0}} km/week", { value0: mileage }) : t("From synced history")}</dd></div><div><dt>{t("Threshold")}</dt><dd>{threshold || t("Not entered")}</dd></div></dl>{defaults?.weeksObserved ? <p className="muted">{t("Baseline from {{value0}} complete weeks of training.", { value0: defaults.weeksObserved })}</p> : null}</aside></div></Dialog>;
}
