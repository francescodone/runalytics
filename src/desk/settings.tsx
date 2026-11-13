import { t, text, languagePreference, dateTime as fmtDateTime, distance as fmtKm } from "./i18n";
import { useEffect, useState, type ReactNode } from "react";
import { IconArrowLeft, IconChevronDown, IconChevronRight, IconDeviceWatch, IconPalette, IconLanguage, IconAdjustments, IconMap, IconCalendarShare, IconCalendarEvent, IconInfoCircle, IconSearch, IconCopy, IconFolder, IconRefresh, IconBrandStrava, IconLink, IconUnlink, IconDeviceFloppy } from "@tabler/icons-react";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { tauri, inTauri } from "../lib/ipc";
import type { CalendarPublishing, PaletteId, Settings, Thresholds } from "../lib/types";
import { planName as displayPlanName } from "./i18n";
import { navigate, patchSettings, useCommand, useRemote, useRoute, useWorkspace } from "./data";
import { Button, Dialog, Empty, ErrorNotice, InfoDialog, Loading } from "./ui";
import { Change, Disclosure, Presence } from "./interaction";
import { ConnectionTransfer } from "./connection-transfer";

const tasks = [
  { id: "watch", title: t("Watch connection"), detail: t("COROS, sign-in, history sync, API diagnostics"), Icon: IconDeviceWatch },
  { id: "strava", title: "Strava", detail: t("Connection, paired activity descriptions, sync status"), Icon: IconBrandStrava },
  { id: "appearance", title: t("Appearance"), detail: t("System theme, light, dark, colour palette"), Icon: IconPalette },
  { id: "language", title: t("Language"), detail: languagePreference() === "system" ? t("System default") : languagePreference().toUpperCase(), Icon: IconLanguage },
  { id: "thresholds", title: t("Training boundaries"), detail: t("Load, recovery, build cycle, pace tolerance"), Icon: IconAdjustments },
  { id: "training", title: t("Training data"), detail: t("Clear plans and scheduled training, local and synced copies"), Icon: IconCalendarEvent },
  { id: "calendar", title: t("Calendar apps"), detail: t("Read-only subscription, calendar link, file snapshot"), Icon: IconCalendarShare },
  { id: "devices", title: t("Device sync"), detail: t("Keep your Runalytics plan up to date across devices"), Icon: IconRefresh },
  { id: "transfer", title: t("Transfer connections"), detail: t("Encrypted QR export and import, calendar, GitHub and Strava credentials"), Icon: IconCopy },
  { id: "maps", title: t("Offline maps"), detail: t("Local PMTiles archive and map source"), Icon: IconMap },
  { id: "about", title: t("About and updates"), detail: t("App version, releases, install an update"), Icon: IconInfoCircle },
];

function SetupOrDialog({ inline, title, onClose, children }: { inline: boolean; title: string; onClose: () => void; children: ReactNode }) {
  return inline ? <section className="settings-section" aria-label={title}><h2>{title}</h2>{children}</section> : <Dialog title={title} onClose={onClose}>{children}</Dialog>;
}

export function SettingsWorkspace() {
  const route = useRoute();
  const [query, setQuery] = useState("");
  const aliases: Record<string, string> = { setup: "watch", zones: "thresholds", tiles: "maps", theme: "appearance", publishing: "calendar" };
  const section = aliases[route.section] ?? route.section;
  const selected = tasks.find(task => task.id === section);
  const connectionIds = ["watch", "strava", "calendar", "devices", "transfer"];
  const groups = [
    { id: "connections", title: t("Third-party connections"), items: ["watch", "transfer", "devices", "calendar", "strava"].map(id => tasks.find(task => task.id === id)!) },
    { id: "general", title: t("General"), items: tasks.filter(task => !connectionIds.includes(task.id)) },
  ].map(group => ({...group, items:group.items.filter(task => `${group.title} ${task.title} ${task.detail}`.toLowerCase().includes(query.toLowerCase()))}));
  return <Change identity={section}>{selected ? <section className="settings-editor"><Button onClick={() => navigate("settings")}><IconArrowLeft size={18} />{t("All settings")}</Button><header><selected.Icon size={24} aria-hidden="true" /><h2>{selected.title}</h2></header>
    {section === "watch" ? <WatchSettings /> : section === "strava" ? <StravaSettings /> : section === "appearance" ? <Appearance /> : section === "language" ? <Language /> : section === "thresholds" ? <TrainingBoundaries /> : section === "training" ? <TrainingData /> : section === "calendar" ? <CalendarSettings /> : section === "devices" ? <DeviceSync /> : section === "transfer" ? <ConnectionTransfer /> : section === "maps" ? <MapSettings /> : <About />}
  </section> : <section className="settings-directory"><label className="search-field"><IconSearch size={20} /><input type="search" aria-label={t("Search settings")} value={query} onChange={event => setQuery(event.target.value)} placeholder={t("Find a setting")} /></label>{groups.filter(group => group.items.length > 0).map(group => <section className="settings-group" key={group.id} aria-labelledby={`settings-${group.id}`}><h2 id={`settings-${group.id}`}>{group.title}</h2><div className="settings-task-list">{group.items.map(task => <a key={task.id} href={`#settings/${task.id}`}><task.Icon size={25} aria-hidden="true" /><span><strong>{task.title}</strong><small>{task.detail}</small></span><IconChevronRight size={18} aria-hidden="true" /></a>)}</div></section>)}{groups.every(group => group.items.length === 0) && <Empty title={t("No matching settings")} action={<Button onClick={() => setQuery("")}>{t("Clear search")}</Button>} />}</section>}</Change>;
}

