import { useEffect, useState } from "react";
import { IconArrowLeft, IconChevronRight, IconDeviceWatch, IconPalette, IconAdjustments, IconMap, IconCalendarShare, IconCalendarEvent, IconMessage, IconInfoCircle, IconSearch, IconCopy, IconFolder, IconRefresh } from "@tabler/icons-react";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { tauri, inTauri, fmtDateTime } from "../lib/ipc";
import type { CalendarPublishing, PaletteId, Settings, Thresholds } from "../lib/types";
import { displayPlanName } from "../lib/types";
import { navigate, patchSettings, useCommand, useRemote, useRoute, useWorkspace } from "./data";
import { Button, Dialog, Empty, ErrorNotice, InfoDialog, Loading } from "./ui";
import { Change, Presence } from "./interaction";

const tasks = [
  { id: "watch", title: "Watch connection", detail: "COROS, sign-in, history sync, API diagnostics", Icon: IconDeviceWatch },
  { id: "appearance", title: "Appearance", detail: "System theme, light, dark, colour palette", Icon: IconPalette },
  { id: "thresholds", title: "Training boundaries", detail: "Load, recovery, build cycle, pace tolerance", Icon: IconAdjustments },
  { id: "training", title: "Training data", detail: "Remove all saved plans, sessions and plan offers", Icon: IconCalendarEvent },
  { id: "calendar", title: "Calendar and devices", detail: "Calendar subscription, publishing, plan sync", Icon: IconCalendarShare },
  { id: "assistant", title: "Injury assistant", detail: "Provider, model, secure API key, privacy", Icon: IconMessage },
  { id: "maps", title: "Offline maps", detail: "Local PMTiles archive and map source", Icon: IconMap },
  { id: "about", title: "About and updates", detail: "App version, releases, install an update", Icon: IconInfoCircle },
];

export function SettingsWorkspace() {
  const route = useRoute();
  const [query, setQuery] = useState("");
  const aliases: Record<string, string> = { setup: "watch", zones: "thresholds", tiles: "maps", theme: "appearance", publishing: "calendar" };
  const section = aliases[route.section] ?? route.section;
  const selected = tasks.find(task => task.id === section);
  return <Change identity={section}>{selected ? <section className="settings-editor"><Button onClick={() => navigate("settings")}><IconArrowLeft size={18} />All settings</Button><header><selected.Icon size={24} aria-hidden="true" /><h2>{selected.title}</h2></header>
    {section === "watch" ? <WatchSettings /> : section === "appearance" ? <Appearance /> : section === "thresholds" ? <TrainingBoundaries /> : section === "training" ? <TrainingData /> : section === "calendar" ? <CalendarSettings /> : section === "assistant" ? <AssistantSettings /> : section === "maps" ? <MapSettings /> : <About />}
  </section> : <section className="settings-directory"><label className="search-field"><IconSearch size={20} /><input type="search" aria-label="Search settings" value={query} onChange={event => setQuery(event.target.value)} placeholder="Find a setting" /></label><div className="settings-task-list">{tasks.filter(task => `${task.title} ${task.detail}`.toLowerCase().includes(query.toLowerCase())).map(task => <a key={task.id} href={`#settings/${task.id}`}><task.Icon size={25} aria-hidden="true" /><span><strong>{task.title}</strong><small>{task.detail}</small></span><IconChevronRight size={18} aria-hidden="true" /></a>)}</div>{!tasks.some(task => `${task.title} ${task.detail}`.toLowerCase().includes(query.toLowerCase())) && <Empty title="No matching settings" action={<Button onClick={() => setQuery("")}>Clear search</Button>} />}</section>}</Change>;
}

