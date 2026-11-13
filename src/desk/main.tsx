import { FeedbackReminder } from "./feedback";
import { Activity, useEffect, useLayoutEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauri } from "@tauri-apps/api/core";
import { IconCalendarWeek, IconRun, IconChartLine, IconTrophy, IconSettings, IconHelp, IconRefresh } from "@tabler/icons-react";
import { motion, useReducedMotion } from "motion/react";
import { tauri } from "../lib/ipc";
import { navigate, useCommand, useRemote, useRoute, useWorkspace, WorkspaceProvider } from "./data";
import type { Workspace } from "./data";
import { BrandMark, ErrorNotice, IconButton } from "./ui";
import "./desk.css";
import { Training } from "./training";
import { Journal } from "./journal";
import { Milestones } from "./health";
import { PaceInsightsView } from "./pace-insights";
import { SettingsWorkspace, Help } from "./settings";
import { Onboarding, hasOnboardingDraft } from "./onboarding";
import { BrandSplash } from "./splash";
import { Change, MotionSystem, SelectionGroup, SelectionMarker, tempo, useCompactLayout } from "./interaction";
import { locale, t } from "./i18n";
import "./interaction.css";

const workspaces = [
  { id: "training", title: t("training"), Icon: IconCalendarWeek },
  { id: "journal", title: t("journal"), Icon: IconRun },
  { id: "pace", title: t("Pace insights"), Icon: IconChartLine },
  { id: "milestones", title: t("Milestones"), Icon: IconTrophy },
] as const;