export function WatchSettings({ onboarding = false, onSyncChange }: { onboarding?: boolean; onSyncChange?: (active: boolean) => void } = {}) {
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
          setWaiting(false); setMessage(t("COROS connected.")); refresh(); return;
        }
        const result = await tauri.corosSigninNotice();
        if (!active) return;
        if (result && result !== "pending") { setWaiting(false); setMessage(result); }
        else if (Date.now() > deadline) { setWaiting(false); setMessage(t("Sign-in has not completed. Finish signing in, then try again.")); }
      } catch (error) { if (active && Date.now() > deadline) { setWaiting(false); setMessage(String(error)); } }
      finally { reading = false; }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 2500);
    return () => { active = false; window.clearInterval(timer); };
  }, [waiting]);
  return <><ErrorNotice error={status.error ?? notice.error ?? command.error} retry={status.retry} />{status.loading ? <Loading /> : <>
    <div className="connection-summary"><strong>{coros?.connected ? t("COROS connected") : t("COROS not connected")}</strong><p>{coros?.lastSync ? t("Last synced {{value0}}", { value0: fmtDateTime(coros.lastSync) }) : coros?.detail || t("No training history synced yet.")}</p></div>
    <div className="form-actions">{coros?.connected ? <><Button tone="primary" busy={command.busy} onClick={() => { onSyncChange?.(true); void command.run(async () => { const report = await tauri.syncNow("coros"); notify(t("{{value0}} new activities, {{value1}} health days. {{value2}}", { value0: report.activitiesNew, value1: report.healthDays, value2: report.messages.join(" ") })); }).finally(() => onSyncChange?.(false)); }}><IconRefresh size={18} />{t("Sync history")}</Button><Button tone="danger" onClick={() => setDisconnect(true)}>{t("Disconnect")}</Button></> : <Button tone="primary" busy={command.busy || waiting} onClick={async () => { setMessage(t("Finish signing in to COROS when it opens.")); setWaiting(true); if (!await command.run(() => tauri.connectProvider("coros"))) setWaiting(false); }}>{t("Connect COROS")}</Button>}</div>
    {(waiting || message) && <p role="status">{waiting ? t("Waiting for COROS sign-in...") : message}</p>}
    {coros?.connected && !onboarding && <a className="desk-button normal context-link" href="#training/plan">{t("Open your training plan")}<IconChevronRight size={18} /></a>}
    {!onboarding && <InfoDialog title={t("Connection diagnostics")} className="settings-section"><Button busy={command.busy} onClick={() => void command.run(async () => setTools(await tauri.checkProviderApi("coros")))}>{t("Check COROS API")}</Button><ErrorNotice error={command.error} />{tools && <><p role="status">{t("API reachable: {{value0}} tools exposed.", { value0: tools.length })}</p><ul className="plain-list">{tools.map(tool => <li key={tool}>{tool}</li>)}</ul></>}</InfoDialog>}
  </>}<Presence>{disconnect && <SetupOrDialog inline={onboarding} title={t("Disconnect COROS?")} onClose={() => setDisconnect(false)}><p>{t("Downloaded training history stays on this device. Future syncing requires signing in again.")}</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setDisconnect(false)}>{t("Keep connected")}</Button><Button tone="danger" busy={command.busy} onClick={async () => { if (await command.run(() => tauri.disconnectProvider("coros"), t("COROS disconnected."))) setDisconnect(false); }}>{t("Disconnect COROS")}</Button></div></SetupOrDialog>}</Presence></>;
}

