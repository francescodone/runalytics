import { useEffect, useState } from "react";
import { IconArrowLeft, IconChevronDown, IconChevronRight, IconDeviceWatch, IconPalette, IconAdjustments, IconMap, IconCalendarShare, IconCalendarEvent, IconMessage, IconInfoCircle, IconSearch, IconCopy, IconFolder, IconRefresh, IconBrandStrava, IconLink, IconUnlink, IconDeviceFloppy } from "@tabler/icons-react";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { tauri, inTauri, fmtDateTime } from "../lib/ipc";
import type { CalendarPublishing, PaletteId, Settings, Thresholds } from "../lib/types";
import { displayPlanName } from "../lib/types";
import { navigate, patchSettings, useCommand, useRemote, useRoute, useWorkspace } from "./data";
import { Button, Dialog, Empty, ErrorNotice, InfoDialog, Loading } from "./ui";
import { Change, Disclosure, Presence } from "./interaction";
import { ConnectionTransfer } from "./connection-transfer";

const tasks = [
  { id: "watch", title: "Watch connection", detail: "COROS, sign-in, history sync, API diagnostics", Icon: IconDeviceWatch },
  { id: "strava", title: "Strava", detail: "Connection, paired activity descriptions, sync status", Icon: IconBrandStrava },
  { id: "appearance", title: "Appearance", detail: "System theme, light, dark, colour palette", Icon: IconPalette },
  { id: "thresholds", title: "Training boundaries", detail: "Load, recovery, build cycle, pace tolerance", Icon: IconAdjustments },
  { id: "training", title: "Training data", detail: "Clear plans and scheduled training, local and synced copies", Icon: IconCalendarEvent },
  { id: "calendar", title: "Calendar apps", detail: "Read-only subscription, calendar link, file snapshot", Icon: IconCalendarShare },
  { id: "devices", title: "Device sync", detail: "Keep your Runalytics plan up to date across devices", Icon: IconRefresh },
  { id: "transfer", title: "Transfer connections", detail: "Encrypted QR export and import, calendar, GitHub and Strava credentials", Icon: IconCopy },
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
  const connectionIds = ["watch", "strava", "calendar", "devices", "transfer", "assistant"];
  const groups = [
    { id: "connections", title: "Third-party connections", items: ["watch", "transfer", "devices", "calendar", "strava", "assistant"].map(id => tasks.find(task => task.id === id)!) },
    { id: "general", title: "General", items: tasks.filter(task => !connectionIds.includes(task.id)) },
  ].map(group => ({...group, items:group.items.filter(task => `${group.title} ${task.title} ${task.detail}`.toLowerCase().includes(query.toLowerCase()))}));
  return <Change identity={section}>{selected ? <section className="settings-editor"><Button onClick={() => navigate("settings")}><IconArrowLeft size={18} />All settings</Button><header><selected.Icon size={24} aria-hidden="true" /><h2>{selected.title}</h2></header>
    {section === "watch" ? <WatchSettings /> : section === "strava" ? <StravaSettings /> : section === "appearance" ? <Appearance /> : section === "thresholds" ? <TrainingBoundaries /> : section === "training" ? <TrainingData /> : section === "calendar" ? <CalendarSettings /> : section === "devices" ? <DeviceSync /> : section === "transfer" ? <ConnectionTransfer /> : section === "assistant" ? <AssistantSettings /> : section === "maps" ? <MapSettings /> : <About />}
  </section> : <section className="settings-directory"><label className="search-field"><IconSearch size={20} /><input type="search" aria-label="Search settings" value={query} onChange={event => setQuery(event.target.value)} placeholder="Find a setting" /></label>{groups.filter(group => group.items.length > 0).map(group => <section className="settings-group" key={group.id} aria-labelledby={`settings-${group.id}`}><h2 id={`settings-${group.id}`}>{group.title}</h2><div className="settings-task-list">{group.items.map(task => <a key={task.id} href={`#settings/${task.id}`}><task.Icon size={25} aria-hidden="true" /><span><strong>{task.title}</strong><small>{task.detail}</small></span><IconChevronRight size={18} aria-hidden="true" /></a>)}</div></section>)}{groups.every(group => group.items.length === 0) && <Empty title="No matching settings" action={<Button onClick={() => setQuery("")}>Clear search</Button>} />}</section>}</Change>;
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

function StravaSettings() {
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
    try { await run(tauri.stravaConnect, "Strava connected. Description updates remain paused until enabled."); }
    finally { setConnecting(false); }
  };
  return <><ErrorNotice error={remote.error ?? command.error ?? cancelError} retry={remote.retry} />{remote.loading && !data ? <Loading /> : data && <>
    <div className="connection-summary"><strong>{data.connected ? `Strava connected: athlete ${data.athleteId}` : "Strava not connected"}</strong><p role="status">{data.lastMessage || (data.configured ? "API credentials saved in Keychain." : "API credentials not configured.")}</p></div>
    <form onSubmit={async event => {
      event.preventDefault();
      if (await run(() => tauri.stravaConfigure(clientId, secret || null), "Strava API credentials saved.")) setSecret("");
    }}><div className="form-grid"><label>Strava Client ID<input required inputMode="numeric" pattern="[0-9]+" value={clientId} onChange={event => setClientId(event.target.value)} autoComplete="off" /></label><label>Strava Client Secret<input type="password" required={!data.configured} value={secret} onChange={event => setSecret(event.target.value)} autoComplete="new-password" placeholder={data.configured ? "Saved in Keychain; leave blank to keep" : "Not configured"} /></label></div>
      <Button type="submit" busy={command.busy}><IconDeviceFloppy size={18} />Save API credentials</Button>
    </form>
    <div className="form-actions">{data.connected ? <><Button busy={command.busy} onClick={() => void connect()}><IconLink size={18} />Reconnect Strava</Button><Button tone="danger" busy={command.busy} onClick={() => setConfirmation("disconnect")}><IconUnlink size={18} />Disconnect Strava</Button></> : <Button tone="primary" busy={command.busy} disabled={!data.configured} onClick={() => void connect()}><IconBrandStrava size={18} />Connect with Strava</Button>}
      {connecting && <Button onClick={() => void tauri.stravaCancelConnect().catch(error => setCancelError(String(error)))}>Cancel sign-in</Button>}
      {data.configured && <Button busy={command.busy} onClick={() => setConfirmation("forget")}>Remove API credentials</Button>}
    </div>
    <section className="settings-section"><h3>Activity descriptions</h3><label className="toggle-setting"><input type="checkbox" checked={data.enabled} disabled={!data.connected || command.busy} onChange={event => { if (event.target.checked) setConfirmation("enable"); else void run(() => tauri.stravaSetEnabled(false), "Strava description updates paused."); }} />Update paired activity descriptions</label>
      {data.enabledAt != null && <p className="muted">Eligible runs: recorded from {fmtDateTime(new Date(data.enabledAt * 1000).toISOString())}.</p>}
      <dl className="metrics-line"><div><dt>Copied</dt><dd>{data.updated}</dd></div><div><dt>Already present</dt><dd>{data.alreadyPresent}</dd></div><div><dt>Pending</dt><dd>{data.pending}</dd></div><div><dt>Needs attention</dt><dd>{data.needsReview}</dd></div></dl>
      {data.lastAttemptAt != null && <p className="muted">Last attempt {fmtDateTime(new Date(data.lastAttemptAt * 1000).toISOString())}</p>}
      <Button disabled={!data.connected || !data.enabled || !data.pending} busy={command.busy} onClick={() => void run(tauri.stravaRetry, "Strava pending descriptions checked.")}><IconRefresh size={18} />Retry pending descriptions</Button>
    </section>
    <InfoDialog title="Strava API setup and privacy" className="settings-section"><p>Create your own API application in Strava settings. Set its Authorization Callback Domain to <strong>strava</strong> for iPhone; macOS uses the whitelisted loopback address. Enter that application's Client ID and Client Secret separately on each device.</p><p>Credentials stay in this device's Keychain. No Runalytics server receives them. Connecting requires activity read and write access, including private activities.</p><p>Automatic updates append the same text as Copy prescription, including results and any load or injury-risk lines, under the activity's existing Strava visibility. Existing notes are preserved. Only runs recorded after activation are eligible; nothing runs while Runalytics is closed.</p><p>Another device's copied text is checked before writing. Simultaneous device or third-party edits can still race.</p><Button onClick={() => void command.run(() => openUrl("https://www.strava.com/settings/api"))}><IconLink size={18} />Open Strava API settings</Button></InfoDialog>
  </>}<Presence>{confirmation && <Dialog title={confirmation === "enable" ? "Enable Strava description updates?" : confirmation === "forget" ? "Remove Strava API credentials?" : "Disconnect Strava?"} onClose={() => setConfirmation(null)}>
    {confirmation === "enable" ? <><p>Future paired runs will receive the same text as Copy prescription, including results and any training-load or injury-risk information. This text may be public if the Strava activity is public.</p><p>Your existing descriptions stay. Older activities will not be updated.</p></> : <p>Updates stop on this device. Existing Strava descriptions and local completion receipts stay. Authorization on other devices is not revoked.{confirmation === "forget" ? " The Client ID, Client Secret and local tokens will be removed from Keychain." : " API credentials remain in Keychain for reconnecting."}</p>}
    <ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setConfirmation(null)}>Cancel</Button><Button tone={confirmation === "enable" ? "primary" : "danger"} busy={command.busy} onClick={async () => {
      if (await run(() => confirmation === "enable" ? tauri.stravaSetEnabled(true) : tauri.stravaDisconnect(confirmation === "forget"), confirmation === "enable" ? "Strava description updates enabled for new runs." : "Strava disconnected on this device.")) { setConfirmation(null); setSecret(""); }
    }}>{confirmation === "enable" ? "Enable updates" : confirmation === "forget" ? "Remove credentials" : "Disconnect"}</Button></div>
  </Dialog>}</Presence></>;
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
  return <><ErrorNotice error={settings.error ?? command.error} retry={settings.retry} />{settings.loading ? <Loading /> : <><fieldset disabled={command.busy}><legend>Theme</legend><div className="choice-row">{["system", "light", "dark"].map(theme => <label key={theme}><input type="radio" name="theme" value={theme} checked={(settings.data?.theme ?? "system") === theme} onChange={() => void command.run(() => patchSettings({theme: theme as Settings["theme"]}), "Theme saved.")} />{theme[0].toUpperCase() + theme.slice(1)}</label>)}</div></fieldset><fieldset disabled={command.busy}><legend>Colour palette</legend><div className="palette-options">{palettes.map(palette => <label key={palette.id}><input type="radio" name="palette" checked={selectedPalette === palette.id} onChange={() => void command.run(() => patchSettings({palette:palette.id}), "Palette saved.")} /><span className="palette-swatch" style={{background:palette.colour}} /><span className="palette-name">{palette.name}</span></label>)}</div></fieldset></>}</>;
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
  const scope = <ul className="plain-list">
    <li><strong>This device's database:</strong> deletes all saved plans, including drafts and older plans, all their planned sessions (past and upcoming), and pending plan adjustments.</li>
    <li><strong>GitHub Gist and other Runalytics devices:</strong> when device sync is enabled and connected, attempts to publish the cleared plan to the existing Gist. Other devices receive the clear on their next successful sync. The Gist itself is not deleted. With sync off or failing, other devices are not cleared.</li>
    <li><strong>COROS:</strong> attempts to cancel the schedules of plans published from Runalytics. This does not delete recorded runs or remove the plan entry from COROS.</li>
    <li><strong>Calendar:</strong> regenerates this device's calendar file and, when calendar publishing is enabled, attempts to upload the updated feed. Calendar apps fetch it on their own schedule; previously imported calendar snapshots are not removed.</li>
  </ul>;
  return <><ErrorNotice error={current.error ?? command.error} retry={current.retry} />{current.loading ? <Loading /> : <>
    <p className="muted">{plan ? `This device holds one plan: ${displayPlanName(plan)}.` : "This device holds no plan right now."}</p>
    <p>This is not a local-only reset. It clears saved plans and their scheduled training here and can propagate the cleared plan to your other devices.</p>
    {scope}
    <p>Recorded activities, GPS tracks, health history, pain entries, connection settings and credentials are kept. Remote updates can fail after local deletion; check sync results and external calendars afterward.</p>
    {plan && <a className="desk-button normal context-link" href="#training/plan">Only end the current plan<IconChevronRight size={18} /></a>}
    <div className="form-actions"><Button tone="danger" disabled={!plan} onClick={() => setConfirm(true)}>Clear plans and scheduled training</Button></div>
  </>}<Presence>{confirm && <Dialog title="Clear plans and scheduled training?" onClose={() => setConfirm(false)}><p>{plan ? `${displayPlanName(plan)} and every other saved plan on this device will be deleted.` : "All saved plans on this device will be deleted."} This cannot be undone.</p>{scope}<p>Recorded activities, GPS tracks, health history, pain entries and connection credentials stay. This is not a local-only reset. Remote cleanup is not guaranteed; check sync results afterward.</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setConfirm(false)}>Keep my plans</Button><Button tone="danger" busy={command.busy} onClick={async () => {
    if (await command.run(async () => { await tauri.clearLocalPlanData(); refresh(); })) { setConfirm(false); notify("Local plans and scheduled training cleared. Check sync results for other devices, COROS and your calendar. Recorded history is retained."); }
  }}>Clear plans and scheduled training</Button></div></Dialog>}</Presence></>;
}

