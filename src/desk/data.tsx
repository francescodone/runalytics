import { t, text, formatLocale, date as fmtDateDMY } from "./i18n";
import { createContext, useContext, useEffect, useMemo, useRef, useState, type DependencyList, type ReactNode } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { tauri, ymdLocal, inTauri, displayTitle } from "../lib/ipc";
import type { PlanSyncProgress, Settings } from "../lib/types";
import { IconLoader2, IconCheck, IconX } from "@tabler/icons-react";
import { actionOrigin, Change, Presence } from "./interaction";
import { Notification, NotificationViewport } from "./overlays";

export type Workspace = "training" | "journal" | "health" | "settings" | "help";
/// `detail` is the third hash segment (`#journal/1/splits`): the open tab of a
/// selected record. It is only ever read by the record itself, so every other
/// route leaves it empty.
export type Route = { workspace: Workspace; section: string; detail: string };
export function readRoute(): Route {
  const [workspace, section = "", detail = ""] = location.hash.slice(1).split("/");
  if (["training", "journal", "health", "settings", "help"].includes(workspace)) return { workspace: workspace as Workspace, section, detail };
  const file = location.pathname.split("/").pop();
  if (file === "settings.html") return { workspace: "settings", section: workspace || "", detail: "" };
  if (file === "support.html") return { workspace: "help", section: "", detail: "" };
  if (file === "injury.html") return { workspace: "health", section: "journal", detail: "" };
  if (file === "achievements.html") {
    if (workspace === "milestones") return { workspace: "health", section: "milestones", detail: "" };
    if (["performance", "recovery", "risk"].includes(workspace)) return { workspace: "health", section: "", detail: "" };
    return { workspace: "journal", section: "", detail: "" };
  }
  return { workspace: "training", section: file === "plan.html" ? "plan" : file === "calendar.html" ? "month" : "week", detail: "" };
}
export function navigate(workspace: Workspace, section = "", detail = "") {
  location.hash = `${workspace}${section ? `/${section}` : ""}${detail ? `/${detail}` : ""}`;
}
export function useRoute() {
  const [route, setRoute] = useState(readRoute);
  useEffect(() => {
    const changed = () => setRoute(readRoute());
    changed();
    window.addEventListener("hashchange", changed);
    return () => window.removeEventListener("hashchange", changed);
  }, []);
  return route;
}

type Notice = { text: string; action?: { label: string; run: () => Promise<unknown> } } | null;
const noticeDuration = () => {
  const duration = getComputedStyle(document.documentElement).getPropertyValue("--notice-duration").trim();
  const value = parseFloat(duration);
  return Number.isFinite(value) ? value * (duration.endsWith("ms") ? 1 : 1000) : 6000;
};
function scheduleNoticeDismissal(node: HTMLElement | null, dismiss: () => void) {
  if (!node) return () => {};
  let remaining = noticeDuration();
  let started = 0;
  let timer = 0;
  let disposed = false;
  const update = () => {
    if (disposed) return;
    const held = node.matches(":hover") || node.contains(document.activeElement);
    if (held && timer) {
      window.clearTimeout(timer);
      timer = 0;
      remaining = Math.max(0, remaining - (performance.now() - started));
    } else if (!held && !timer) {
      started = performance.now();
      timer = window.setTimeout(dismiss, remaining);
    }
  };
  const focusOut = () => queueMicrotask(update);
  node.addEventListener("pointerenter", update);
  node.addEventListener("pointerleave", update);
  node.addEventListener("focusin", update);
  node.addEventListener("focusout", focusOut);
  update();
  return () => {
    disposed = true;
    window.clearTimeout(timer);
    node.removeEventListener("pointerenter", update);
    node.removeEventListener("pointerleave", update);
    node.removeEventListener("focusin", update);
    node.removeEventListener("focusout", focusOut);
  };
}