function WatchSettings() {
  const status = useRemote(tauri.providerStatus);
  const notice = useRemote(tauri.corosSigninNotice);
  const { refresh, notify } = useWorkspace();
  const command = useCommand();
  const [waiting, setWaiting] = useState(false);
  const [message, setMessage] = useState("");
  const [tools, setTools] = useState<string[] | null>(null);
  const [disconnect, setDisconnect] = useState(false);
  const coros = status.data?.find(provider => provider.provider === "coros");
  useEffect(() => {
    if (notice.data === "pending") setWaiting(true);
    else if (notice.data) setMessage(notice.data);
  }, [notice.data]);
  useEffect(() => {
    if (!waiting) return;
    let active = true;
    let reading = false;
    const deadline = Date.now() + 300_000;
    const poll = async () => {
      if (reading || !active) return;
      reading = true;
      try {
        const providers = await tauri.providerStatus();
        if (!active) return;
        if (providers.some(provider => provider.provider === "coros" && provider.connected)) {
          setWaiting(false); setMessage("COROS connected."); refresh(); return;
        }
        const result = await tauri.corosSigninNotice();
        if (!active) return;
        if (result && result !== "pending") { setWaiting(false); setMessage(result); }
        else if (Date.now() > deadline) { setWaiting(false); setMessage("Sign-in has not completed. Finish signing in, then try again."); }
      } catch (error) { if (active && Date.now() > deadline) { setWaiting(false); setMessage(String(error)); } }
      finally { reading = false; }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 2500);
    return () => { active = false; window.clearInterval(timer); };
  }, [waiting]);
  return <><ErrorNotice error={status.error ?? notice.error ?? command.error} retry={status.retry} />{status.loading ? <Loading /> : <>
    <div className="connection-summary"><strong>{coros?.connected ? "COROS connected" : "COROS not connected"}</strong><p>{coros?.lastSync ? `Last synced ${fmtDateTime(coros.lastSync)}` : coros?.detail || "No training history synced yet."}</p></div>
    <div className="form-actions">{coros?.connected ? <><Button tone="primary" busy={command.busy} onClick={() => void command.run(async () => { const report = await tauri.syncNow("coros"); notify(`${report.activitiesNew} new activities, ${report.healthDays} health days. ${report.messages.join(" ")}`); })}><IconRefresh size={18} />Sync history</Button><Button tone="danger" onClick={() => setDisconnect(true)}>Disconnect</Button></> : <Button tone="primary" busy={command.busy || waiting} onClick={async () => { setMessage("Finish signing in to COROS when it opens."); setWaiting(true); if (!await command.run(() => tauri.connectProvider("coros"))) setWaiting(false); }}>Connect COROS</Button>}</div>
    {(waiting || message) && <p role="status">{waiting ? "Waiting for COROS sign-in..." : message}</p>}
    {coros?.connected && <a className="desk-button normal context-link" href="#training/plan">Open your training plan<IconChevronRight size={18} /></a>}
    <InfoDialog title="Connection diagnostics" className="settings-section"><Button busy={command.busy} onClick={() => void command.run(async () => setTools(await tauri.checkProviderApi("coros")))}>Check COROS API</Button><ErrorNotice error={command.error} />{tools && <><p role="status">API reachable: {tools.length} tools exposed.</p><ul className="plain-list">{tools.map(tool => <li key={tool}>{tool}</li>)}</ul></>}</InfoDialog>
  </>}<Presence>{disconnect && <Dialog title="Disconnect COROS?" onClose={() => setDisconnect(false)}><p>Downloaded training history stays on this device. Future syncing requires signing in again.</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setDisconnect(false)}>Keep connected</Button><Button tone="danger" busy={command.busy} onClick={async () => { if (await command.run(() => tauri.disconnectProvider("coros"), "COROS disconnected.")) setDisconnect(false); }}>Disconnect COROS</Button></div></Dialog>}</Presence></>;
}