function CalendarSettings() {
  const data = useRemote(() => Promise.all([tauri.calendarPublishing(), tauri.icsPath()]));
  return <><ErrorNotice error={data.error} retry={data.retry} />{data.loading ? <Loading /> : data.data && <PublishingForm config={data.data[0]} path={data.data[1]} />}</>;
}
function PublishingForm({ config, path }: { config: CalendarPublishing; path: string }) {
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
  }, "Calendar feed published. Your calendar app may take time to fetch it.");
  return <section><div className="connection-summary"><strong>{!config.enabled ? "Calendar publishing is off" : config.lastError ? "Calendar publishing needs attention" : ready ? "Calendar feed available" : "Calendar feed not published yet"}</strong><p role="status">{config.lastError || (config.lastPublished ? `Last published ${fmtDateTime(config.lastPublished)}` : "No calendar feed published.")}</p></div>
    <p className="muted">A read-only subscription to your training schedule. Changes in your calendar app do not change your Runalytics plan. Your calendar app controls when it fetches updates.</p>
    <div className="form-actions">{!config.enabled ? <Button tone="primary" aria-expanded={setup} aria-controls="calendar-setup" onClick={() => { setEnabled(true); setSetup(!setup); }}><IconCalendarShare size={18} />Connect a calendar</Button> : config.lastError || !ready ? <Button tone="primary" busy={command.busy} onClick={() => void publish()}><IconRefresh size={18} />{config.lastError ? "Retry publishing" : "Publish calendar feed"}</Button> : <Button tone="primary" busy={command.busy} onClick={() => void command.run(() => openUrl(config.subscriptionUrl.replace(/^https?:/, "webcal:")))}><IconCalendarShare size={18} />Open in calendar app</Button>}
      <Button className="connection-disclosure" aria-expanded={options} aria-controls="calendar-options" onClick={() => setOptions(!options)}><IconChevronDown className="disclosure-indicator" size={18} />More options</Button></div>
    <ErrorNotice error={command.error} />
    <Disclosure id="calendar-options" open={options} className="connection-options"><div className="form-actions"><Button disabled={!ready} onClick={() => void command.run(async () => { await navigator.clipboard.writeText(config.subscriptionUrl); notify("Calendar link copied."); })}><IconCopy size={18} />Copy calendar link</Button><Button className="connection-disclosure" aria-expanded={setup} aria-controls="calendar-setup" onClick={() => setSetup(!setup)}><IconChevronDown className="disclosure-indicator" size={18} />Connection settings</Button><Button disabled={!inTauri} onClick={() => void command.run(() => revealItemInDir(path))}><IconFolder size={18} />Show calendar file</Button>{config.enabled && <Button tone="danger" onClick={() => setDisconnect(true)}><IconUnlink size={18} />Stop publishing</Button>}</div><p className="muted">A calendar file is a one-time snapshot, not an updating subscription.</p>{inTauri && <p className="file-path">{path}</p>}</Disclosure>
    <Disclosure id="calendar-setup" open={setup}><section className="settings-section"><h3>Calendar hosting</h3><p className="muted">Use an existing GitHub Gist or a calendar host that accepts uploads. Save the connection, then open the feed in your calendar app.</p><form onSubmit={async event => {
    event.preventDefault();
    if (await command.run(async () => {
      const latest = await tauri.calendarPublishing();
      await tauri.saveCalendarPublishing({...latest,enabled,uploadUrl:upload.trim(),subscriptionUrl:subscription.trim(),username:username.trim()}, clear ? "" : secret || null);
    }, "Calendar settings saved.")) { setSecret(""); setClear(false); setSetup(false); }
  }}><fieldset disabled={command.busy}><label className="toggle-setting"><input type="checkbox" checked={enabled} onChange={event => setEnabled(event.target.checked)} />Publish calendar remotely</label><div className="form-grid"><label className="full">Upload URL<input type="url" required={enabled} value={upload} onChange={event => setUpload(event.target.value)} /></label><label className="full">Subscription URL<input type="url" required={enabled} value={subscription} onChange={event => setSubscription(event.target.value)} /></label><label>Username<input autoComplete="username" value={username} onChange={event => setUsername(event.target.value)} /></label><label>Password or token<input type="password" autoComplete="new-password" value={secret} onChange={event => setSecret(event.target.value)} placeholder={config.hasPassword ? "Saved in Keychain; leave blank to keep" : "Not configured"} /></label></div><label className="toggle-setting"><input type="checkbox" checked={clear} onChange={event => setClear(event.target.checked)} />Remove saved credential</label><Button type="submit" tone="primary" busy={command.busy}><IconDeviceFloppy size={18} />Save calendar settings</Button></fieldset></form></section></Disclosure>
    <Presence>{disconnect && <Dialog title="Stop calendar publishing?" onClose={() => setDisconnect(false)}><p>Future changes will no longer be published. The existing feed and saved credentials are kept. Remove the subscription in your calendar app separately.</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setDisconnect(false)}>Keep publishing</Button><Button tone="danger" busy={command.busy} onClick={async () => { if (await command.run(async () => { const latest = await tauri.calendarPublishing(); await tauri.saveCalendarPublishing({...latest,enabled:false}, null); }, "Calendar publishing stopped.")) { setEnabled(false); setDisconnect(false); } }}>Stop publishing</Button></div></Dialog>}</Presence></section>;
}

