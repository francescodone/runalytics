import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { IconArrowLeft, IconArrowRight, IconCheck, IconChevronDown } from "@tabler/icons-react";
import { AnimatePresence, motion, usePresence, useReducedMotion } from "motion/react";
import { tauri, fmtPace, parsePace } from "../lib/ipc";
import type { PlanKind, PlanRequest, PlanWorkout, TrainingPlan } from "../lib/types";
import { t, date as formatDate, number } from "./i18n";
import { monday, shiftDate, today, useRemote, useWorkspace, SetupFeedback } from "./data";
import { BrandMark, Button, ErrorNotice, Loading } from "./ui";
import { tempo } from "./motion";
import { useCascadeExit } from "./interaction";
import { WatchSettings, StravaSettings, DeviceSync, CalendarSettings } from "./settings";
import { ConnectionTransfer, TransferContentsSummary, type TransferContents } from "./connection-transfer";

const storageKey = "runalytics.onboarding.v1";
type Draft = {
  step: string; name: string; transfer: boolean | null; strava: boolean | null; devices: boolean | null; calendar: boolean | null;
  kind: PlanKind; method: NonNullable<PlanRequest["trainingMethod"]>; distance: PlanRequest["distance"];
  start: string; end: string; threshold: string; mileage: string; recentLongRun: string; frequency: number; longDay: number;
  qualityDays: number[]; restDays: number[]; increase: PlanRequest["increase"];
};
const initialDraft = (): Draft => ({ step: "transfer", name: "", transfer: null, strava: null, devices: null, calendar: null, kind: "race", method: "norwegian", distance: "21K", start: today(), end: shiftDate(today(), 70), threshold: "", mileage: "", recentLongRun: "", frequency: 4, longDay: 7, qualityDays: [2, 4], restDays: [], increase: "moderate" });
export function hasOnboardingDraft() { try { return localStorage.getItem(storageKey) !== null; } catch { return false; } }
function loadDraft(): Draft {
  try { return { ...initialDraft(), ...JSON.parse(localStorage.getItem(storageKey) || "{}") }; } catch { return initialDraft(); }
}

