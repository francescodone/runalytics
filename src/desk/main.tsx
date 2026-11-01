import { useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";
import { IconCalendarWeek, IconRun, IconHeart, IconSettings, IconHelp, IconRefresh } from "@tabler/icons-react";
import { tauri } from "../lib/ipc";
import { navigate, useCommand, useRemote, useRoute, useWorkspace, WorkspaceProvider } from "./data";
import { ErrorNotice, IconButton } from "./ui";
import appIcon from "../../src-tauri/icons/128x128@2x.png";
import "./desk.css";
import { Training } from "./training";
import { Journal } from "./journal";
import { Health } from "./health";
import { SettingsWorkspace, Help } from "./settings";
import { Change, MotionSystem, SelectionGroup, SelectionMarker } from "./interaction";
import "./interaction.css";

const workspaces = [
  { id: "training", title: "Training", Icon: IconCalendarWeek },
  { id: "journal", title: "Activities", Icon: IconRun },
  { id: "health", title: "Health", Icon: IconHeart },
] as const;

function App() {
  const route = useRoute();
  const heading = useRef<HTMLHeadingElement>(null);
  const initial = useRef(true);
  const settings = useRemote(tauri.getSettings);
  const providers = useRemote(tauri.providerStatus);
  const command = useCommand();
  const { syncing, notify } = useWorkspace();
  useEffect(() => {
    if (initial.current) { initial.current = false; return; }
    heading.current?.focus({ preventScroll: true });
  }, [route.workspace]);
  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");
    const paint = () => {
      const theme = settings.data?.theme ?? localStorage.getItem("runalytics.theme") ?? "system";
      document.documentElement.dataset.deskTheme = theme === "system" ? (media.matches ? "dark" : "light") : theme;
      document.documentElement.dataset.deskPalette = settings.data?.palette ?? "reference";
    };
    paint(); media.addEventListener("change", paint);
    return () => media.removeEventListener("change", paint);
  }, [settings.data]);
  const title = workspaces.find(entry => entry.id === route.workspace)?.title ?? route.workspace[0].toUpperCase() + route.workspace.slice(1);
  return <>
    <a className="desk-skip" href="#workspace" onClick={event => { event.preventDefault(); document.getElementById("workspace")?.focus(); }}>Skip to workspace</a>
    <header className="desk-appbar">
      <a href="#training" className="desk-brand" aria-label="Runalytics training"><img src={appIcon} alt="" width="30" height="30" /><span>Runalytics</span></a>
      <SelectionGroup><nav aria-label="Workspaces" className="desk-navigation">{workspaces.map(({ id, title: label, Icon }) => <a key={id} href={`#${id}`} aria-current={route.workspace === id ? "page" : undefined}>{route.workspace === id && <SelectionMarker />}<Icon size={21} aria-hidden="true" /><span>{label}</span></a>)}</nav></SelectionGroup>
      <div className="desk-utilities">
        <IconButton label={syncing ? "Syncing activities" : "Sync activities"} disabled={syncing || command.busy || !providers.data?.some(provider => provider.connected)} onClick={() => void command.run(async () => {
          const report = await tauri.syncNow("coros");
          notify(`${report.activitiesNew} new activities, ${report.healthDays} health days synced. ${report.messages.join(" ")}`);
        })}><IconRefresh size={20} className={syncing || command.busy ? "spinner spinner-reverse" : ""} /></IconButton>
        <IconButton label="Help" onClick={() => navigate("help")}><IconHelp size={21} /></IconButton>
        <IconButton label="Settings" onClick={() => navigate("settings")}><IconSettings size={21} /></IconButton>
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