export function StravaSettings({ mode = "all" }: { mode?: "all" | "connection" | "publishing" } = {}) {
  const remote = useRemote(tauri.stravaStatus);
  const command = useCommand();
  const [clientId, setClientId] = useState("");
  const [secret, setSecret] = useState("");
  const [confirmation, setConfirmation] = useState<"enable" | "disconnect" | "forget" | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [cancelError, setCancelError] = useState<string | null>(null);
  const data = remote.data;
  useEffect(() => { if (data) setClientId(data.clientId); }, [data?.clientId]);
  const run = async (action: () => Promise<unknown>, message: string) => {
    if (await command.run(action, message)) { remote.retry(); return true; }
    return false;
  };
  const connect = async () => {
    setConnecting(true); setCancelError(null);
    try { await run(tauri.stravaConnect, t("Strava connected. Description updates remain paused until enabled.")); }
    finally { setConnecting(false); }
  };
  const setupInformation = <><p>{t("Create your own API application in Strava settings. Set its Authorization Callback Domain to ")}<strong>{t("strava")}</strong>{t(" for iPhone; macOS uses the whitelisted loopback address. Enter that application's Client ID and Client Secret separately on each device.")}</p><p>{t("Credentials stay in this device's Keychain. No Runalytics server receives them. Connecting requires activity read and write access, including private activities.")}</p><p>{t("Automatic updates append the same text as Copy prescription, including results and training load, under the activity's existing Strava visibility. Existing notes are preserved. Only runs recorded after activation are eligible; nothing runs while Runalytics is closed.")}</p><p>{t("Another device's copied text is checked before writing. Simultaneous device or third-party edits can still race.")}</p><Button onClick={() => void command.run(() => openUrl("https://www.strava.com/settings/api"))}><IconLink size={18} />{t("Open Strava API settings")}</Button></>;
  return <><ErrorNotice error={remote.error ?? command.error ?? cancelError} retry={remote.retry} />{remote.loading && !data ? <Loading /> : data && <>
    <div className="connection-summary"><strong>{data.connected ? t("Strava connected: athlete {{value0}}", { value0: data.athleteId }) : t("Strava not connected")}</strong><p role="status">{(data.lastMessage && !data.lastMessage.startsWith("Strava:")) ? data.lastMessage : (data.configured ? t("API credentials saved in Keychain.") : t("API credentials not configured."))}</p></div>
    {mode === "connection" && <section className="settings-section"><h2>{t("Strava API setup and privacy")}</h2>{setupInformation}</section>}
    {mode !== "publishing" && <><form onSubmit={async event => {
      event.preventDefault();
      if (await run(() => tauri.stravaConfigure(clientId, secret || null), t("Strava API credentials saved."))) setSecret("");
    }}><div className="form-grid"><label>{t("Strava Client ID")}<input required inputMode="numeric" pattern="[0-9]+" value={clientId} onChange={event => setClientId(event.target.value)} autoComplete="off" /></label><label>{t("Strava Client Secret")}<input type="password" required={!data.configured} value={secret} onChange={event => setSecret(event.target.value)} autoComplete="new-password" placeholder={data.configured ? t("Saved in Keychain; leave blank to keep") : t("Not configured")} /></label></div>
      <Button type="submit" busy={command.busy}><IconDeviceFloppy size={18} />{t("Save API credentials")}</Button>
    </form>
    <div className="form-actions">{data.connected ? <><Button busy={command.busy} onClick={() => void connect()}><IconLink size={18} />{t("Reconnect Strava")}</Button><Button tone="danger" busy={command.busy} onClick={() => setConfirmation("disconnect")}><IconUnlink size={18} />{t("Disconnect Strava")}</Button></> : <Button tone="primary" busy={command.busy} disabled={!data.configured} onClick={() => void connect()}><IconBrandStrava size={18} />{t("Connect with Strava")}</Button>}
      {connecting && <Button onClick={() => void tauri.stravaCancelConnect().catch(error => setCancelError(String(error)))}>{t("Cancel sign-in")}</Button>}
      {data.configured && <Button busy={command.busy} onClick={() => setConfirmation("forget")}>{t("Remove API credentials")}</Button>}
    </div></>}
    {mode !== "connection" && <section className="settings-section"><h3>{t("Activity descriptions")}</h3><label className="toggle-setting"><input type="checkbox" checked={data.enabled} disabled={!data.connected || command.busy} onChange={event => { if (event.target.checked) setConfirmation("enable"); else void run(() => tauri.stravaSetEnabled(false), t("Strava description updates paused.")); }} />{t("Update paired activity descriptions")}</label>
      {data.enabledAt != null && <p className="muted">{t("Eligible runs: recorded from {{value0}}.", { value0: fmtDateTime(new Date(data.enabledAt * 1000).toISOString()) })}</p>}
      <h3>{t("Needs attention")} ({data.needsReview})</h3>
      <p className="muted">{t("These runs could not be updated. Try again or discard the pending description; discarding does not change Strava.")}</p>
      {(data.reviewItems ?? []).map(item => <div className="settings-section" key={item.sourceKey}><strong>{t("Run")} · {fmtDateTime(new Date(item.startTs * 1000).toISOString())} · {fmtKm(item.distanceM)}</strong><p>{text(item.message)}</p><div className="form-actions"><Button disabled={!data.connected || !data.enabled || !item.retryable} busy={command.busy} onClick={() => void run(() => tauri.stravaResolveReview(item.sourceKey, true), t("Try again"))}><IconRefresh size={18} />{t("Try again")}</Button><Button disabled={command.busy} onClick={() => void run(() => tauri.stravaResolveReview(item.sourceKey, false), t("Discard"))}>{t("Discard")}</Button></div></div>)}
    </section>}
    {mode === "all" && <InfoDialog title={t("Strava API setup and privacy")} className="settings-section">{setupInformation}</InfoDialog>}
  </>}<Presence>{confirmation && <SetupOrDialog inline={mode !== "all"} title={confirmation === "enable" ? t("Enable Strava description updates?") : confirmation === "forget" ? t("Remove Strava API credentials?") : t("Disconnect Strava?")} onClose={() => setConfirmation(null)}>
    {confirmation === "enable" ? <><p>{t("Future paired runs will receive the same text as Copy prescription, including results and training load. This text may be public if the Strava activity is public.")}</p><p>{t("Your existing descriptions stay. Older activities will not be updated.")}</p></> : <p>{t("Updates stop on this device. Existing Strava descriptions and local completion receipts stay. Authorization on other devices is not revoked.{{value0}}", { value0: confirmation === "forget" ? t(" The Client ID, Client Secret and local tokens will be removed from Keychain.") : t(" API credentials remain in Keychain for reconnecting.") })}</p>}
    <ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setConfirmation(null)}>{t("Cancel")}</Button><Button tone={confirmation === "enable" ? "primary" : "danger"} busy={command.busy} onClick={async () => {
      if (await run(() => confirmation === "enable" ? tauri.stravaSetEnabled(true) : tauri.stravaDisconnect(confirmation === "forget"), confirmation === "enable" ? t("Strava description updates enabled for new runs.") : t("Strava disconnected on this device."))) { setConfirmation(null); setSecret(""); }
    }}>{confirmation === "enable" ? t("Enable updates") : confirmation === "forget" ? t("Remove credentials") : t("Disconnect")}</Button></div>
  </SetupOrDialog>}</Presence></>;
}

function Language() {
  const [languageError, setLanguageError] = useState(false);
  return <><div className="form-grid"><label>{t("Language")}<select value={languagePreference()} onChange={event => { try { if (event.target.value === "system") localStorage.removeItem("runalytics.language"); else localStorage.setItem("runalytics.language", event.target.value); window.location.reload(); } catch { setLanguageError(true); } }}><option value="system">{t("System default")}</option><option value="en" lang="en">English</option><option value="es" lang="es">Español</option><option value="fr" lang="fr">Français</option><option value="de" lang="de">Deutsch</option><option value="it" lang="it">Italiano</option><option value="pt" lang="pt">Português</option><option value="ja" lang="ja">日本語</option><option value="zh" lang="zh">简体中文</option></select></label></div>{languageError && <p role="alert">{t("Could not save the language preference.")}</p>}</>;
}