/// One mirror report from the backend's `plan://stage` channel. `target`
/// "op" is the operation itself: its settle means no new line can arrive.
type StageEvent = { op: string; target: string; state: "working" | "done" | "error"; detail?: string | null };
type StageLine = { target: string; state: "working" | "done" | "error"; detail?: string | null };
type StageOp = { lines: StageLine[]; settled: boolean };
/// What the runner reads for each mirror. The order the backend opens the
/// lines in (DB first, then calendar, gist, watch) is kept as-is: it is the
/// chronological order the work actually happens in.
const STAGE_LABELS: Record<string, string> = { db: t("local DB"), calendar: "calendar", gist: "gist", coros: t("COROS calendar"), strava: "Strava" };
function stageText(line: StageLine): string {
  const label = text(STAGE_LABELS[line.target] ?? line.target);
  if (line.state === "working") return t("Updating {{value0}}…", { value0: label });
  if (line.target === "strava" && line.detail) return line.state === "error" ? t("Updating Strava failed: {{value0}}", { value0: line.detail }) : text(line.detail);
  return line.state === "done" ? t("Updated {{value0}}", { value0: label }) : t("Updating {{value0}} failed", { value0: label });
}
type WorkspaceContext = { revision: number; planCheck: number; syncing: boolean; planProgress: PlanSyncProgress | null; refresh: () => void; notify: (text: string, action?: NonNullable<Notice>["action"]) => void };
const Context = createContext<WorkspaceContext>({ revision: 0, planCheck: 0, syncing: false, planProgress: null, refresh() {}, notify() {} });
export const useWorkspace = () => useContext(Context);
export function WorkspaceProvider({ children }: { children: ReactNode }) {
  const [revision, setRevision] = useState(0);
  const [planCheck, setPlanCheck] = useState(0);
  const planUpdates = useRef(0);
  const [notices, setNotices] = useState<NonNullable<Notice>[]>([]);
  const notice = notices[0] ?? null;
  const setNotice = (next: Notice, immediate = false) => setNotices(previous => {
    if (!next) return previous.slice(1);
    if (previous.some(item => item.text === next.text && item.action === next.action)) return previous;
    return immediate && !acting ? [next, ...previous.filter(item => item.action)] : [...previous, next];
  });
  const noticeRef = useRef<HTMLDivElement>(null);
  const [stageOps, setStageOps] = useState<Record<string, StageOp>>({});
  const stagesRef = useRef<HTMLDivElement>(null);
  const [acting, setActing] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [planProgress, setPlanProgress] = useState<PlanSyncProgress | null>(null);
  const refresh = () => setRevision(value => value + 1);
  useEffect(() => {
    if (!inTauri) return;
    let disposed = false;
    let receivedEvent = false;
    const stops: UnlistenFn[] = [];
    const register = <Payload,>(name: string, callback: (payload: Payload) => void) => {
      void listen<Payload>(name, event => { if (!disposed) callback(event.payload); }).then(stop => { if (disposed) stop(); else stops.push(stop); }).catch(() => undefined);
    };
    register("sync://started", () => { receivedEvent = true; setSyncing(true); });
    register<{ok:boolean}>("sync://completed", result => {
      receivedEvent = true; setSyncing(false); setRevision(value => value + 1);
      if (!result.ok) setNotice({text:t("Background sync failed. Check your watch connection and try again.")});
    });
    register<PlanSyncProgress>("plan://sync", progress => setPlanProgress(progress));
    register<{ discarded?: boolean }>("plan://updated", result => {
      planUpdates.current += 1;
      setRevision(value => value + 1);
      if (result?.discarded) setNotice({ text: t("The Gist changed on another device. Its latest plan replaced unpublished local changes.") });
    });
    register("plan://checked", () => setPlanCheck(value => value + 1));
    register<StageEvent>("plan://stage", event => setStageOps(previous => {
      const current = previous[event.op] ?? { lines: [], settled: false };
      if (event.target === "op") return { ...previous, [event.op]: { lines: event.state === "working" ? [] : current.lines, settled: event.state !== "working" } };
      const lines = current.lines.slice();
      const index = lines.findIndex(line => line.target === event.target);
      const line: StageLine = { target: event.target, state: event.state, detail: event.detail ?? null };
      if (index >= 0) lines[index] = line; else lines.push(line);
      return { ...previous, [event.op]: { lines, settled: current.settled } };
    }));
    void tauri.syncStatus().then(value => { if (!disposed && !receivedEvent) setSyncing(value); }).catch(() => undefined);
    return () => { disposed = true; stops.forEach(stop => stop()); };
  }, []);
  useEffect(() => {
    if (!inTauri) return;
    let disposed = false;
    let pending = false;
    let requested = false;
    let timer = 0;
    let failures = 0;
    let retryAt = 0;
    let lastAttempt = 0;
    const eligible = () => !disposed && document.visibilityState === "visible" && navigator.onLine;
    const check = async () => {
      if (!eligible() || Date.now() < retryAt) return;
      if (pending) { requested = true; return; }
      if (Date.now() - lastAttempt < 2000) {
        timer = window.setTimeout(() => void check(), 2000 - (Date.now() - lastAttempt));
        return;
      }
      pending = true;
      requested = false;
      lastAttempt = Date.now();
      try {
        const status = await tauri.planSyncStatus();
        if (!eligible() || !status.config.enabled || !status.config.hasToken || !status.accountLinked) return;
        const before = planUpdates.current;
        const changed = await tauri.planSyncRefresh();
        if (changed && before === planUpdates.current && !disposed) setRevision(value => value + 1);
        failures = 0;
      } catch {
        failures += 1;
        retryAt = Date.now() + Math.min(300000, 15000 * 2 ** Math.min(failures - 1, 5));
        timer = window.setTimeout(() => void check(), retryAt - Date.now());
      } finally {
        pending = false;
        if (requested && !disposed) {
          requested = false;
          timer = window.setTimeout(() => void check(), Math.max(2000, retryAt - Date.now()));
        }
      }
    };
    const changed = () => { window.clearTimeout(timer); timer = window.setTimeout(() => void check(), 150); };
    window.addEventListener("focus", changed);
    window.addEventListener("pageshow", changed);
    window.addEventListener("online", changed);
    window.addEventListener("runalytics:plan-sync-enabled", changed);
    document.addEventListener("visibilitychange", changed);
    const interval = window.setInterval(() => void check(), 60000);
    changed();
    return () => {
      disposed = true;
      window.clearTimeout(timer);
      window.clearInterval(interval);
      window.removeEventListener("focus", changed);
      window.removeEventListener("pageshow", changed);
      window.removeEventListener("online", changed);
      window.removeEventListener("runalytics:plan-sync-enabled", changed);
      document.removeEventListener("visibilitychange", changed);
    };
  }, []);
  useEffect(() => {
    const changed = () => setRevision(value => value + 1);
    window.addEventListener("runalytics:sync-completed", changed);
    window.addEventListener("runalytics:calendar-updated", changed);
    return () => {
      window.removeEventListener("runalytics:sync-completed", changed);
      window.removeEventListener("runalytics:calendar-updated", changed);
    };
  }, []);
  useEffect(() => {
    if (!notice || acting) return;
    return scheduleNoticeDismissal(noticeRef.current, () => setNotice(null));
  }, [notice, acting]);
  // The stage toast keeps itself open while any line is still working, and
  // while the operation itself has not returned (a mirror that was never
  // attempted emits nothing, so only the op settle proves the list is
  // final). Once every line has a verdict it self-dismisses on the same
  // hover-hold clock as the plain notice.
  const stageEntries = useMemo(() => Object.entries(stageOps).filter(([, op]) => op.lines.length > 0), [stageOps]);
  const stagesQuiet = stageEntries.length > 0 && stageEntries.every(([, op]) => op.settled && op.lines.every(line => line.state !== "working"));
  useEffect(() => {
    if (!stagesQuiet) return;
    return scheduleNoticeDismissal(stagesRef.current, () => setStageOps({}));
  }, [stagesQuiet, stageOps]);
  return <Context.Provider value={{ revision, planCheck, syncing, planProgress, refresh, notify: (text, action) => setNotice({ text, action }, true) }}>
    {children}
    <NotificationViewport>
    <Presence>{notice && <Notification key="notice" elementRef={noticeRef} onDismiss={() => setNotice(null)} dismissDisabled={acting} countdown={!acting} countdownKey={text(notice.text)} actions={notice.action && <button type="button" className="desk-button" disabled={acting} onClick={async () => {
        setActing(true);
        try { await notice.action!.run(); refresh(); setNotices(previous => [{ text: t("Change undone.") }, ...previous.slice(1)]); }
        catch (error) { setNotices(previous => [{ text: String(error) }, ...previous.slice(1)]); }
        finally { setActing(false); }
      }}>{notice.action.label}</button>}><Change identity={text(notice.text)}><span>{text(notice.text)}</span></Change></Notification>}</Presence>
    <Presence>{stageEntries.length > 0 && <Notification key="stages" elementRef={stagesRef} label={t("Plan sync progress")} onDismiss={() => setStageOps({})} countdown={stagesQuiet}>
      <ul className="stage-lines">{stageEntries.flatMap(([opName, op]) => op.lines.map(line => <li key={`${opName}/${line.target}`} className={`stage-line stage-${line.state}`} title={line.detail ?? undefined}>
        {line.state === "working" ? <IconLoader2 size={16} className="spinner" aria-hidden="true" /> : line.state === "done" ? <IconCheck size={16} aria-hidden="true" /> : <IconX size={16} aria-hidden="true" />}
        <span>{stageText(line)}</span>
      </li>))}</ul>
    </Notification>}</Presence>
    </NotificationViewport>
  </Context.Provider>;
}

