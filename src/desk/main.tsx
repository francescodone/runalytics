import { useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { IconCalendarWeek, IconRun, IconHeart, IconSettings, IconHelp, IconRefresh } from "@tabler/icons-react";
import { motion, useReducedMotion } from "motion/react";
import { tauri } from "../lib/ipc";
import { navigate, useCommand, useRemote, useRoute, useWorkspace, WorkspaceProvider } from "./data";
import { ErrorNotice, IconButton } from "./ui";
import "./desk.css";
import { Training } from "./training";
import { Journal } from "./journal";
import { Health } from "./health";
import { SettingsWorkspace, Help } from "./settings";
import { Change, MotionSystem, SelectionGroup, SelectionMarker, tempo, useCompactLayout } from "./interaction";
import "./interaction.css";

const workspaces = [
  { id: "training", title: "Training", Icon: IconCalendarWeek },
  { id: "journal", title: "Activities", Icon: IconRun },
  { id: "health", title: "Health", Icon: IconHeart },
] as const;

function App() {
  const route = useRoute();
  const compact = useCompactLayout();
  const reduced = useReducedMotion();
  const heading = useRef<HTMLHeadingElement>(null);
  const initial = useRef(true);
  const settings = useRemote(tauri.getSettings);
  const providers = useRemote(tauri.providerStatus);
  const command = useCommand();
  const { syncing, notify } = useWorkspace();
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
  }, [route.workspace]);
  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");
    let nativeTheme: string | undefined;
    const paint = () => {
      let cachedTheme = "system";
      let cachedPalette = "reference";
      try {
        cachedTheme = localStorage.getItem("runalytics.theme") ?? "system";
        cachedPalette = localStorage.getItem("runalytics.palette") ?? "reference";
        if (settings.data) {
          localStorage.setItem("runalytics.theme", settings.data.theme ?? "system");
          localStorage.setItem("runalytics.palette", settings.data.palette ?? "reference");
        }
      } catch {}
      const theme = settings.data?.theme ?? cachedTheme;
      document.documentElement.dataset.deskTheme = theme === "system" ? (media.matches ? "dark" : "light") : theme;
      document.documentElement.style.colorScheme = document.documentElement.dataset.deskTheme;
      document.documentElement.dataset.deskPalette = settings.data?.palette ?? cachedPalette;
      if (document.documentElement.dataset.nativeWindow === "macos" && nativeTheme !== theme) {
        nativeTheme = theme;
        void getCurrentWindow().setTheme(theme === "system" ? null : theme as "light" | "dark").catch(() => { nativeTheme = undefined; });
      }
      document.querySelector('meta[name="theme-color"]')?.setAttribute("content", getComputedStyle(document.documentElement).getPropertyValue("--desk-canvas").trim());
    };
    paint(); media.addEventListener("change", paint);
    return () => media.removeEventListener("change", paint);
  }, [settings.data]);
  const title = workspaces.find(entry => entry.id === route.workspace)?.title ?? route.workspace[0].toUpperCase() + route.workspace.slice(1);
  return <>
    <span className="native-window-title" aria-hidden="true">Runalytics</span>
    <a className="desk-skip" href="#workspace" onClick={event => { event.preventDefault(); document.getElementById("workspace")?.focus(); }}>Skip to workspace</a>
    <header className="desk-appbar">
      <a href="#training" className="desk-brand" aria-label="Runalytics training">
        <span className="desk-brand-icon" data-entrance aria-hidden="true"><svg viewBox="0 0 64 64" width="44" height="44" fill="currentColor" focusable="false">
          <path d="M32 4C8 4 4 8 4 32s4 28 28 28 28-4 28-28S56 4 32 4Z" />
          <rect x="12" y="12" width="40" height="40" rx="15" fill="var(--desk-on-action)" />
          <path d="M23.044 40.875 27.481 23.125H35.606Q37.581 23.125 38.879 23.775Q40.176 24.425 40.678 25.608Q41.18 26.79 40.777 28.4Q40.467 29.64 39.735 30.598Q39.004 31.555 37.956 32.165Q36.909 32.775 35.651 32.965L35.642 32.72Q37.467 32.72 38.2 33.5Q38.932 34.28 38.655 35.83L37.814 40.875H33.379L34.169 36.555Q34.359 35.555 33.96 35.08Q33.561 34.605 32.291 34.605H28.961L27.394 40.875ZM29.851 31.045H32.961Q34.266 31.045 35.11 30.47Q35.954 29.895 36.215 28.85Q36.482 27.78 35.914 27.232Q35.346 26.685 33.996 26.685H30.941Z" />
        </svg></span>
        <span className="desk-brand-name" data-entrance>Runalytics</span>
      </a>
      <SelectionGroup><nav aria-label="Workspaces" className="desk-navigation">{workspaces.map(({ id, title: label, Icon }) => <a key={id} href={`#${id}`} aria-label={label} title={label} aria-current={route.workspace === id ? "page" : undefined}>{route.workspace === id && <SelectionMarker identity={compact && !reduced ? "workspace-selection" : undefined} />}<motion.div className="navigation-icon" aria-hidden="true" initial={false} animate={{ y: compact && !reduced && route.workspace === id ? -2 : 0, scale: compact && !reduced && route.workspace === id ? 1.08 : 1 }} transition={reduced ? { duration: 0 } : tempo.spring}><Icon size={21} /></motion.div><span>{label}</span></a>)}</nav></SelectionGroup>
      <div className="desk-utilities">
        <IconButton label={syncing ? "Syncing activities" : "Sync activities"} disabled={syncing || command.busy || !providers.data?.some(provider => provider.connected)} onClick={() => void command.run(async () => {
          const report = await tauri.syncNow("coros");
          notify(`${report.activitiesNew} new activities, ${report.healthDays} health days synced. ${report.messages.join(" ")}`);
        })}><IconRefresh size={20} className={syncing || command.busy ? "spinner spinner-reverse" : ""} /></IconButton>
        <IconButton label="Help" aria-pressed={route.workspace === "help"} onClick={() => navigate("help")}><IconHelp size={21} /></IconButton>
        <IconButton label="Settings" aria-pressed={route.workspace === "settings"} onClick={() => navigate("settings")}><IconSettings size={21} /></IconButton>
      </div>
    </header>
    <main id="workspace" tabIndex={-1} className={`desk-workspace workspace-${route.workspace}`}>
      <div className="workspace-heading"><h1 ref={heading} tabIndex={-1}>{title}</h1><span className="connection-state">{providers.data?.some(provider => provider.connected) ? "COROS connected" : "Local training workspace"}</span></div>
      <ErrorNotice error={command.error} />
      <Change identity={route.workspace}>{route.workspace === "training" ? <Training /> : route.workspace === "journal" ? <Journal /> : route.workspace === "health" ? <Health /> : route.workspace === "settings" ? <SettingsWorkspace /> : <Help />}</Change>
    </main>
  </>;
}

createRoot(document.getElementById("desk-root")!).render(<MotionSystem><WorkspaceProvider><App /></WorkspaceProvider></MotionSystem>);