function Appearance() {
  const settings = useRemote(tauri.getSettings);
  const command = useCommand();
  const palettes: {id: PaletteId; name: string; colour: string}[] = [
    {id:"cobalt",name:"Blue Steel Sprint",colour:"linear-gradient(135deg, #17375f, #3978bd)"},
    {id:"ember",name:"Burnt Toast Tempo",colour:"#9b4925"},
    {id:"ochre",name:"Golden Hour Hustle",colour:"linear-gradient(135deg, #493719, #ad8124)"},
    {id:"midnight",name:"Graphite Gallop",colour:"#525866"},
    {id:"glacier",name:"Ice Ice Pacey",colour:"#1d647b"},
    {id:"moss",name:"Moss Boss",colour:"linear-gradient(135deg, #263b24, #718e4e)"},
    {id:"sakura",name:"Pink It and Sprint",colour:"#943c65"},
    {id:"amethyst",name:"Purple Reign",colour:"#6844ab"},
    {id:"tide",name:"Teal We Meet Again",colour:"#176b6a"},
    {id:"evergreen",name:"The Green Mile",colour:"#16634b"},
    {id:"citrus",name:"Chasing Confetti",colour:"conic-gradient(#ad2835 0 40%, #e3b231 40% 75%, #244f86 75%)"},
    {id:"reference",name:"Fast and Curious",colour:"conic-gradient(#b6401e 0 25%, #2968ba 25% 50%, #be781d 50% 75%, #7571b7 75%)"},
    {id:"aurora",name:"Kaleidoscope Cadence",colour:"conic-gradient(#166b68 0 33%, #6d4aa4 33% 66%, #c55b52 66%)"},
    {id:"wildflower",name:"Miles of Mischief",colour:"conic-gradient(#963e6c 0 33%, #31715a 33% 66%, #a46912 66%)"},
    {id:"summit",name:"Sunrise Shenanigans",colour:"conic-gradient(#a84a2e 0 33%, #285fa5 33% 66%, #317756 66%)"},
    {id:"carnival",name:"Raspberry Relay",colour:"conic-gradient(#aa2858 0 33%, #08758a 33% 66%, #cf8a18 66%)"},
    {id:"electric",name:"Electric Stride",colour:"conic-gradient(#6541a5 0 33%, #688026 33% 66%, #167bac 66%)"},
    {id:"sorbet",name:"Sorbet Sprint",colour:"conic-gradient(#b34c16 0 33%, #a32d80 33% 66%, #157d88 66%)"},
    {id:"regatta",name:"Regatta Ready",colour:"conic-gradient(#24456f 0 33%, #a07800 33% 66%, #8c4f91 66%)"},
    {id:"meadow",name:"Meadow Medley",colour:"conic-gradient(#32713c 0 33%, #8561af 33% 66%, #b54a6a 66%)"},
  ];
  const savedPalette = settings.data?.palette ?? "reference";
  const selectedPalette = savedPalette === "garnet" ? "cobalt" : savedPalette === "coastline" ? "summit" : savedPalette;
  return <><ErrorNotice error={settings.error ?? command.error} retry={settings.retry} />{settings.loading ? <Loading /> : <><fieldset disabled={command.busy}><legend>{t("Theme")}</legend><div className="choice-row">{["system", "light", "dark"].map(theme => <label key={theme}><input type="radio" name="theme" value={theme} checked={(settings.data?.theme ?? "system") === theme} onChange={() => void command.run(() => patchSettings({theme: theme as Settings["theme"]}), t("Theme saved."))} />{t(theme[0].toUpperCase() + theme.slice(1))}</label>)}</div></fieldset><fieldset disabled={command.busy}><legend>{t("Colour palette")}</legend><div className="palette-options">{palettes.map(palette => <label key={palette.id}><input type="radio" name="palette" checked={selectedPalette === palette.id} onChange={() => void command.run(() => patchSettings({palette:palette.id}), t("Palette saved."))} /><span className="palette-swatch" style={{background:palette.colour}} /><span className="palette-name">{text(palette.name)}</span></label>)}</div></fieldset></>}</>;
}