export function useRemote<Value>(load: () => Promise<Value>, dependencies: DependencyList = []) {
  const { revision } = useWorkspace();
  const [retry, setRetry] = useState(0);
  const [state, setState] = useState<{ data?: Value; error: string | null; loading: boolean }>({ error: null, loading: true });
  const previousDependencies = useRef(dependencies);
  useEffect(() => {
    let active = true;
    const changed = dependencies.length !== previousDependencies.current.length || dependencies.some((value, index) => !Object.is(value, previousDependencies.current[index]));
    previousDependencies.current = dependencies;
    setState(previous => changed ? { error: null, loading: true } : { ...previous, error: null, loading: previous.data === undefined });
    load().then(data => { if (active) setState({ data, error: null, loading: false }); }, error => {
      if (active) setState(previous => ({ ...previous, error: String(error), loading: false }));
    });
    return () => { active = false; };
  }, [...dependencies, revision, retry]);
  return { ...state, retry: () => setRetry(value => value + 1) };
}

export function useCommand() {
  const { refresh, notify } = useWorkspace();
  const guard = useRef(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function run(action: () => Promise<unknown>, message?: string) {
    if (guard.current) return false;
    guard.current = true;
    setBusy(true);
    setError(null);
    const origin = actionOrigin();
    try {
      await action();
      refresh();
      if (message) notify(message);
      origin?.dispatchEvent(new CustomEvent("desk:action-result", { detail: { ok: true } }));
      return true;
    } catch (failure) {
      setError(String(failure));
      origin?.dispatchEvent(new CustomEvent("desk:action-result", { detail: { ok: false } }));
      return false;
    } finally { guard.current = false; setBusy(false); }
  }
  return { busy, error, run };
}

export const today = () => ymdLocal(new Date());
export const dateValue = (date: string) => new Date(`${date.slice(0, 10)}T12:00:00`);
export function shiftDate(date: string, days: number) {
  const value = dateValue(date);
  value.setDate(value.getDate() + days);
  return ymdLocal(value);
}
export function monday(date: string) { return shiftDate(date, -((dateValue(date).getDay() + 6) % 7)); }
export const shortDate = (date: string) => fmtDateDMY(date);
export const dayName = (date: string) => dateValue(date).toLocaleDateString(formatLocale, { weekday: "short" });
export const cleanTitle = (title: string) => text(title.replace(/^[^\p{L}\p{N}]+/u, "").trim());
/// A session name for display: emoji stripped, and the softener's stored
/// `"(was: …)"` suffix dropped. See `displayTitle` in lib/ipc for why the
/// suffix stays in the database but never reaches the runner.
export const sessionTitle = (title: string) => cleanTitle(displayTitle(title));
/// Upper-cases only the first letter, leaving an already-capitalised or
/// deliberately-styled label alone; provider codes arrive in any case.
export const capitalizeFirst = (value: string) =>
  text(value.charAt(0).toUpperCase() + value.slice(1));
export const kindName = (kind: string) => ({ longRun: t("Long run"), easy: t("Easy"), tempo: t("Tempo"), intervals: t("Intervals"), race: t("Race"), rest: t("Rest"), completed: t("Recorded") }[kind] ?? kind);
export async function patchSettings(patch: Partial<Settings>) {
  const latest = await tauri.getSettings();
  await tauri.saveSettings({ ...latest, ...patch });
  return tauri.getSettings();
}