function DeviceSync() {
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
  }, "Device sync checked.");
  const pull = (force: boolean) => command.run(async () => {
    const result = await tauri.planSyncPull(force);
    if (result.conflict !== null) { setConflict(result.conflict); return; }
    setConflict(null);
    notify(result.installed ? `Imported ${result.workouts} sessions: ${result.planName}.` : "This device already has the newest plan.");
  });
  return <section>
    <ErrorNotice error={status.error ?? command.error} retry={status.retry} />
    {status.loading ? <Loading /> : config && <>
      <div className="connection-summary"><strong>{!config.hasToken ? "Device sync not configured" : !config.enabled ? "Device sync paused" : !status.data?.accountLinked ? "Watch account required" : syncError ? "Device sync needs attention" : !remote && !status.data?.local ? "No plan to sync" : ({upToDate:"Plan up to date",unpublished:"Changes waiting to sync",updateAvailable:"A newer plan version is available",otherPlan:"Different plans on your devices",importAvailable:"Your plan is ready to download"})[remoteStatus ?? "unpublished"]}</strong><p role="status">{config.lastChecked ? `Last checked ${fmtDateTime(config.lastChecked)}` : "Not checked yet."}</p></div>
      {ready && remote && remoteStatus !== "upToDate" && <p>{remote.planName} · {remote.workouts} sessions{status.data?.local && remoteStatus === "otherPlan" ? `; on this device: ${displayPlanName(status.data.local)}.` : "."}</p>}
      {!status.data?.accountLinked && <a className="desk-button normal context-link" href="#settings/watch">Connect COROS<IconChevronRight size={18} /></a>}
      <div className="form-actions">{!config.hasToken ? <Button tone="primary" aria-expanded={setup} aria-controls="device-setup" onClick={() => setSetup(!setup)}><IconLink size={18} />Set up device sync</Button> : ready && needsAction ? <Button tone="primary" busy={command.busy} onClick={() => void (remoteStatus === "otherPlan" || remoteStatus === "updateAvailable" || remoteStatus === "importAvailable" ? pull(false) : sync())}><IconRefresh size={18} />{syncError ? "Retry sync" : remoteStatus === "otherPlan" ? "Review plans" : remoteStatus === "updateAvailable" ? "Update this device" : remoteStatus === "importAvailable" ? "Get my plan" : "Sync now"}</Button> : null}
        <Button className="connection-disclosure" aria-expanded={options} aria-controls="device-options" onClick={() => setOptions(!options)}><IconChevronDown className="disclosure-indicator" size={18} />More options</Button></div>
      <label className="toggle-setting"><input type="checkbox" disabled={command.busy} checked={config.enabled} onChange={event => {
        const enabled = event.target.checked;
        void command.run(async () => {
          await tauri.planSyncConfigure({enabled}, null);
          if (enabled) window.dispatchEvent(new Event("runalytics:plan-sync-enabled"));
        }, "Plan sync preference saved.");
      }} />Sync this plan across devices</label>
      <p className="muted">Use the same COROS account and GitHub token on each device. Calendar publishing is not required.</p>
      <Disclosure id="device-options" open={options} className="connection-options"><div className="form-actions">{!needsAction && <Button disabled={!ready} busy={command.busy} onClick={() => void sync()}><IconRefresh size={18} />Sync now</Button>}<Button className="connection-disclosure" aria-expanded={setup} aria-controls="device-setup" onClick={() => setSetup(!setup)}><IconChevronDown className="disclosure-indicator" size={18} />Connection settings</Button></div><InfoDialog title="Sync diagnostics"><dl className="definition-list">
        <div><dt>Channel</dt><dd className="file-path">{status.data?.code ?? "Unavailable until COROS is connected"}</dd></div>
        <div><dt>Last checked</dt><dd>{config.lastChecked ? fmtDateTime(config.lastChecked) : "Never"}</dd></div>
        <div><dt>Last published</dt><dd>{config.lastPushed ? fmtDateTime(config.lastPushed) : "Never"}</dd></div>
        <div><dt>Last imported</dt><dd>{config.lastPulled ? fmtDateTime(config.lastPulled) : "Never"}</dd></div>
      </dl></InfoDialog></Disclosure>
      <Disclosure id="device-setup" open={setup}><section className="settings-section"><h3>GitHub connection</h3><p className="muted">A personal access token with gist scope is needed on each device. It stays in Keychain and is also used by GitHub calendar publishing.</p><form onSubmit={async event => {
        event.preventDefault();
        if (await command.run(async () => {
          await tauri.planSyncConfigure({enabled:config.enabled || !config.hasToken}, token.trim() || null);
          window.dispatchEvent(new Event("runalytics:plan-sync-enabled"));
        }, "Device sync connection saved.")) { setToken(""); setSetup(false); }
      }}><label>GitHub token<input type="password" autoComplete="new-password" required={!config.hasToken} value={token} onChange={event => setToken(event.target.value)} placeholder={config.hasToken ? "Saved in Keychain; leave blank to keep" : "Token with gist scope"} /></label><Button type="submit" tone="primary" busy={command.busy}><IconDeviceFloppy size={18} />Save device connection</Button></form></section></Disclosure>
      {syncError && <ErrorNotice error={syncError} />}
    </>}
    <Presence>{conflict !== null && <Dialog title="Replace the plan on this device?" onClose={() => setConflict(null)}><p>{conflict}</p><p>The published plan will replace this device's local plan. Your recorded activities are kept.</p><ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={() => setConflict(null)}>Keep local plan</Button><Button tone="danger" busy={command.busy} onClick={() => void pull(true)}>Replace local plan</Button></div></Dialog>}</Presence>
  </section>;
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
  return <div className="about-settings"><div><h3>Runalytics {__APP_VERSION__}</h3><p className="muted">Training analytics for runners.</p></div><ErrorNotice error={command.error} />{desktop && <div className="app-update"><div className="form-actions"><Button tone="primary" busy={command.busy} onClick={() => void command.run(async () => {
    if (!pending) { const update = await check(); setPending(update); setMessage(update ? `${update.version} is available. Installing will restart Runalytics.` : `You are on the latest version (${__APP_VERSION__}).`); return; }
    let bytes = 0;
    let total = 0;
    setProgress(null);
    setMessage("Downloading update...");
    try {
      await pending.downloadAndInstall(event => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") { bytes += event.data.chunkLength; setProgress(total ? bytes / total * 100 : null); }
        if (event.event === "Finished") setMessage("Download complete. Installing...");
      });
      setMessage("Installed. Restarting...");
    } catch (error) {
      setProgress(null);
      setMessage("Update not installed. Try again or browse releases.");
      throw error;
    }
    await relaunch();
  })}>{pending ? `Install ${pending.version} and restart` : "Check for updates"}</Button></div>{(progress != null || (command.busy && pending)) && <progress value={progress ?? undefined} max={100} aria-label="Update download" />}{message && <p role="status">{message}</p>}</div>}<div className="form-actions"><Button onClick={() => void command.run(() => openUrl("https://github.com/francescodone/runalytics/releases"))}>Browse releases</Button></div></div>;
}