const thresholdFields: {key: keyof Thresholds; label: string; min: number; max: number; step: number}[] = [
  {key:"ratioDecreasingHi",label:t("Decreasing load: upper ratio"),min:.2,max:.7,step:.05},
  {key:"ratioResumingHi",label:t("Resuming load: upper ratio"),min:.3,max:.95,step:.05},
  {key:"ratioMaintainingHi",label:t("Maintaining load: upper ratio"),min:.5,max:1.2,step:.05},
  {key:"ratioOptimizedHi",label:t("Optimized load: upper ratio"),min:.9,max:3,step:.05},
  {key:"recoveryExhausted",label:t("Exhausted: upper score"),min:0,max:95,step:1},
  {key:"recoveryFatigued",label:t("Fatigued: upper score"),min:1,max:97,step:1},
  {key:"recoveryNormal",label:t("Normal: upper score"),min:2,max:99,step:1},
  {key:"buildWeeks",label:t("Build weeks before recovery"),min:1,max:8,step:1},
  {key:"downWeekFactor",label:t("Recovery week load (%)"),min:40,max:100,step:1},
  {key:"paceToleranceSecKm",label:t("Pace tolerance (seconds/km)"),min:0,max:20,step:1},
  {key:"paceStreak",label:t("Quality sessions before adjustment"),min:2,max:6,step:1},
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
  const save = (thresholds: Thresholds) => command.run(async () => { const saved = await patchSettings({ thresholds }); setValues(saved.thresholds); }, t("Training boundaries saved."));
  return <form onSubmit={event => { event.preventDefault(); void save(values); }}><div className="form-grid">{thresholdFields.map(field => <label key={field.key}>{field.label}<input type="number" required min={field.min} max={field.max} step={field.step} value={field.key === "downWeekFactor" ? Math.round(values[field.key] * 100) : values[field.key]} onChange={event => setValues({...values,[field.key]:Number(event.target.value) / (field.key === "downWeekFactor" ? 100 : 1)})} /></label>)}</div><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setReset(true)}>{t("Restore defaults")}</Button><Button type="submit" tone="primary" busy={command.busy}>{t("Save boundaries")}</Button></div><Presence>{reset && <Dialog title={t("Restore training defaults?")} onClose={() => setReset(false)}><p>{t("Your custom load, recovery and pacing boundaries will be replaced with the standard values.")}</p><div className="form-actions"><Button onClick={() => setReset(false)}>{t("Keep changes")}</Button><Button tone="primary" busy={command.busy} onClick={async () => { if (await save(defaultThresholds)) setReset(false); }}>{t("Restore defaults")}</Button></div><ErrorNotice error={command.error} /></Dialog>}</Presence></form>;
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
  const scope = <ul className="plain-list">
    <li><strong>{t("This device's database:")}</strong>{t(" deletes all saved plans, including drafts and older plans, all their planned sessions (past and upcoming), and pending plan adjustments.")}</li>
    <li><strong>{t("GitHub Gist and other Runalytics devices:")}</strong>{t(" when device sync is enabled and connected, attempts to publish the cleared plan to the existing Gist. Other devices receive the clear on their next successful sync. The Gist itself is not deleted. With sync off or failing, other devices are not cleared.")}</li>
    <li><strong>{t("COROS:")}</strong>{t(" attempts to cancel the schedules of plans published from Runalytics. This does not delete recorded runs or remove the plan entry from COROS.")}</li>
    <li><strong>{t("Calendar:")}</strong>{t(" regenerates this device's calendar file and, when calendar publishing is enabled, attempts to upload the updated feed. Calendar apps fetch it on their own schedule; previously imported calendar snapshots are not removed.")}</li>
  </ul>;
  return <><ErrorNotice error={current.error ?? command.error} retry={current.retry} />{current.loading ? <Loading /> : <>
    <p className="muted">{plan ? t("This device holds one plan: {{value0}}.", { value0: displayPlanName(plan) }) : t("This device holds no plan right now.")}</p>
    <p>{t("This is not a local-only reset. It clears saved plans and their scheduled training here and can propagate the cleared plan to your other devices.")}</p>
    {scope}
    <p>{t("Recorded activities, GPS tracks, health history, pain entries, connection settings and credentials are kept. Remote updates can fail after local deletion; check sync results and external calendars afterward.")}</p>
    {plan && <a className="desk-button normal context-link" href="#training/plan">{t("Only end the current plan")}<IconChevronRight size={18} /></a>}
    <div className="form-actions"><Button tone="danger" disabled={!plan} onClick={() => setConfirm(true)}>{t("Clear plans and scheduled training")}</Button></div>
  </>}<Presence>{confirm && <Dialog title={t("Clear plans and scheduled training?")} onClose={() => setConfirm(false)}><p>{t("{{value0}} This cannot be undone.", { value0: plan ? t("{{value0}} and every other saved plan on this device will be deleted.", { value0: displayPlanName(plan) }) : t("All saved plans on this device will be deleted.") })}</p>{scope}<p>{t("Recorded activities, GPS tracks, health history, pain entries and connection credentials stay. This is not a local-only reset. Remote cleanup is not guaranteed; check sync results afterward.")}</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setConfirm(false)}>{t("Keep my plans")}</Button><Button tone="danger" busy={command.busy} onClick={async () => {
    if (await command.run(async () => { await tauri.clearLocalPlanData(); refresh(); })) { setConfirm(false); notify(t("Local plans and scheduled training cleared. Check sync results for other devices, COROS and your calendar. Recorded history is retained.")); }
  }}>{t("Clear plans and scheduled training")}</Button></div></Dialog>}</Presence></>;
}