function App() {
  const route = useRoute();
  const compact = useCompactLayout();
  const reduced = useReducedMotion();
  const previousWorkspace = useRef(route.workspace);
  const workspaceOrder: Workspace[] = ["training", "journal", "pace", "milestones", "settings", "help"];
  const direction = Math.sign(workspaceOrder.indexOf(route.workspace) - workspaceOrder.indexOf(previousWorkspace.current)) || 1;
  useEffect(() => { previousWorkspace.current = route.workspace; }, [route.workspace]);
  const [visitedWorkspaces, setVisitedWorkspaces] = useState<Workspace[]>([route.workspace]);
  if (compact && !visitedWorkspaces.includes(route.workspace)) setVisitedWorkspaces([...visitedWorkspaces, route.workspace]);
  const heading = useRef<HTMLHeadingElement>(null);
  const initial = useRef(true);
  const appearanceTransition = useRef<ViewTransition | null>(null);
  const appearanceVersion = useRef(0);
  const settings = useRemote(tauri.getSettings);
  const providers = useRemote(tauri.providerStatus);
  const currentPlan = useRemote(tauri.currentPlan);
  const [setupCompleted, setSetupCompleted] = useState(() => { try { return localStorage.getItem("runalytics.onboarding.complete") === "1"; } catch { return false; } });
  const [runnerName, setRunnerName] = useState(() => { try { return localStorage.getItem("runalytics.runnerName") ?? ""; } catch { return ""; } });
  const setupPending = !setupCompleted && (currentPlan.loading || currentPlan.error !== null || hasOnboardingDraft() || !currentPlan.data);
  const command = useCommand();
  const { syncing, notify, setSetupPending } = useWorkspace();
  useEffect(() => {
    const updateName = () => { try { setRunnerName(localStorage.getItem("runalytics.runnerName") ?? ""); } catch {} };
    window.addEventListener("runalytics:runner-name-changed", updateName);
    return () => window.removeEventListener("runalytics:runner-name-changed", updateName);
  }, []);
  useLayoutEffect(() => { setSetupPending(setupPending); }, [setupPending]);
  useEffect(() => {
    if (setupCompleted || !currentPlan.data || hasOnboardingDraft()) return;
    try { localStorage.setItem("runalytics.onboarding.complete", "1"); } catch {}
    setSetupCompleted(true);
  }, [setupCompleted, currentPlan.data]);
  useEffect(() => {
    if (setupPending) return;
    const probe = import.meta.env.DEV && new URLSearchParams(location.search).has("nativeChromeProbe");
    if (!probe && !(isTauri() && /iPhone|iPod/.test(navigator.userAgent))) return;
    let disposed = false;
    let stop: (() => void) | undefined;
    void import("./native-navigation").then(module => {
      if (!disposed) stop = module.startNativeNavigation();
    }).catch(() => {});
    return () => { disposed = true; stop?.(); };
  }, [setupPending]);
  useEffect(() => {
    const dragBackground = (event: MouseEvent) => {
      if (document.documentElement.dataset.nativeWindow !== "macos" || event.button !== 0 || event.defaultPrevented) return;
      if (event.target !== document.documentElement && event.target !== document.body && event.target !== document.getElementById("desk-root")) return;
      event.preventDefault();
      void getCurrentWindow().startDragging().catch(() => {});
    };
    document.addEventListener("mousedown", dragBackground);
    return () => document.removeEventListener("mousedown", dragBackground);
  }, []);
  useEffect(() => {
    if (initial.current) { initial.current = false; return; }
    document.getElementById("workspace")?.scrollTo({ top: 0, behavior: "instant" });
    heading.current?.focus({ preventScroll: true });
    heading.current?.parentElement?.dispatchEvent(new Event("desk:view-arrive", { bubbles: true }));
  }, [route.workspace]);
  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");
    let nativeTheme: string | undefined;
    const paint = () => {
      let cachedTheme = "system";
      let cachedPalette = "amethyst";
      try {
        cachedTheme = localStorage.getItem("runalytics.theme") ?? "system";
        cachedPalette = localStorage.getItem("runalytics.palette") ?? "amethyst";
        if (settings.data) {
          localStorage.setItem("runalytics.theme", settings.data.theme ?? "system");
          localStorage.setItem("runalytics.palette", settings.data.palette ?? "reference");
        }
      } catch {}
      const theme = settings.data?.theme ?? cachedTheme;
      const resolvedTheme = theme === "system" ? (media.matches ? "dark" : "light") : theme;
      const palette = settings.data ? (settings.data.palette ?? "reference") : cachedPalette;
      const root = document.documentElement;
      const changed = root.dataset.deskTheme !== resolvedTheme || root.dataset.deskPalette !== palette;
      const version = ++appearanceVersion.current;
      appearanceTransition.current?.skipTransition();
      const apply = () => {
        if (version !== appearanceVersion.current) return;
        root.dataset.deskTheme = resolvedTheme;
        root.style.colorScheme = resolvedTheme;
        root.dataset.deskPalette = palette;
        document.querySelector('meta[name="theme-color"]')?.setAttribute("content", getComputedStyle(root).getPropertyValue("--desk-canvas").trim());
      };
      if (changed && !isTauri() && typeof document.startViewTransition === "function" && !matchMedia("(prefers-reduced-motion: reduce)").matches) {
        appearanceTransition.current = document.startViewTransition(apply);
      } else {
        appearanceTransition.current = null;
        apply();
      }
      if (document.documentElement.dataset.nativeWindow === "macos" && nativeTheme !== theme) {
        nativeTheme = theme;
        void getCurrentWindow().setTheme(theme === "system" ? null : theme as "light" | "dark").catch(() => { nativeTheme = undefined; });
      }
    };
    paint(); media.addEventListener("change", paint);
    return () => media.removeEventListener("change", paint);
  }, [settings.data]);
  if (setupPending) {
    if (currentPlan.loading && !hasOnboardingDraft()) return <main id="workspace" className="desk-workspace"><span role="status">{t("Loading")}</span></main>;
    if (currentPlan.error && !hasOnboardingDraft()) return <main id="workspace" className="desk-workspace"><ErrorNotice error={currentPlan.error} retry={currentPlan.retry} /></main>;
    return <Onboarding onComplete={() => { try { setRunnerName(localStorage.getItem("runalytics.runnerName") ?? ""); } catch {} setSetupCompleted(true); navigate("training", "plan"); }} />;
  }
  const title = workspaces.find(entry => entry.id === route.workspace)?.title ?? t(route.workspace === "settings" ? "settings" : "help");
  return <>
    <span className="native-window-title" aria-hidden="true">Runalytics</span>
    <a lang={locale} className="desk-skip" href="#workspace" onClick={event => { event.preventDefault(); document.getElementById("workspace")?.focus(); }}>{t("skip")}</a>
    <header lang={locale} className="desk-appbar">
      <a href="#training" className="desk-brand" aria-label={t("brand")}>
        <BrandMark />
        <span className="desk-brand-name brand-wordmark" data-entrance>Runalytics</span>
      </a>
      <SelectionGroup><nav aria-label={t("workspaces")} className="desk-navigation">{workspaces.map(({ id, title: label, Icon }) => <a key={id} href={`#${id}`} aria-label={label} title={label} aria-current={route.workspace === id ? "page" : undefined}>{route.workspace === id && <SelectionMarker identity={compact && !reduced ? "workspace-selection" : undefined} />}<motion.div className="navigation-icon" aria-hidden="true" initial={false} animate={{ y: compact && !reduced && route.workspace === id ? -2 : 0, scale: compact && !reduced && route.workspace === id ? 1.08 : 1 }} transition={reduced ? { duration: 0 } : tempo.spring}><Icon size={21} /></motion.div><span>{label}</span></a>)}</nav></SelectionGroup>
      <div className="desk-utilities">
        <IconButton label={syncing ? t("syncing") : t("sync")} disabled={syncing || command.busy || !providers.data?.some(provider => provider.connected)} onClick={() => void command.run(async () => {
          const report = await tauri.syncNow("coros");
          notify(t("{{value0}} new activities, {{value1}} health days synced. {{value2}}", { value0: report.activitiesNew, value1: report.healthDays, value2: report.messages.join(" ") }));
        })}><IconRefresh size={20} className={syncing || command.busy ? "spinner spinner-reverse" : ""} /></IconButton>
        <IconButton label={t("help")} aria-pressed={route.workspace === "help"} onClick={() => navigate("help")}><IconHelp size={21} /></IconButton>
        <IconButton label={t("settings")} aria-pressed={route.workspace === "settings"} onClick={() => navigate("settings")}><IconSettings size={21} /></IconButton>
      </div>
    </header>
    <main id="workspace" tabIndex={-1} className={`desk-workspace workspace-${route.workspace}`}>
      <div lang={locale} className="workspace-heading"><h1 ref={heading} tabIndex={-1}>{title}</h1>{runnerName ? <span className="runner-greeting">{t("Hello, {{value0}}", { value0: runnerName })}</span> : <span className="connection-state">{providers.data?.some(provider => provider.connected) ? t("connected") : t("local")}</span>}</div>
      <ErrorNotice error={command.error} />
      <FeedbackReminder />
      <Change identity={route.workspace} direction={direction}>{(compact ? visitedWorkspaces : [route.workspace]).map(workspace => <Activity key={workspace} mode={route.workspace === workspace ? "visible" : "hidden"}>{workspace === "training" ? <Training /> : workspace === "journal" ? <Journal /> : workspace === "pace" ? <PaceInsightsView /> : workspace === "milestones" ? <Milestones /> : workspace === "settings" ? <SettingsWorkspace /> : <Help />}</Activity>)}</Change>
    </main>
  </>;
}

createRoot(document.getElementById("desk-root")!).render(<MotionSystem><WorkspaceProvider><App /><BrandSplash /></WorkspaceProvider></MotionSystem>);