function Appearance() {
  const settings = useRemote(tauri.getSettings);
  const command = useCommand();
  const palettes: {id: PaletteId; name: string; colour: string}[] = [{id:"reference",name:"Training Desk",colour:"#16634b"},{id:"amethyst",name:"Amethyst",colour:"#6844ab"},{id:"glacier",name:"Glacier",colour:"#1d647b"},{id:"evergreen",name:"Evergreen",colour:"#326344"},{id:"sakura",name:"Sakura",colour:"#943c65"},{id:"ember",name:"Ember",colour:"#9b4925"}];
  return <><ErrorNotice error={settings.error ?? command.error} retry={settings.retry} />{settings.loading ? <Loading /> : <><fieldset disabled={command.busy}><legend>Theme</legend><div className="choice-row">{["system", "light", "dark"].map(theme => <label key={theme}><input type="radio" name="theme" value={theme} checked={(settings.data?.theme ?? "system") === theme} onChange={() => void command.run(() => patchSettings({theme: theme as Settings["theme"]}), "Theme saved.")} />{theme[0].toUpperCase() + theme.slice(1)}</label>)}</div></fieldset><fieldset disabled={command.busy}><legend>Colour palette</legend><div className="palette-options">{palettes.map(palette => <label key={palette.id}><input type="radio" name="palette" checked={(settings.data?.palette ?? "reference") === palette.id} onChange={() => void command.run(() => patchSettings({palette:palette.id}), "Palette saved.")} /><span className="palette-swatch" style={{background:palette.colour}} />{palette.name}</label>)}</div></fieldset></>}</>;
}

const thresholdFields: {key: keyof Thresholds; label: string; min: number; max: number; step: number}[] = [
  {key:"ratioDecreasingHi",label:"Decreasing load: upper ratio",min:.2,max:.7,step:.05},
  {key:"ratioResumingHi",label:"Resuming load: upper ratio",min:.3,max:.95,step:.05},
  {key:"ratioMaintainingHi",label:"Maintaining load: upper ratio",min:.5,max:1.2,step:.05},
  {key:"ratioOptimizedHi",label:"Optimized load: upper ratio",min:.9,max:3,step:.05},
  {key:"recoveryExhausted",label:"Exhausted: upper score",min:0,max:95,step:1},
  {key:"recoveryFatigued",label:"Fatigued: upper score",min:1,max:97,step:1},
  {key:"recoveryNormal",label:"Normal: upper score",min:2,max:99,step:1},
  {key:"buildWeeks",label:"Build weeks before recovery",min:1,max:8,step:1},
  {key:"downWeekFactor",label:"Recovery week load (%)",min:40,max:100,step:1},
  {key:"paceToleranceSecKm",label:"Pace tolerance (seconds/km)",min:0,max:20,step:1},
  {key:"paceStreak",label:"Quality sessions before adjustment",min:2,max:6,step:1},
];
const defaultThresholds: Thresholds = {ratioDecreasingHi:.5,ratioResumingHi:.8,ratioMaintainingHi:1,ratioOptimizedHi:1.5,recoveryExhausted:19,recoveryFatigued:69,recoveryNormal:89,buildWeeks:3,downWeekFactor:.7,paceToleranceSecKm:4,paceStreak:3};
function TrainingBoundaries() {
  const settings = useRemote(tauri.getSettings);
  return <><ErrorNotice error={settings.error} retry={settings.retry} />{settings.loading ? <Loading /> : settings.data && <BoundaryForm initial={settings.data.thresholds} />}</>;
}
function BoundaryForm({ initial }: { initial: Thresholds }) {
  const [values, setValues] = useState(initial);
  const [reset, setReset] = useState(false);
  const command = useCommand();
  const save = (thresholds: Thresholds) => command.run(async () => { const saved = await patchSettings({ thresholds }); setValues(saved.thresholds); }, "Training boundaries saved.");
  return <form onSubmit={event => { event.preventDefault(); void save(values); }}><div className="form-grid">{thresholdFields.map(field => <label key={field.key}>{field.label}<input type="number" required min={field.min} max={field.max} step={field.step} value={field.key === "downWeekFactor" ? Math.round(values[field.key] * 100) : values[field.key]} onChange={event => setValues({...values,[field.key]:Number(event.target.value) / (field.key === "downWeekFactor" ? 100 : 1)})} /></label>)}</div><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setReset(true)}>Restore defaults</Button><Button type="submit" tone="primary" busy={command.busy}>Save boundaries</Button></div><Presence>{reset && <Dialog title="Restore training defaults?" onClose={() => setReset(false)}><p>Your custom load, recovery and pacing boundaries will be replaced with the standard values.</p><div className="form-actions"><Button onClick={() => setReset(false)}>Keep changes</Button><Button tone="primary" busy={command.busy} onClick={async () => { if (await save(defaultThresholds)) setReset(false); }}>Restore defaults</Button></div><ErrorNotice error={command.error} /></Dialog>}</Presence></form>;
}