export function CalendarSettings({ onboarding = false }: { onboarding?: boolean } = {}) {
  const data = useRemote(() => Promise.all([tauri.calendarPublishing(), tauri.icsPath()]));
  return <><ErrorNotice error={data.error} retry={data.retry} />{data.loading ? <Loading /> : data.data && <PublishingForm config={data.data[0]} path={data.data[1]} onboarding={onboarding} />}</>;
}
function PublishingForm({ config, path, onboarding }: { config: CalendarPublishing; path: string; onboarding: boolean }) {
  const [enabled, setEnabled] = useState(config.enabled);
  const [upload, setUpload] = useState(config.uploadUrl);
  const [subscription, setSubscription] = useState(config.subscriptionUrl);
  const [username, setUsername] = useState(config.username);
  const [secret, setSecret] = useState("");
  const [clear, setClear] = useState(false);
  const [setup, setSetup] = useState(false);
  const [options, setOptions] = useState(false);
  const [disconnect, setDisconnect] = useState(false);
  const command = useCommand();
  const { notify } = useWorkspace();
  const ready = Boolean(config.enabled && config.lastPublished && config.subscriptionUrl);
  const publish = () => command.run(async () => {
    await tauri.publishCalendar();
    const latest = await tauri.calendarPublishing();
    if (latest.lastError) throw new Error(latest.lastError);
  }, t("Calendar feed published. Your calendar app may take time to fetch it."));
  return <section><div className="connection-summary"><strong>{!config.enabled ? t("Calendar publishing is off") : text(config.lastError) ? t("Calendar publishing needs attention") : ready ? t("Calendar feed available") : t("Calendar feed not published yet")}</strong><p role="status">{config.lastError || (config.lastPublished ? t("Last published {{value0}}", { value0: fmtDateTime(config.lastPublished) }) : t("No calendar feed published."))}</p></div>
    <p className="muted">{t("A read-only subscription to your training schedule. Changes in your calendar app do not change your Runalytics plan. Your calendar app controls when it fetches updates.")}</p>
    <div className="form-actions">{!config.enabled ? <Button tone="primary" aria-expanded={setup} aria-controls="calendar-setup" onClick={() => { setEnabled(true); setSetup(!setup); }}><IconCalendarShare size={18} />{t("Connect a calendar")}</Button> : config.lastError || !ready ? <Button tone="primary" busy={command.busy} onClick={() => void publish()}><IconRefresh size={18} />{text(config.lastError) ? t("Retry publishing") : t("Publish calendar feed")}</Button> : <Button tone="primary" busy={command.busy} onClick={() => void command.run(() => openUrl(config.subscriptionUrl.replace(/^https?:/, "webcal:")))}><IconCalendarShare size={18} />{t("Open in calendar app")}</Button>}
      <Button className="connection-disclosure" aria-expanded={options} aria-controls="calendar-options" onClick={() => setOptions(!options)}><IconChevronDown className="disclosure-indicator" size={18} />{t("More options")}</Button></div>
    <ErrorNotice error={command.error} />
    <Disclosure id="calendar-options" open={options} className="connection-options"><div className="form-actions"><Button disabled={!ready} onClick={() => void command.run(async () => { await navigator.clipboard.writeText(config.subscriptionUrl); notify(t("Calendar link copied.")); })}><IconCopy size={18} />{t("Copy calendar link")}</Button><Button className="connection-disclosure" aria-expanded={setup} aria-controls="calendar-setup" onClick={() => setSetup(!setup)}><IconChevronDown className="disclosure-indicator" size={18} />{t("Connection settings")}</Button><Button disabled={!inTauri} onClick={() => void command.run(() => revealItemInDir(path))}><IconFolder size={18} />{t("Show calendar file")}</Button>{config.enabled && <Button tone="danger" onClick={() => setDisconnect(true)}><IconUnlink size={18} />{t("Stop publishing")}</Button>}</div><p className="muted">{t("A calendar file is a one-time snapshot, not an updating subscription.")}</p>{inTauri && <p className="file-path">{path}</p>}</Disclosure>
    <Disclosure id="calendar-setup" open={setup}><section className="settings-section"><h3>{t("Calendar hosting")}</h3><p className="muted">{t("Use an existing GitHub Gist or a calendar host that accepts uploads. Save the connection, then open the feed in your calendar app.")}</p><form onSubmit={async event => {
    event.preventDefault();
    if (await command.run(async () => {
      const latest = await tauri.calendarPublishing();
      await tauri.saveCalendarPublishing({...latest,enabled,uploadUrl:upload.trim(),subscriptionUrl:subscription.trim(),username:username.trim()}, clear ? "" : secret || null);
    }, t("Calendar settings saved."))) { setSecret(""); setClear(false); setSetup(false); }
  }}><fieldset disabled={command.busy}><label className="toggle-setting"><input type="checkbox" checked={enabled} onChange={event => setEnabled(event.target.checked)} />{t("Publish calendar remotely")}</label><div className="form-grid"><label className="full">{t("Upload URL")}<input type="url" required={enabled} value={upload} onChange={event => setUpload(event.target.value)} /></label><label className="full">{t("Subscription URL")}<input type="url" required={enabled} value={subscription} onChange={event => setSubscription(event.target.value)} /></label><label>{t("Username")}<input autoComplete="username" value={username} onChange={event => setUsername(event.target.value)} /></label><label>{t("Password or token")}<input type="password" autoComplete="new-password" value={secret} onChange={event => setSecret(event.target.value)} placeholder={config.hasPassword ? t("Saved in Keychain; leave blank to keep") : t("Not configured")} /></label></div><label className="toggle-setting"><input type="checkbox" checked={clear} onChange={event => setClear(event.target.checked)} />{t("Remove saved credential")}</label><Button type="submit" tone="primary" busy={command.busy}><IconDeviceFloppy size={18} />{t("Save calendar settings")}</Button></fieldset></form></section></Disclosure>
    <Presence>{disconnect && <SetupOrDialog inline={onboarding} title={t("Stop calendar publishing?")} onClose={() => setDisconnect(false)}><p>{t("Future changes will no longer be published. The existing feed and saved credentials are kept. Remove the subscription in your calendar app separately.")}</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setDisconnect(false)}>{t("Keep publishing")}</Button><Button tone="danger" busy={command.busy} onClick={async () => { if (await command.run(async () => { const latest = await tauri.calendarPublishing(); await tauri.saveCalendarPublishing({...latest,enabled:false}, null); }, t("Calendar publishing stopped."))) { setEnabled(false); setDisconnect(false); } }}>{t("Stop publishing")}</Button></div></SetupOrDialog>}</Presence></section>;
}

