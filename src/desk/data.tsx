import { createContext, useContext, useEffect, useRef, useState, type DependencyList, type ReactNode } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { tauri, ymdLocal, inTauri } from "../lib/ipc";
import type { PlanSyncProgress, Settings } from "../lib/types";
import { motion, useReducedMotion } from "motion/react";
import { IconX } from "@tabler/icons-react";
import { actionOrigin, Change, Presence, tempo } from "./interaction";

export type Workspace = "training" | "journal" | "health" | "settings" | "help";
export type Route = { workspace: Workspace; section: string };
export function readRoute(): Route {
  const [workspace, section = ""] = location.hash.slice(1).split("/");
  if (["training", "journal", "health", "settings", "help"].includes(workspace)) return { workspace: workspace as Workspace, section };
  const file = location.pathname.split("/").pop();
  if (file === "settings.html") return { workspace: "settings", section: workspace || "" };
  if (file === "support.html") return { workspace: "help", section: "" };
  if (file === "injury.html") return { workspace: "health", section: "journal" };
  if (file === "achievements.html") {
    if (workspace === "milestones") return { workspace: "health", section: "milestones" };
    if (["performance", "recovery", "risk"].includes(workspace)) return { workspace: "health", section: "" };
    return { workspace: "journal", section: workspace === "routes" ? "map" : "" };
  }
  return { workspace: "training", section: file === "plan.html" ? "plan" : file === "calendar.html" ? "month" : "week" };
}
export function navigate(workspace: Workspace, section = "") {
  location.hash = `${workspace}${section ? `/${section}` : ""}`;
}
export function useRoute() {
  const [route, setRoute] = useState(readRoute);
  useEffect(() => {
    const changed = () => setRoute(readRoute());
    window.addEventListener("hashchange", changed);
    return () => window.removeEventListener("hashchange", changed);
  }, []);
  return route;
}

type Notice = { text: string; action?: { label: string; run: () => Promise<unknown> } } | null;
type WorkspaceContext = { revision: number; syncing: boolean; planProgress: PlanSyncProgress | null; refresh: () => void; notify: (text: string, action?: NonNullable<Notice>["action"]) => void };
const Context = createContext<WorkspaceContext>({ revision: 0, syncing: false, planProgress: null, refresh() {}, notify() {} });
export const useWorkspace = () => useContext(Context);
export function WorkspaceProvider({ children }: { children: ReactNode }) {
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState<Notice>(null);
  const [acting, setActing] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [planProgress, setPlanProgress] = useState<PlanSyncProgress | null>(null);
  const reduced = useReducedMotion();
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
      if (!result.ok) setNotice({text:"Background sync failed. Check your watch connection and try again."});
    });
    register<PlanSyncProgress>("plan://sync", progress => setPlanProgress(progress));
    void tauri.syncStatus().then(value => { if (!disposed && !receivedEvent) setSyncing(value); }).catch(() => undefined);
    return () => { disposed = true; stops.forEach(stop => stop()); };
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
  return <Context.Provider value={{ revision, syncing, planProgress, refresh, notify: (text, action) => setNotice({ text, action }) }}>
    {children}
    <Presence>{notice && <motion.div key="notice" className="desk-notice" role="status" initial={{ opacity: 0, y: reduced ? 0 : 8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: reduced ? 0 : 8 }} transition={{ duration: reduced ? .08 : tempo.change, ease: tempo.settle }}><Change identity={notice.text}><span>{notice.text}</span></Change>
      {notice.action && <button disabled={acting} onClick={async () => {
        setActing(true);
        try { await notice.action!.run(); refresh(); setNotice({ text: "Change undone." }); }
        catch (error) { setNotice({ text: String(error) }); }
        finally { setActing(false); }
      }}>{notice.action.label}</button>}
      <button className="notice-close" aria-label="Dismiss notification" onClick={() => setNotice(null)}><IconX size={18} aria-hidden="true" /></button>
    </motion.div>}</Presence>
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
export const shortDate = (date: string) => dateValue(date).toLocaleDateString("en-GB", { day: "numeric", month: "short" });
export const dayName = (date: string) => dateValue(date).toLocaleDateString("en-GB", { weekday: "short" });
export const cleanTitle = (title: string) => title.replace(/^[^\p{L}\p{N}]+/u, "").trim();
export const kindName = (kind: string) => ({ longRun: "Long run", easy: "Easy", tempo: "Tempo", intervals: "Intervals", race: "Race", rest: "Rest", completed: "Recorded" }[kind] ?? kind);
export async function patchSettings(patch: Partial<Settings>) {
  const latest = await tauri.getSettings();
  await tauri.saveSettings({ ...latest, ...patch });
  return tauri.getSettings();
}