/// The runner's "the plan keeps getting overridden, let me start clean"
/// escape hatch. Everything here is a mirror of the plan; the watch history
/// is not touched, so a clear never costs months of re-syncing.
function TrainingData() {
  const current = useRemote(tauri.currentPlan);
  const command = useCommand();
  const { refresh, notify } = useWorkspace();
  const [confirm, setConfirm] = useState(false);
  const plan = current.data?.[0];
  return <><ErrorNotice error={current.error ?? command.error} retry={current.retry} />{current.loading ? <Loading /> : <>
    <p className="muted">{plan ? `This device holds one plan: ${displayPlanName(plan)}.` : "This device holds no plan right now."}</p>
    <p>Remove every saved plan, including older plans and drafts, their scheduled sessions and pending plan offers. Recorded activities, health history and pain entries are kept.</p>
    <p>Runalytics will attempt to cancel published plans on COROS and update your calendar and connected devices so removed plans do not return on the next sync.</p>
    {plan && <a className="desk-button normal context-link" href="#training/plan">Only end the current plan<IconChevronRight size={18} /></a>}
    <div className="form-actions"><Button tone="danger" disabled={!plan} onClick={() => setConfirm(true)}>Remove all saved plans</Button></div>
  </>}<Presence>{confirm && <Dialog title="Remove all saved plans?" onClose={() => setConfirm(false)}><p>{plan ? `${displayPlanName(plan)}, any other saved plans and all their scheduled sessions will be deleted.` : "All saved plans and scheduled sessions will be deleted."} Pending plan offers are also removed. This cannot be undone.</p><p>Recorded activities, health history and pain entries stay. Runalytics will attempt to clear the published schedules on COROS and update your calendar and connected devices.</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setConfirm(false)}>Keep my plans</Button><Button tone="danger" busy={command.busy} onClick={async () => {
    if (await command.run(async () => { await tauri.clearLocalPlanData(); refresh(); }, "Saved plans removed.")) { setConfirm(false); notify("All saved plans removed. Your recorded history is retained."); }
  }}>Remove all saved plans</Button></div></Dialog>}</Presence></>;
}

function CalendarSettings() {
  const data = useRemote(() => Promise.all([tauri.calendarPublishing(), tauri.icsPath()]));
  return <><ErrorNotice error={data.error} retry={data.retry} />{data.loading ? <Loading /> : data.data && <PublishingForm config={data.data[0]} path={data.data[1]} />}<DeviceSync /></>;
}
function PublishingForm({ config, path }: { config: CalendarPublishing; path: string }) {
  const [enabled, setEnabled] = useState(config.enabled);
  const [upload, setUpload] = useState(config.uploadUrl);
  const [subscription, setSubscription] = useState(config.subscriptionUrl);
  const [username, setUsername] = useState(config.username);
  const [secret, setSecret] = useState("");
  const [clear, setClear] = useState(false);
  const command = useCommand();
  const { notify } = useWorkspace();
  const ready = Boolean(config.lastPublished && config.subscriptionUrl);
  return <section><h3>Calendar subscription</h3><p className="muted" role="status">{config.lastError || (config.lastPublished ? `Last published ${fmtDateTime(config.lastPublished)}` : "Not published yet.")}</p><form onSubmit={async event => {
    event.preventDefault();
    if (await command.run(async () => {
      const latest = await tauri.calendarPublishing();
      await tauri.saveCalendarPublishing({...latest,enabled,uploadUrl:upload.trim(),subscriptionUrl:subscription.trim(),username:username.trim()}, clear ? "" : secret || null);
    }, "Calendar settings saved.")) { setSecret(""); setClear(false); }
  }}><label className="toggle-setting"><input type="checkbox" checked={enabled} onChange={event => setEnabled(event.target.checked)} />Publish calendar remotely</label><div className="form-grid"><label className="full">Upload URL<input type="url" required={enabled} value={upload} onChange={event => setUpload(event.target.value)} /></label><label className="full">Subscription URL<input type="url" required={enabled} value={subscription} onChange={event => setSubscription(event.target.value)} /></label><label>Username<input autoComplete="username" value={username} onChange={event => setUsername(event.target.value)} /></label><label>Password or token<input type="password" autoComplete="new-password" value={secret} onChange={event => setSecret(event.target.value)} placeholder={config.hasPassword ? "Saved in Keychain; leave blank to keep" : "Not configured"} /></label></div><label className="toggle-setting"><input type="checkbox" checked={clear} onChange={event => setClear(event.target.checked)} />Remove saved credential</label><Button type="submit" tone="primary" busy={command.busy}>Save calendar settings</Button></form><ErrorNotice error={command.error} /><div className="form-actions"><Button busy={command.busy} onClick={() => void command.run(tauri.publishCalendar, "Calendar refreshed.")}><IconRefresh size={18} />Refresh calendar</Button><Button disabled={!ready} onClick={() => void command.run(async () => { await navigator.clipboard.writeText(config.subscriptionUrl); notify("Subscription URL copied."); })}><IconCopy size={18} />Copy subscription</Button><Button disabled={!ready} onClick={() => void command.run(() => openUrl(config.subscriptionUrl.replace(/^https:/, "webcal:")))}>Subscribe</Button></div><InfoDialog title="Local calendar file"><p className="file-path">{path}</p><Button disabled={!inTauri} onClick={() => void command.run(() => revealItemInDir(path))}><IconFolder size={18} />Reveal calendar</Button><ErrorNotice error={command.error} /></InfoDialog></section>;
}