export function DeviceSync({ onRequestWatch }: { onRequestWatch?: () => void } = {}) {
  const { planCheck } = useWorkspace();
  const status = useRemote(async () => {
    const connection = await tauri.planSyncStatus();
    const ready = connection.config.enabled && connection.config.hasToken && connection.accountLinked;
    const [checked, current] = await Promise.all([
      (ready ? tauri.planSyncCheck() : Promise.resolve(null)).then(remote => ({remote, error:null}), error => ({remote:null, error:String(error)})),
      tauri.currentPlan(),
    ]);
    return {...connection, remote:checked.remote, checkError:checked.error, local:current?.[0]};
  }, [planCheck]);
  const command = useCommand();
  const { notify } = useWorkspace();
  const [conflict, setConflict] = useState<string | null>(null);
  const [setup, setSetup] = useState(false);
  const [options, setOptions] = useState(false);
  const [token, setToken] = useState("");
  const config = status.data?.config;
  const ready = Boolean(config?.enabled && config.hasToken && status.data?.accountLinked);
  const remote = status.data?.remote;
  const remoteStatus = remote?.status;
  const syncError = config?.lastError || status.data?.checkError;
  const needsAction = Boolean(syncError || (remoteStatus !== "upToDate" && (remote || status.data?.local)));
  const sync = () => command.run(async () => {
    await tauri.planSyncRefresh();
    const latest = await tauri.planSyncStatus();
    if (latest.config.lastError) throw new Error(latest.config.lastError);
  }, t("Device sync checked."));
  const pull = (force: boolean) => command.run(async () => {
    const result = await tauri.planSyncPull(force);
    if (result.conflict !== null) { setConflict(result.conflict); return; }
    setConflict(null);
    notify(result.installed ? t("Imported {{value0}} sessions: {{value1}}.", { value0: result.workouts, value1: result.planName }) : t("This device already has the newest plan."));
  });
  return <section>
    <ErrorNotice error={status.error ?? command.error} retry={status.retry} />
    {status.loading ? <Loading /> : config && <>
      <div className="connection-summary"><strong>{!config.hasToken ? t("Device sync not configured") : !config.enabled ? t("Device sync paused") : !status.data?.accountLinked ? t("Watch account required") : syncError ? t("Device sync needs attention") : !remote && !status.data?.local ? t("No plan to sync") : ({upToDate:t("Plan up to date"),unpublished:t("Changes waiting to sync"),updateAvailable:t("A newer plan version is available"),otherPlan:t("Different plans on your devices"),importAvailable:t("Your plan is ready to download")})[remoteStatus ?? "unpublished"]}</strong><p role="status">{config.lastChecked ? t("Last checked {{value0}}", { value0: fmtDateTime(config.lastChecked) }) : t("Not checked yet.")}</p></div>
      {ready && remote && remoteStatus !== "upToDate" && <p>{t("{{value0}} · {{value1}} sessions{{value2}}", { value0: remote.planName, value1: remote.workouts, value2: status.data?.local && remoteStatus === "otherPlan" ? t("; on this device: {{value0}}.", { value0: displayPlanName(status.data.local) }) : "." })}</p>}
      {!status.data?.accountLinked && (onRequestWatch ? <Button onClick={onRequestWatch}>{t("Connect COROS")}<IconChevronRight size={18} /></Button> : <a className="desk-button normal context-link" href="#settings/watch">{t("Connect COROS")}<IconChevronRight size={18} /></a>)}
      <div className="form-actions">{!config.hasToken ? <Button tone="primary" aria-expanded={setup} aria-controls="device-setup" onClick={() => setSetup(!setup)}><IconLink size={18} />{t("Set up device sync")}</Button> : ready && needsAction ? <Button tone="primary" busy={command.busy} onClick={() => void (remoteStatus === "otherPlan" || remoteStatus === "updateAvailable" || remoteStatus === "importAvailable" ? pull(false) : sync())}><IconRefresh size={18} />{syncError ? t("Retry sync") : remoteStatus === "otherPlan" ? t("Review plans") : remoteStatus === "updateAvailable" ? t("Update this device") : remoteStatus === "importAvailable" ? t("Get my plan") : t("Sync now")}</Button> : null}
        <Button className="connection-disclosure" aria-expanded={options} aria-controls="device-options" onClick={() => setOptions(!options)}><IconChevronDown className="disclosure-indicator" size={18} />{t("More options")}</Button></div>
      <label className="toggle-setting"><input type="checkbox" disabled={command.busy} checked={config.enabled} onChange={event => {
        const enabled = event.target.checked;
        void command.run(async () => {
          await tauri.planSyncConfigure({enabled}, null);
          if (enabled) window.dispatchEvent(new Event("runalytics:plan-sync-enabled"));
        }, t("Plan sync preference saved."));
      }} />{t("Sync this plan across devices")}</label>
      <p className="muted">{t("Use the same COROS account and GitHub token on each device. Calendar publishing is not required.")}</p>
      <Disclosure id="device-options" open={options} className="connection-options"><div className="form-actions">{!needsAction && <Button disabled={!ready} busy={command.busy} onClick={() => void sync()}><IconRefresh size={18} />{t("Sync now")}</Button>}<Button className="connection-disclosure" aria-expanded={setup} aria-controls="device-setup" onClick={() => setSetup(!setup)}><IconChevronDown className="disclosure-indicator" size={18} />{t("Connection settings")}</Button></div>{!onRequestWatch && <InfoDialog title={t("Sync diagnostics")}><dl className="definition-list">
        <div><dt>{t("Channel")}</dt><dd className="file-path">{status.data?.code ?? t("Unavailable until COROS is connected")}</dd></div>
        <div><dt>{t("Last checked")}</dt><dd>{config.lastChecked ? fmtDateTime(config.lastChecked) : t("Never")}</dd></div>
        <div><dt>{t("Last published")}</dt><dd>{config.lastPushed ? fmtDateTime(config.lastPushed) : t("Never")}</dd></div>
        <div><dt>{t("Last imported")}</dt><dd>{config.lastPulled ? fmtDateTime(config.lastPulled) : t("Never")}</dd></div>
      </dl></InfoDialog>}</Disclosure>
      <Disclosure id="device-setup" open={setup}><section className="settings-section"><h3>{t("GitHub connection")}</h3><p className="muted">{t("A personal access token with gist scope is needed on each device. It stays in Keychain and is also used by GitHub calendar publishing.")}</p><form onSubmit={async event => {
        event.preventDefault();
        if (await command.run(async () => {
          await tauri.planSyncConfigure({enabled:config.enabled || !config.hasToken}, token.trim() || null);
          window.dispatchEvent(new Event("runalytics:plan-sync-enabled"));
        }, t("Device sync connection saved."))) { setToken(""); setSetup(false); }
      }}><label>{t("GitHub token")}<input type="password" autoComplete="new-password" required={!config.hasToken} value={token} onChange={event => setToken(event.target.value)} placeholder={config.hasToken ? t("Saved in Keychain; leave blank to keep") : t("Token with gist scope")} /></label><Button type="submit" tone="primary" busy={command.busy}><IconDeviceFloppy size={18} />{t("Save device connection")}</Button></form></section></Disclosure>
      {syncError && <ErrorNotice error={syncError} />}
    </>}
    <Presence>{conflict !== null && <SetupOrDialog inline={Boolean(onRequestWatch)} title={t("Replace the plan on this device?")} onClose={() => setConflict(null)}><p>{text(conflict)}</p><p>{t("The published plan will replace this device's local plan. Your recorded activities are kept.")}</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setConflict(null)}>{t("Keep local plan")}</Button><Button tone="danger" busy={command.busy} onClick={() => void pull(true)}>{t("Replace local plan")}</Button></div></SetupOrDialog>}</Presence>
  </section>;
}