function OnboardingStep({ children }: { children: ReactNode }) {
  const [present, remove] = usePresence();
  const reduced = useReducedMotion();
  const [immediate, setImmediate] = useState(() => matchMedia("(max-width: 800px)").matches);
  const host = useRef<HTMLElement>(null);
  useEffect(() => {
    const media = matchMedia("(max-width: 800px)");
    const update = () => setImmediate(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);
  const tail = useCascadeExit(host, present, !reduced && !immediate);
  useEffect(() => {
    if (present) return;
    if (reduced) { remove?.(); return; }
    const timer = setTimeout(() => remove?.(), (tempo.exit + tail) * 1000);
    return () => clearTimeout(timer);
  }, [present, reduced, immediate, remove, tail]);
  useLayoutEffect(() => {
    if (!present || !host.current) return;
    host.current.closest(".onboarding-shell")?.scrollTo({ top: 0, behavior: "instant" });
    host.current.querySelector<HTMLElement>("h1")?.focus({ preventScroll: true });
  }, [present]);
  return <motion.section ref={host} className="onboarding-step" data-entrance={immediate ? "none" : undefined} data-step-state={present ? "present" : "exiting"} inert={!present} initial={immediate && !reduced ? { opacity: 0 } : false} animate={{ opacity: 1 }} exit={{ opacity: reduced ? 1 : 0 }} transition={{ duration: present ? tempo.landing : tempo.exit + tail }}>{children}</motion.section>;
}

export function Onboarding({ onComplete }: { onComplete: () => void }) {
  const [draft, setDraft] = useState(loadDraft);
  const [preview, setPreview] = useState<[TrainingPlan, PlanWorkout[]] | null>(null);
  const [imported, setImported] = useState(false);
  const [busy, setBusy] = useState(false);
  const [operation, setOperation] = useState<"sync" | "create" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [restoreToken, setRestoreToken] = useState<boolean | null>(null);
  const [transferContents, setTransferContents] = useState<TransferContents | null>(null);
  const { syncStage } = useWorkspace();
  const loading = operation !== null;
  const main = useRef<HTMLElement>(null);
  const wasLoading = useRef(false);
  useLayoutEffect(() => {
    if (loading) main.current?.querySelector<HTMLElement>(".onboarding-loading")?.focus();
    else if (wasLoading.current) main.current?.querySelector<HTMLElement>('.onboarding-step[data-step-state="present"] h1')?.focus({ preventScroll: true });
    wasLoading.current = loading;
  }, [loading]);
  const defaults = useRemote(tauri.planDefaults);
  const providers = useRemote(tauri.providerStatus);
  const strava = useRemote(() => draft.step === "strava-config" ? tauri.stravaStatus() : Promise.resolve(null), [draft.step]);
  const calendar = useRemote(() => draft.step === "calendar-config" ? tauri.calendarPublishing() : Promise.resolve(null), [draft.step]);
  const devices = useRemote(async () => {
    if (draft.step !== "devices-config") return null;
    const status = await tauri.planSyncStatus();
    if (status.config.enabled && status.config.hasToken && status.accountLinked) await tauri.planSyncCheck();
    return status;
  }, [draft.step]);
  const connected = Boolean(providers.data?.some(provider => provider.provider === "coros" && provider.connected));
  const steps = ["transfer", ...(draft.transfer ? ["transfer-config"] : []), "name", "watch", "strava", ...(draft.strava ? ["strava-config", "publishing"] : []), "calendar", ...(draft.calendar ? ["calendar-config"] : []), "devices", ...(draft.devices ? ["devices-config"] : []), "goal", "method", "distance", "start", "end", "pace", "mileage", "recent-long-run", "frequency", "rest-days", "long-day", "quality", "preview"];
  const index = Math.max(0, steps.indexOf(draft.step));
  const update = (patch: Partial<Draft>) => setDraft(current => ({ ...current, ...patch }));
  const restore = (hasToken: boolean) => {
    setRestoreToken(hasToken); setBusy(true); setOperation("sync"); setError(null);
    void (async () => {
      const results = await Promise.allSettled([hasToken ? tauri.planSyncRefresh() : Promise.resolve(false), tauri.syncNow("coros")]);
      const failures = results.filter((result): result is PromiseRejectedResult => result.status === "rejected");
      if (failures.length) { setError(failures.map(result => String(result.reason)).join("\n")); return; }
      await tauri.currentPlan();
      setRestoreToken(null); update({ step: "name" });
    })().catch(failure => setError(String(failure))).finally(() => { setBusy(false); setOperation(null); });
  };
  useEffect(() => { try { localStorage.setItem(storageKey, JSON.stringify(draft)); } catch {} }, [draft]);
  useEffect(() => { if (!draft.threshold && defaults.data?.thresholdSecKm) update({ threshold: fmtPace(defaults.data.thresholdSecKm) }); }, [defaults.data?.thresholdSecKm]);
  const request: PlanRequest = { kind: draft.kind, trainingMethod: draft.method, distance: draft.distance, startDate: draft.start, targetDate: draft.end, thresholdSecKm: parsePace(draft.threshold) ?? 0, currentWeeklyKm: draft.mileage ? Number(draft.mileage) : null, recentLongRunKm: draft.recentLongRun ? Number(draft.recentLongRun) : null, weeklyFrequency: draft.frequency, longrunDay: draft.longDay, restDays: draft.restDays, qualityDays: draft.qualityDays.filter(day => !draft.restDays.includes(day) && Math.min(Math.abs(day - draft.longDay), 7 - Math.abs(day - draft.longDay)) >= 2), increase: draft.increase };
  const needsInput = (draft.step === "transfer" && draft.transfer === null) || (draft.step === "strava" && draft.strava === null) || (draft.step === "calendar" && draft.calendar === null) || (draft.step === "devices" && draft.devices === null) || (draft.step === "name" && !draft.name.trim()) || (draft.step === "watch" && !connected) || draft.step === "transfer-config" || (draft.step === "strava-config" && !strava.data?.connected) || (draft.step === "calendar-config" && (calendar.loading || !!calendar.error || !(calendar.data?.enabled && calendar.data.uploadUrl && calendar.data.subscriptionUrl && calendar.data.lastPublished && !calendar.data.lastError))) || (draft.step === "devices-config" && (devices.loading || !!devices.error || !(devices.data?.config.enabled && devices.data.config.hasToken && devices.data.accountLinked))) || (draft.step === "start" && !draft.start) || (draft.step === "end" && !draft.end) || (draft.step === "pace" && request.thresholdSecKm <= 0) || (draft.step === "quality" && !request.qualityDays?.length) || (draft.step === "preview" && !preview);
  const advance = async () => {
    setError(null);
    if (draft.step === "name" && !draft.name.trim()) { setError(t("Enter your name.")); return; }
    if (draft.step === "watch" && !connected) { setError(t("Connect your watch to continue.")); return; }
    if (draft.step === "watch" && draft.transfer) {
      try { if (await tauri.currentPlan()) { update({ step: "preview" }); return; } }
      catch (failure) { setError(String(failure)); return; }
    }
    if ((draft.step === "transfer" && draft.transfer === null) || (draft.step === "strava" && draft.strava === null) || (draft.step === "calendar" && draft.calendar === null) || (draft.step === "devices" && draft.devices === null)) { setError(t("Choose an option to continue.")); return; }
    if ((draft.step === "start" && !draft.start) || (draft.step === "end" && (!draft.end || draft.end < shiftDate(draft.start, 15) || draft.end <= today()))) { setError(t("Choose an end date at least 15 days after the start date and later than today.")); return; }
    if (draft.step === "pace" && request.thresholdSecKm <= 0) { setError(t("Enter a threshold pace such as 4:45.")); return; }
    if (draft.step === "recent-long-run" && request.recentLongRunKm != null && (!Number.isFinite(request.recentLongRunKm) || request.recentLongRunKm < 3 || request.recentLongRunKm > 100)) { setError(t("Enter a distance between 3 and 100 km, or leave it empty.")); return; }
    if (draft.step === "rest-days" && (draft.restDays.length > 7 - draft.frequency || (draft.kind !== "maintenance" && draft.restDays.includes((new Date(`${draft.end}T12:00:00`).getDay() + 6) % 7 + 1)))) { setError(t("Rest days must leave enough running days and cannot include race day.")); return; }
    if (draft.step === "long-day" && draft.restDays.includes(draft.longDay)) { setError(t("Choose a long run day that is not a rest day.")); return; }
    if (draft.step === "quality" && !request.qualityDays?.length) { setError(t("Choose at least one quality day.")); return; }
    if (draft.step === "quality") { setPreview(null); update({ step: "preview" }); return; }
    if (draft.step === "preview") {
      setBusy(true);
      setOperation("create");
      try {
        if (!imported) await tauri.createPlan(request);
        try { localStorage.setItem("runalytics.onboarding.complete", "1"); localStorage.removeItem(storageKey); localStorage.setItem("runalytics.runnerName", draft.name.trim()); } catch {}
        onComplete();
      } catch (failure) { setError(String(failure)); } finally { setBusy(false); setOperation(null); }
      return;
    }
    update({ step: steps[index + 1] });
  };
  useEffect(() => {
    if (draft.step !== "preview") return;
    let active = true;
    setBusy(true); setError(null);
    tauri.currentPlan().then(async current => ({ current, result: current ?? await tauri.previewPlan(request) })).then(({ current, result }) => { if (active) { setImported(Boolean(current)); setPreview(result); } }, failure => { if (active) { setPreview(null); setError(String(failure)); } }).finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, [draft.step, draft.increase]);
  const choice = (value: boolean | null, field: "transfer" | "strava" | "devices" | "calendar") => <fieldset className="onboarding-options"><legend className="sr-only">{t("Choose an option")}</legend>{([true, false] as const).map(option => <label key={String(option)}><input type="radio" name={field} checked={value === option} onChange={() => update({ [field]: option })} />{option ? t("Yes") : t("Not now")}</label>)}</fieldset>;
  const weekdays = [t("Monday"), t("Tuesday"), t("Wednesday"), t("Thursday"), t("Friday"), t("Saturday"), t("Sunday")];
  const previewWeeks = new Map<string, PlanWorkout[]>();
  for (const workout of preview?.[1] ?? []) {
    const week = monday(workout.date);
    previewWeeks.set(week, [...(previewWeeks.get(week) ?? []), workout]);
  }
  const heading: Record<string, string> = { name: t("What should we call you?"), watch: t("Connect your watch"), transfer: t("Import connections with a QR code?"), "transfer-config": t("Import your connections"), strava: t("Connect Strava?"), "strava-config": t("Connect with Strava"), publishing: t("Update your Strava activities?"), calendar: t("Connect a calendar or GitHub Gist?"), "calendar-config": t("Calendar apps"), devices: t("Sync across devices?"), "devices-config": t("Set up device sync"), goal: t("What are you training for?"), method: t("Training method"), distance: t("Distance"), start: t("Start date"), end: t("End date"), pace: t("Threshold pace (min/km)"), mileage: t("Current weekly mileage (km)"), "recent-long-run": t("What is your longest recent long run (km)?"), frequency: t("Runs per week"), "rest-days": t("Which days must stay free?"), "long-day": t("Long run day"), quality: t("Tempo and interval days"), preview: t("Your plan preview") };
  return <div className="onboarding-shell"><header className="onboarding-header"><strong><BrandMark /><span className="brand-wordmark">Runalytics</span></strong><nav aria-label={t("workspaces")}><span aria-current="page">{t("Setup")}</span></nav></header><main ref={main} className="onboarding-main" id="workspace" tabIndex={-1}>
    {loading && <div className="onboarding-loading" tabIndex={-1}><Loading label={operation === "create" ? t("Building your baseline") : syncStage ?? t("syncing")} /></div>}
    <div hidden={loading}>
    <div className="onboarding-progress"><span>{t("Step {{value0}} of {{value1}}", { value0: index + 1, value1: steps.length })}</span><progress max={steps.length} value={index + 1} aria-label={t("Setup progress")} /></div>
    <div className="onboarding-step-host"><AnimatePresence initial={false} mode="wait"><OnboardingStep key={draft.step}><SetupFeedback><h1 tabIndex={-1}>{heading[draft.step]}</h1>
      {draft.step === "name" && <>{transferContents && <TransferContentsSummary contents={transferContents} />}<label>{t("Name")}<input autoComplete="given-name" value={draft.name} maxLength={80} onChange={event => update({ name: event.target.value })} /></label></>}
      {draft.step === "watch" && <WatchSettings onboarding onSyncChange={active => setOperation(active ? "sync" : null)} />}
      {draft.step === "transfer" && choice(draft.transfer, "transfer")}
      {draft.step === "transfer-config" && <ConnectionTransfer setupWatch={<WatchSettings onboarding />} onImported={(hasToken, contents, name) => { setTransferContents(contents); if (name) update({ name }); restore(hasToken); }} />}
      {draft.step === "strava" && choice(draft.strava, "strava")}
      {draft.step === "strava-config" && <StravaSettings mode="connection" />}
      {draft.step === "publishing" && <StravaSettings mode="publishing" />}
      {draft.step === "calendar" && choice(draft.calendar, "calendar")}
      {draft.step === "calendar-config" && <CalendarSettings onboarding />}
      {draft.step === "devices" && choice(draft.devices, "devices")}
      {draft.step === "devices-config" && <DeviceSync onRequestWatch={() => update({ step: "watch" })} />}
      {draft.step === "goal" && <fieldset className="onboarding-options"><legend className="sr-only">{heading.goal}</legend>{([ ["race", t("A race")], ["improvement", t("Improve my fitness")], ["maintenance", t("Stay consistent")] ] as const).map(([value, label]) => <label key={value}><input type="radio" name="goal" checked={draft.kind === value} onChange={() => update({ kind: value })} />{label}</label>)}</fieldset>}
      {draft.step === "method" && <fieldset className="onboarding-options"><legend className="sr-only">{heading.method}</legend>{([ ["norwegian", t("Norwegian method")], ["runna_like", t("Runna-like")] ] as const).map(([value, label]) => <label key={value}><input type="radio" name="method" checked={draft.method === value} onChange={() => update({ method: value })} />{label}</label>)}</fieldset>}
      {draft.step === "distance" && <label>{t("Distance")}<select value={draft.distance} onChange={event => update({ distance: event.target.value as Draft["distance"] })}><option value="5K">5 km</option><option value="10K">10 km</option><option value="21K">{t("Half marathon")}</option><option value="42K">{t("Marathon")}</option></select></label>}
      {draft.step === "start" && <label>{t("Start date")}<input type="date" value={draft.start} onChange={event => update({ start: event.target.value })} /></label>}
      {draft.step === "end" && <label>{t("End date")}<input type="date" value={draft.end} onChange={event => update({ end: event.target.value })} /></label>}
      {draft.step === "pace" && <label>{heading.pace}<input inputMode="numeric" placeholder="4:45" value={draft.threshold} onChange={event => update({ threshold: event.target.value })} /></label>}
      {draft.step === "mileage" && <label>{heading.mileage}<input type="number" min="5" max="250" step="0.1" value={draft.mileage} placeholder={t("Leave empty to use your synced history")} onChange={event => update({ mileage: event.target.value })} /></label>}
      {draft.step === "recent-long-run" && <label>{heading["recent-long-run"]}<input type="number" min="3" max="100" step="0.1" placeholder={t("Optional")} value={draft.recentLongRun} onChange={event => update({ recentLongRun: event.target.value })} /></label>}
      {draft.step === "frequency" && <label>{heading.frequency}<select value={draft.frequency} onChange={event => update({ frequency: Number(event.target.value) })}>{[3, 4, 5, 6, 7].map(value => <option key={value}>{value}</option>)}</select></label>}
      {draft.step === "rest-days" && <fieldset className="onboarding-options"><legend>{t("Choose days without training, or leave all unselected.")}</legend>{weekdays.map((day, dayIndex) => { const value = dayIndex + 1; return <label key={day}><input type="checkbox" checked={draft.restDays.includes(value)} onChange={event => update({ restDays: event.target.checked ? [...draft.restDays, value] : draft.restDays.filter(selected => selected !== value) })} />{day}</label>; })}</fieldset>}
      {draft.step === "long-day" && <fieldset className="onboarding-options"><legend className="sr-only">{heading["long-day"]}</legend>{weekdays.map((day, dayIndex) => { const value = dayIndex + 1; return <label key={day}><input type="radio" name="long-day" checked={draft.longDay === value && !draft.restDays.includes(value)} disabled={draft.restDays.includes(value)} onChange={() => update({ longDay: value })} />{day}</label>; })}</fieldset>}
      {draft.step === "quality" && <fieldset className="onboarding-options"><legend>{t("Choose available days at least two days from your long run. Some weeks may have fewer quality sessions.")}</legend>{weekdays.map((day, dayIndex) => { const value = dayIndex + 1; const distance = Math.abs(value - draft.longDay); const disabled = draft.restDays.includes(value) || Math.min(distance, 7 - distance) < 2; return <label key={day}><input type="checkbox" disabled={disabled} checked={!disabled && draft.qualityDays.includes(value)} onChange={event => update({ qualityDays: event.target.checked ? [...draft.qualityDays, value] : draft.qualityDays.filter(selected => selected !== value) })} />{day}</label>; })}</fieldset>}
      {draft.step === "preview" && <>{imported ? <p>{t("Your imported plan is ready.")}</p> : <><p>{t("Adjust the progression before creating your plan.")}</p><label>{t("Mileage progression")}<select value={draft.increase} onChange={event => { setPreview(null); update({ increase: event.target.value as Draft["increase"] }); }}><option value="conservative">{t("Conservative / 5%")}</option><option value="moderate">{t("Moderate / 8%")}</option><option value="ambitious">{t("Ambitious / 12%")}</option></select></label></>}{busy ? <Loading /> : preview && <><p>{t("{{value0}} sessions", { value0: preview[1].length })}</p>{[...previewWeeks].map(([week, workouts], weekIndex) => <details key={week} open={weekIndex === 0 || undefined}><summary><span className="onboarding-week-name">{t("Week {{value0}}", { value0: weekIndex + 1 })}<time dateTime={week}>{formatDate(week)}</time></span><span className="onboarding-week-total">{number(workouts.reduce((total, workout) => total + (workout.distanceM ?? 0), 0) / 1000, 1)} km</span><IconChevronDown className="onboarding-week-chevron" size={18} aria-hidden="true" /></summary><ul className="onboarding-preview">{workouts.map(workout => <li key={workout.id}><div><strong>{workout.title}</strong><time dateTime={workout.date}>{formatDate(workout.date)}</time></div><span>{workout.distanceM != null ? `${number(workout.distanceM / 1000, 1)} km` : ""}</span></li>)}</ul></details>)}</>}</>}
    </SetupFeedback></OnboardingStep></AnimatePresence></div>
    <ErrorNotice error={error ?? (draft.step === "watch" ? providers.error : draft.step === "devices-config" ? devices.error : draft.step === "calendar-config" ? calendar.error : null)} retry={error && draft.step === "transfer-config" && restoreToken !== null && !busy ? () => restore(restoreToken) : draft.step === "devices-config" && devices.error ? devices.retry : draft.step === "calendar-config" && calendar.error ? calendar.retry : undefined} />
    <div className="form-actions onboarding-actions">{index > 0 && <Button disabled={busy} onClick={() => { setError(null); update({ step: steps[index - 1] }); }}><IconArrowLeft size={18} />{t("Back")}</Button>}{draft.step === "watch" && !connected && <Button onClick={() => { setError(null); update({ step: steps[index + 1] }); }}>{t("Not now")}</Button>}{["transfer-config", "strava-config", "calendar-config", "devices-config"].includes(draft.step) && <Button disabled={busy} onClick={() => { setError(null); const field = draft.step === "transfer-config" ? "transfer" : draft.step === "strava-config" ? "strava" : draft.step === "calendar-config" ? "calendar" : "devices"; const next = draft.step === "transfer-config" ? "name" : draft.step === "strava-config" ? "calendar" : draft.step === "calendar-config" ? "devices" : "goal"; update({ [field]: false, step: next }); }}>{t("Not now")}</Button>}<Button tone="primary" busy={busy} disabled={busy || needsInput} onClick={() => void advance()}>{draft.step === "preview" ? <IconCheck size={18} /> : <IconArrowRight size={18} />}{draft.step === "preview" && !imported ? t("Build plan") : t("Continue")}</Button></div>
    </div>
  </main></div>;
}