function DeviceSync() {
  const status = useRemote(tauri.planSyncStatus);
  const command = useCommand();
  const { notify } = useWorkspace();
  const [conflict, setConflict] = useState<string | null>(null);
  const config = status.data?.config;
  const pull = (force: boolean) => command.run(async () => {
    const result = await tauri.planSyncPull(force);
    if (result.conflict !== null) { setConflict(result.conflict); return; }
    setConflict(null);
    notify(result.installed ? `Imported ${result.workouts} sessions: ${result.planName}.` : "This device already has the newest plan.");
  });
  return <section className="settings-section"><h3>Plan sync between devices</h3><ErrorNotice error={status.error ?? command.error} retry={status.retry} />{status.loading ? <Loading /> : config && <><label className="toggle-setting"><input type="checkbox" disabled={command.busy} checked={config.enabled} onChange={event => void command.run(() => tauri.planSyncConfigure({enabled:event.target.checked},null), "Plan sync preference saved.")} />Sync this plan across devices</label><p className="muted">{!status.data?.accountLinked ? "Connect the same COROS account on both devices." : !config.hasToken ? "Save a GitHub token with gist scope in calendar publishing on both devices." : "Both devices publish their edits and import the other's — the newest version wins."}</p><dl className="definition-list"><div><dt>Channel</dt><dd className="file-path">{status.data?.code ?? "Unavailable until COROS is connected"}</dd></div><div><dt>Last published</dt><dd>{config.lastPushed ? fmtDateTime(config.lastPushed) : "Never"}</dd></div><div><dt>Last imported</dt><dd>{config.lastPulled ? fmtDateTime(config.lastPulled) : "Never"}</dd></div></dl>{config.lastError && <ErrorNotice error={config.lastError} />}<div className="form-actions"><Button tone="primary" disabled={!config.enabled || !config.hasToken || !status.data?.accountLinked} busy={command.busy} onClick={() => void command.run(tauri.planSyncPush, "Plan published to the other device.")}>Publish plan</Button><Button disabled={!config.enabled || !config.hasToken || !status.data?.accountLinked} busy={command.busy} onClick={() => void pull(false)}>Import plan</Button></div></>}<Presence>{conflict !== null && <Dialog title="Replace the plan on this device?" onClose={() => setConflict(null)}><p>{conflict}</p><p>The published plan will replace this device's local plan. Your recorded activities are kept.</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setConflict(null)}>Keep local plan</Button><Button tone="danger" busy={command.busy} onClick={() => void pull(true)}>Replace local plan</Button></div></Dialog>}</Presence></section>;
}

function AssistantSettings() {
  const status = useRemote(tauri.llmStatus);
  const command = useCommand();
  const [endpoint, setEndpoint] = useState("");
  const [model, setModel] = useState("");
  const [key, setKey] = useState("");
  const [consent, setConsent] = useState(false);
  const [remove, setRemove] = useState(false);
  return <><ErrorNotice error={status.error ?? command.error} retry={status.retry} />{status.loading ? <Loading /> : status.data?.endpoint ? <><dl className="definition-list"><div><dt>Endpoint</dt><dd className="file-path">{status.data.endpoint}</dd></div><div><dt>Model</dt><dd>{status.data.model || "Provider default"}</dd></div><div><dt>API key</dt><dd>{status.data.hasKey ? "Stored in Keychain" : "No key stored"}</dd></div></dl><label className="toggle-setting"><input type="checkbox" checked={status.data.configured} disabled={command.busy} onChange={event => void command.run(() => patchSettings({llmEnabled:event.target.checked}), "Assistant preference saved.")} />Enable assistant</label><Button tone="danger" onClick={() => setRemove(true)}>Remove assistant</Button></> : <form onSubmit={async event => {
    event.preventDefault();
    if (!consent) return;
    await command.run(async () => {
      const url = new URL(endpoint.trim());
      if (url.protocol !== "https:" && !(url.protocol === "http:" && ["localhost","127.0.0.1","[::1]"].includes(url.hostname))) throw new Error("Use HTTPS, or HTTP for a localhost endpoint only.");
      if (key.trim()) await tauri.setLlmApiKey(key.trim());
      await patchSettings({llmEndpoint:url.href.replace(/\/$/,""),llmModel:model.trim() || null,llmEnabled:true});
      setKey("");
    }, "Assistant configured.");
  }}><div className="form-grid"><label className="full">OpenAI-compatible endpoint<input type="url" required value={endpoint} onChange={event => setEndpoint(event.target.value)} placeholder="https://api.openai.com/v1" /></label><label>Model<input value={model} onChange={event => setModel(event.target.value)} /></label><label>API key<input type="password" autoComplete="new-password" value={key} onChange={event => setKey(event.target.value)} /></label></div><label className="consent toggle-setting"><input type="checkbox" required checked={consent} onChange={event => setConsent(event.target.checked)} />I understand that questions can send my pain episode and conversation to this provider.</label><Button type="submit" tone="primary" busy={command.busy} disabled={!consent}>Add assistant</Button></form>}<Presence>{remove && <Dialog title="Remove this assistant?" onClose={() => setRemove(false)}><p>The provider configuration and saved API key will be removed. Pain entries and conversations stay on this device.</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setRemove(false)}>Keep assistant</Button><Button tone="danger" busy={command.busy} onClick={async () => { if (await command.run(async () => { await tauri.clearLlmApiKey(); await patchSettings({llmEnabled:false,llmEndpoint:null,llmModel:null}); }, "Assistant and API key removed.")) setRemove(false); }}>Remove assistant</Button></div></Dialog>}</Presence></>;
}