function MapSettings() {
  const settings = useRemote(tauri.getSettings);
  return <><ErrorNotice error={settings.error} retry={settings.retry} />{settings.loading ? <Loading /> : settings.data && <MapForm initial={settings.data.pmtilesPath} />}</>;
}
function MapForm({ initial }: { initial: string | null }) {
  const [path, setPath] = useState(initial ?? "");
  const command = useCommand();
  return <form onSubmit={event => { event.preventDefault(); void command.run(() => patchSettings({pmtilesPath:path.trim() || null}), t("Map source saved.")); }}><label>{t("Raster PMTiles archive")}<input value={path} onChange={event => setPath(event.target.value)} placeholder="/path/to/region.pmtiles" /></label><p className="muted">{t("Leave empty to use online OpenStreetMap tiles. Online tiles send map-area requests to the tile provider.")}</p><ErrorNotice error={command.error} /><div className="form-actions"><Button tone="primary" type="submit" busy={command.busy}>{t("Save map source")}</Button></div></form>;
}

function About() {
  const command = useCommand();
  const [pending, setPending] = useState<Update | null>(null);
  const [message, setMessage] = useState("");
  const [progress, setProgress] = useState<number | null>(null);
  const desktop = inTauri && !/iPhone|iPad|iPod/.test(navigator.userAgent);
  return <div className="about-settings"><div><h3>Runalytics {__APP_VERSION__}</h3><p className="muted">{t("Training analytics for runners.")}</p></div><ErrorNotice error={command.error} />{desktop && <div className="app-update"><div className="form-actions"><Button tone="primary" busy={command.busy} onClick={() => void command.run(async () => {
    if (!pending) { const update = await check(); setPending(update); setMessage(update ? t("{{value0}} is available. Installing will restart Runalytics.", { value0: update.version }) : t("You are on the latest version ({{value0}}).", { value0: __APP_VERSION__ })); return; }
    let bytes = 0;
    let total = 0;
    setProgress(null);
    setMessage(t("Downloading update..."));
    try {
      await pending.downloadAndInstall(event => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") { bytes += event.data.chunkLength; setProgress(total ? bytes / total * 100 : null); }
        if (event.event === "Finished") setMessage(t("Download complete. Installing..."));
      });
      setMessage(t("Installed. Restarting..."));
    } catch (error) {
      setProgress(null);
      setMessage(t("Update not installed. Try again or browse releases."));
      throw error;
    }
    await relaunch();
  })}>{pending ? t("Install {{value0}} and restart", { value0: pending.version }) : t("Check for updates")}</Button></div>{(progress != null || (command.busy && pending)) && <progress value={progress ?? undefined} max={100} aria-label={t("Update download")} />}{message && <p role="status">{message}</p>}</div>}<div className="form-actions"><Button onClick={() => void command.run(() => openUrl("https://github.com/francescodone/runalytics/releases"))}>{t("Browse releases")}</Button></div></div>;
}

export function Help() {
  const [query, setQuery] = useState("");
  const articles = [
    {title:t("A recent workout is missing"),text:t("Confirm that your watch has synced to COROS, then sync history in Runalytics. Keep the app open until the sync completes."),href:"#settings/watch",action:t("Check watch connection")},
    {title:t("Move or skip a session"),text:t("Open a session in Training to choose a new date or skip it. The backend can refuse moves that place hard sessions too close together. A successful change offers Undo."),href:"#training/week",action:t("Open training week")},
    {title:t("Send a plan to the watch"),text:t("A saved local plan is not necessarily installed on your watch. Send or update it from Training. Failed publishing can be retried without recreating the plan."),href:"#training/plan",action:t("Review plan")},
    {title:t("Bring the plan to another device"),text:t("Connect the same COROS account and save the same GitHub gist token on each device. Enable device sync. Both devices can edit the plan. Replacing a different local plan requires confirmation."),href:"#settings/devices",action:t("Manage device sync")},
    {title:t("Review pace against the prescription"),text:t("Activity reviews compare recorded laps with planned target ranges. The kilometre split table compares each split with the previous kilometre, not with the target."),href:"#journal",action:t("Open activities")},
  ];
  const matches = articles.filter(article => `${article.title} ${article.text}`.toLowerCase().includes(query.toLowerCase()));
  return <section className="help-workspace"><label className="search-field"><IconSearch size={20} /><input type="search" aria-label={t("Search help")} placeholder={t("What do you need help with?")} value={query} onChange={event => setQuery(event.target.value)} /></label><div className="help-articles">{matches.map(article => <InfoDialog key={article.title} title={article.title}><p>{article.text}</p><a className="desk-button normal context-link" href={article.href}>{article.action}<IconChevronRight size={18} /></a></InfoDialog>)}</div>{!matches.length && <Empty title={t("No matching topics")} action={<Button onClick={() => setQuery("")}>{t("Clear search")}</Button>} />}<p className="muted">Runalytics {__APP_VERSION__}</p></section>;
}