export function Help() {
  const [query, setQuery] = useState("");
  const articles = [
    {title:"A recent workout is missing",text:"Confirm that your watch has synced to COROS, then sync history in Runalytics. Keep the app open until the sync completes.",href:"#settings/watch",action:"Check watch connection"},
    {title:"Move or skip a session",text:"Open a session in Training to choose a new date or skip it. The backend can refuse moves that place hard sessions too close together. A successful change offers Undo.",href:"#training/week",action:"Open training week"},
    {title:"Send a plan to the watch",text:"A saved local plan is not necessarily installed on your watch. Send or update it from Training. Failed publishing can be retried without recreating the plan.",href:"#training/plan",action:"Review plan"},
    {title:"Bring the plan to another device",text:"Connect the same COROS account and save the same GitHub gist token on each device. Enable device sync. Both devices can edit the plan. Replacing a different local plan requires confirmation.",href:"#settings/devices",action:"Manage device sync"},
    {title:"Understand recovery and risk",text:"These indicators describe trends in synced training and health data. They are not a diagnosis. Log pain and seek medical advice for persistent, severe or concerning symptoms.",href:"#health/overview",action:"View health evidence"},
    {title:"Assistant privacy",text:"Questions can share the selected pain episode and its conversation with your configured provider. Keys are held in the system keychain. Removing the assistant does not delete journal entries; individual conversations can be cleared from their episode.",href:"#settings/assistant",action:"Manage assistant"},
    {title:"Review pace against the prescription",text:"Activity reviews compare recorded laps with planned target ranges. The kilometre split table compares each split with the previous kilometre, not with the target.",href:"#journal",action:"Open activities"},
  ];
  const matches = articles.filter(article => `${article.title} ${article.text}`.toLowerCase().includes(query.toLowerCase()));
  return <section className="help-workspace"><label className="search-field"><IconSearch size={20} /><input type="search" aria-label="Search help" placeholder="What do you need help with?" value={query} onChange={event => setQuery(event.target.value)} /></label><div className="help-articles">{matches.map(article => <InfoDialog key={article.title} title={article.title}><p>{article.text}</p><a className="desk-button normal context-link" href={article.href}>{article.action}<IconChevronRight size={18} /></a></InfoDialog>)}</div>{!matches.length && <Empty title="No matching topics" action={<Button onClick={() => setQuery("")}>Clear search</Button>} />}<p className="muted">Runalytics {__APP_VERSION__}</p></section>;
}