function MapSettings() {
  const settings = useRemote(tauri.getSettings);
  return <><ErrorNotice error={settings.error} retry={settings.retry} />{settings.loading ? <Loading /> : settings.data && <MapForm initial={settings.data.pmtilesPath} />}</>;
}
function MapForm({ initial }: { initial: string | null }) {
  const [path, setPath] = useState(initial ?? "");
  const command = useCommand();
  return <form onSubmit={event => { event.preventDefault(); void command.run(() => patchSettings({pmtilesPath:path.trim() || null}), "Map source saved."); }}><label>Raster PMTiles archive<input value={path} onChange={event => setPath(event.target.value)} placeholder="/path/to/region.pmtiles" /></label><p className="muted">Leave empty to use online OpenStreetMap tiles. Online tiles send map-area requests to the tile provider.</p><ErrorNotice error={command.error} /><div className="form-actions"><Button tone="primary" type="submit" busy={command.busy}>Save map source</Button></div></form>;
}

function About() {
  const command = useCommand();
  const [pending, setPending] = useState<Update | null>(null);
  const [message, setMessage] = useState("");
  const [progress, setProgress] = useState<number | null>(null);
  const desktop = inTauri && !/iPhone|iPad|iPod/.test(navigator.userAgent);
  return <><h3>Runalytics {__APP_VERSION__}</h3><p className="muted">Training analytics for runners.</p><ErrorNotice error={command.error} />{desktop && <Button tone="primary" busy={command.busy} onClick={() => void command.run(async () => {
    if (!pending) { const update = await check(); setPending(update); setMessage(update ? `${update.version} is available. Installing will restart Runalytics.` : `You are on the latest version (${__APP_VERSION__}).`); return; }
    let bytes = 0;
    let total = 0;
    await pending.downloadAndInstall(event => {
      if (event.event === "Started") total = event.data.contentLength ?? 0;
      if (event.event === "Progress") { bytes += event.data.chunkLength; setProgress(total ? bytes / total * 100 : null); }
      if (event.event === "Finished") setMessage("Installed. Restarting...");
    });
    await relaunch();
  })}>{pending ? `Install ${pending.version} and restart` : "Check for updates"}</Button>}{progress != null && <progress value={progress} max={100} aria-label="Update download" />}{message && <p role="status">{message}</p>}<div className="form-actions"><Button onClick={() => void command.run(() => openUrl("https://github.com/francescodone/runalytics/releases"))}>Browse releases</Button></div></>;
}

export function Help() {
  const [query, setQuery] = useState("");
  const articles = [
    {title:"A recent workout is missing",text:"Confirm that your watch has synced to COROS, then sync history in Runalytics. Keep the app open until the sync completes.",href:"#settings/watch",action:"Check watch connection"},
    {title:"Move or skip a session",text:"Open a session in Training to choose a new date or skip it. The backend can refuse moves that place hard sessions too close together. A successful change offers Undo.",href:"#training/week",action:"Open training week"},
    {title:"Send a plan to the watch",text:"A saved local plan is not necessarily installed on your watch. Send or update it from Training. Failed publishing can be retried without recreating the plan.",href:"#training/plan",action:"Review plan"},
    {title:"Bring the plan to another device",text:"Connect the same COROS account and save the same GitHub gist token on each device. Enable plan sync. The computer publishes; the receiving device imports. Replacing a different local plan requires confirmation.",href:"#settings/calendar",action:"Check calendar and devices"},
    {title:"Understand recovery and risk",text:"These indicators describe trends in synced training and health data. They are not a diagnosis. Log pain and seek medical advice for persistent, severe or concerning symptoms.",href:"#health/overview",action:"View health evidence"},
    {title:"Assistant privacy",text:"Questions can share the selected pain episode and its conversation with your configured provider. Keys are held in the system keychain. Removing the assistant does not delete journal entries; individual conversations can be cleared from their episode.",href:"#settings/assistant",action:"Manage assistant"},
    {title:"Review pace against the prescription",text:"Activity reviews compare recorded laps with planned target ranges. The kilometre split table compares each split with the previous kilometre, not with the target.",href:"#journal",action:"Open activities"},
  ];
  const matches = articles.filter(article => `${article.title} ${article.text}`.toLowerCase().includes(query.toLowerCase()));
  return <section className="help-workspace"><label className="search-field"><IconSearch size={20} /><input type="search" aria-label="Search help" placeholder="What do you need help with?" value={query} onChange={event => setQuery(event.target.value)} /></label><div className="help-articles">{matches.map(article => <InfoDialog key={article.title} title={article.title}><p>{article.text}</p><a className="desk-button normal context-link" href={article.href}>{article.action}<IconChevronRight size={18} /></a></InfoDialog>)}</div>{!matches.length && <Empty title="No matching topics" action={<Button onClick={() => setQuery("")}>Clear search</Button>} />}<p className="muted">Runalytics {__APP_VERSION__}</p></section>;
}