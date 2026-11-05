import { useEffect, useRef, useState } from "react";
import QRCode from "qrcode";
import jsQR from "jsqr";
import { IconCamera, IconDownload, IconQrcode, IconUpload } from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { inTauri, tauri } from "../lib/ipc";
import { Button, Dialog, ErrorNotice } from "./ui";
import { Presence } from "./interaction";
import { patchSettings, useCommand, useRemote } from "./data";

async function requireWatchConnection() {
  const providers = await tauri.providerStatus();
  if (!providers.some(provider => provider.provider === "coros" && provider.connected)) throw new Error("Sign in to your watch account before importing or exporting connections.");
}

type Connections = { version: 1; created: number; calendar: { uploadUrl: string; subscriptionUrl: string; username: string; password: string | null }; githubToken: string | null; strava?: {clientId:string; clientSecret:string; athleteId?:number | null} | null; identity?: {corosCode:string | null}; assistant?: {endpoint:string; model:string | null; apiKey:string | null} | null };
type IdentityCheck = { mismatch: boolean; unverified: boolean; details: string[] };
async function checkIdentity(value: Connections): Promise<IdentityCheck> {
  const [coros, strava] = await Promise.all([tauri.planSyncStatus(), tauri.stravaStatus()]);
  const source = value.identity?.corosCode;
  const target = coros.accountLinked ? coros.code : null;
  const sourceAthlete = value.strava?.athleteId;
  const targetAthlete = strava.connected ? strava.athleteId : null;
  const corosMismatch = Boolean(source && target && source !== target);
  const stravaMismatch = Boolean(sourceAthlete && targetAthlete && sourceAthlete !== targetAthlete);
  return {
    mismatch: corosMismatch || stravaMismatch,
    unverified: !source || !target || Boolean(value.strava && (!sourceAthlete || !targetAthlete)),
    details: [source && target ? corosMismatch ? "COROS accounts differ. Sign in to the source account before importing." : "COROS account matches this device." : "COROS account could not be compared. Connect the same account on both devices.", ...(sourceAthlete ? [targetAthlete ? stravaMismatch ? `Strava accounts differ: QR athlete ${sourceAthlete}, this device ${targetAthlete}.` : `Strava athlete ${sourceAthlete} matches this device.` : `QR belongs to Strava athlete ${sourceAthlete}; this device is not connected to Strava.`] : ["No Strava athlete identity is included; Strava account ownership cannot be compared."])],
  };
}
const prefix = "runalytics:connections:1:";
const encode = (bytes: Uint8Array) => btoa(Array.from(bytes, byte => String.fromCharCode(byte)).join(""));
const decode = (value: string) => Uint8Array.from(atob(value), character => character.charCodeAt(0));
async function transferKey(passphrase: string, salt: Uint8Array<ArrayBuffer>) {
  const material = await crypto.subtle.importKey("raw", new TextEncoder().encode(passphrase), "PBKDF2", false, ["deriveKey"]);
  return crypto.subtle.deriveKey({ name: "PBKDF2", salt, iterations: 600000, hash: "SHA-256" }, material, { name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"]);
}
export async function sealConnections(value: Connections, passphrase: string) {
  const salt = crypto.getRandomValues(new Uint8Array(16));
  const nonce = crypto.getRandomValues(new Uint8Array(12));
  const key = await transferKey(passphrase, salt);
  const encrypted = await crypto.subtle.encrypt({ name: "AES-GCM", iv: nonce }, key, new TextEncoder().encode(JSON.stringify(value)));
  return prefix + encode(salt) + "." + encode(nonce) + "." + encode(new Uint8Array(encrypted));
}
export async function openConnections(text: string, passphrase: string): Promise<Connections> {
  if (!text.startsWith(prefix) || text.length > 4500) throw new Error("Not a supported Runalytics connection QR code.");
  const parts = text.slice(prefix.length).split(".");
  if (parts.length !== 3) throw new Error("Invalid transfer code.");
  let value: Connections;
  try {
    const salt = decode(parts[0]);
    const nonce = decode(parts[1]);
    if (salt.length !== 16 || nonce.length !== 12) throw new Error();
    const key = await transferKey(passphrase, salt);
    const raw = await crypto.subtle.decrypt({name:"AES-GCM", iv:nonce}, key, decode(parts[2]));
    value = JSON.parse(new TextDecoder().decode(raw));
  } catch { throw new Error("Could not unlock this QR code. Check the passphrase and image."); }
  if (value?.version !== 1 || !Number.isFinite(value.created) || !value.calendar || typeof value.calendar.username !== "string" || ![value.calendar.password, value.githubToken].every(secret => secret === null || typeof secret === "string")) throw new Error("Invalid connection settings.");
  if (value.strava != null && (typeof value.strava.clientId !== "string" || !/^[0-9]+$/.test(value.strava.clientId) || typeof value.strava.clientSecret !== "string" || !value.strava.clientSecret.trim())) throw new Error("Invalid Strava application credentials.");
  if (value.strava?.athleteId != null && (!Number.isSafeInteger(value.strava.athleteId) || value.strava.athleteId <= 0)) throw new Error("Invalid Strava athlete identity.");
  if (value.identity != null && (typeof value.identity !== "object" || (value.identity.corosCode !== null && typeof value.identity.corosCode !== "string"))) throw new Error("Invalid source account identity.");
  if (value.assistant != null) {
    if (typeof value.assistant.endpoint !== "string" || ![value.assistant.model,value.assistant.apiKey].every(field => field === null || typeof field === "string")) throw new Error("Invalid assistant settings.");
    const endpoint = new URL(value.assistant.endpoint);
    if (endpoint.username || endpoint.password || endpoint.hash || (endpoint.protocol !== "https:" && !(endpoint.protocol === "http:" && ["localhost","127.0.0.1","[::1]"].includes(endpoint.hostname)))) throw new Error("Assistant endpoint must use HTTPS or localhost HTTP without embedded credentials.");
  }
  for (const address of [value.calendar.uploadUrl, value.calendar.subscriptionUrl]) {
    if (typeof address !== "string") throw new Error("Invalid calendar address.");
    if (!address) continue;
    const url = new URL(address);
    if (url.protocol !== "https:" || url.username || url.password || url.hash) throw new Error("Calendar addresses must use HTTPS without embedded credentials.");
  }
  return value;
}

export function ConnectionTransfer() {
  const [mode, setMode] = useState<"export" | "import" | null>(null);
  const providers = useRemote(tauri.providerStatus);
  const watchConnected = !providers.loading && !providers.error && providers.data?.some(provider => provider.provider === "coros" && provider.connected);
  return <section><p className="muted">Move saved connection settings to another Runalytics device using a passphrase-encrypted QR image.</p><p id="transfer-watch-disclaimer">Before importing or exporting connections, sign in to your watch account. Use the same COROS account on both devices.</p><ErrorNotice error={providers.error} /><div className="form-actions"><Button disabled={!watchConnected} aria-describedby="transfer-watch-disclaimer" onClick={() => setMode("export")}><IconQrcode size={18} />Create transfer QR</Button><Button disabled={!watchConnected} aria-describedby="transfer-watch-disclaimer" onClick={() => setMode("import")}><IconUpload size={18} />Import transfer QR</Button>{!watchConnected && <a className="desk-button normal" href="#settings/watch">Connect your watch</a>}</div><Presence>{mode && watchConnected && <TransferDialog key={mode} mode={mode} onClose={() => setMode(null)} />}</Presence></section>;
}

function TransferDialog({mode, onClose}: {mode:"export" | "import"; onClose:() => void}) {
  const command = useCommand();
  const [passphrase, setPassphrase] = useState("");
  const [image, setImage] = useState("");
  const [encryptedCode, setEncryptedCode] = useState("");
  const [code, setCode] = useState("");
  const [preview, setPreview] = useState<Connections | null>(null);
  const [identity, setIdentity] = useState<IdentityCheck | null>(null);
  const [acceptUnverified, setAcceptUnverified] = useState(false);
  const [camera, setCamera] = useState(false);
  const [cameraError, setCameraError] = useState<string | null>(null);
  const video = useRef<HTMLVideoElement>(null);
  const nativeDesktop = inTauri && !/iPhone|iPad|iPod/.test(navigator.userAgent);
  const cameraAvailable = !nativeDesktop && Boolean(navigator.mediaDevices?.getUserMedia) && !cameraError;
  useEffect(() => {
    if (!encryptedCode) return;
    let active = true;
    const render = async () => {
      const styles = getComputedStyle(document.documentElement);
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 1;
      const context = canvas.getContext("2d")!;
      const color = (token: string) => {
        context.clearRect(0, 0, 1, 1);
        context.fillStyle = styles.getPropertyValue(token).trim();
        context.fillRect(0, 0, 1, 1);
        const pixels = context.getImageData(0, 0, 1, 1).data;
        return "#" + Array.from(pixels.slice(0, 3), value => value.toString(16).padStart(2, "0")).join("");
      };
      const png = await QRCode.toDataURL(encryptedCode, {errorCorrectionLevel:"M",width:1000,margin:4,color:{dark:color("--desk-ink"),light:color("--desk-surface")}});
      if (active) setImage(png);
    };
    void render().catch(() => setCameraError("Could not render the QR image. Close and try again."));
    const observer = new MutationObserver(() => void render().catch(() => undefined));
    observer.observe(document.documentElement, {attributes:true, attributeFilter:["data-desk-theme","data-desk-palette","style"]});
    return () => { active = false; observer.disconnect(); };
  }, [encryptedCode]);
  useEffect(() => {
    if (!camera) return;
    let stopped = false;
    let stream: MediaStream | undefined;
    let frame = 0;
    let lastScan = 0;
    const canvas = document.createElement("canvas");
    const scan = (timestamp = 0) => {
      const element = video.current;
      if (stopped || !element) return;
      if (timestamp - lastScan >= 150 && element.readyState >= 2 && element.videoWidth) {
        lastScan = timestamp;
        const scale = Math.min(1, 800 / Math.max(element.videoWidth, element.videoHeight));
        canvas.width = Math.round(element.videoWidth * scale);
        canvas.height = Math.round(element.videoHeight * scale);
        const context = canvas.getContext("2d", {willReadFrequently:true})!;
        context.drawImage(element, 0, 0, canvas.width, canvas.height);
        const pixels = context.getImageData(0, 0, canvas.width, canvas.height);
        const found = jsQR(pixels.data, pixels.width, pixels.height);
        if (found) { setCode(found.data); setPreview(null); setCamera(false); return; }
      }
      frame = requestAnimationFrame(scan);
    };
    if (!navigator.mediaDevices?.getUserMedia) { setCameraError("Camera unavailable. Upload a QR image instead."); setCamera(false); return; }
    void navigator.mediaDevices.getUserMedia({video:{facingMode:"environment"}, audio:false}).then(async source => {
      stream = source;
      if (stopped) { source.getTracks().forEach(track => track.stop()); return; }
      if (video.current) { video.current.srcObject = source; await video.current.play(); scan(); }
    }).catch(() => { setCameraError("Camera unavailable or permission denied. Upload a QR image instead."); setCamera(false); });
    return () => { stopped = true; cancelAnimationFrame(frame); stream?.getTracks().forEach(track => track.stop()); };
  }, [camera]);
  const loadImage = async (file: File) => {
    setCamera(false);
    setPreview(null);
    if (file.size > 10 * 1024 * 1024) throw new Error("Choose an image smaller than 10 MB.");
    const bitmap = await createImageBitmap(file);
    try {
      if (bitmap.width * bitmap.height > 24000000) throw new Error("Choose an image smaller than 24 megapixels.");
      const canvas = document.createElement("canvas");
      const scale = Math.min(1, 2000 / Math.max(bitmap.width, bitmap.height));
      canvas.width = Math.round(bitmap.width * scale); canvas.height = Math.round(bitmap.height * scale);
      const context = canvas.getContext("2d")!;
      context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
      const pixels = context.getImageData(0, 0, canvas.width, canvas.height);
      const found = jsQR(pixels.data, pixels.width, pixels.height);
      if (!found) throw new Error("No QR code found in this image.");
      setCode(found.data);
    } finally { bitmap.close(); }
  };
  return <Dialog title={mode === "export" ? "Create connection transfer" : "Import connection transfer"} onClose={onClose}>
    <div className="connection-transfer-content">
    {!preview && <section className="transfer-summary" aria-label="Transfer contents"><h3>{mode === "export" ? "Transferred when configured" : "Supported connection settings"}</h3><ul><li>Calendar upload and subscription URLs</li><li>Calendar hosting username and password or token</li><li>GitHub token for device sync</li><li>Strava application Client ID and Client Secret</li><li>Injury assistant endpoint, model and API key</li><li>COROS account fingerprint and Strava athlete ID for account comparison</li></ul><p className="muted">Account sign-ins, OAuth access/refresh tokens, training plans and activity history are not included. Assistant use remains disabled after import.</p></section>}
    <p className="transfer-privacy">Keep the QR image and passphrase private. Anyone with both can access these connections.</p>
    <ErrorNotice error={command.error ?? cameraError} />
    {!preview && !image && <form onSubmit={async event => {
      event.preventDefault();
      await command.run(async () => {
        await requireWatchConnection();
        if (mode === "export") {
          const connections = await invoke<Connections>("export_connection_settings");
          const encrypted = await sealConnections(connections, passphrase);
          setEncryptedCode(encrypted); setPassphrase("");
        } else {
          const decoded = await openConnections(code, passphrase);
          setIdentity(await checkIdentity(decoded)); setAcceptUnverified(false);
          setPreview(decoded); setPassphrase("");
        }
      });
    }}>
      <label>Transfer passphrase<input type="password" autoComplete="new-password" required minLength={12} value={passphrase} onChange={event => setPassphrase(event.target.value)} /></label>
      {mode === "import" && <><label>QR image<input type="file" accept="image/png,image/jpeg,image/webp" disabled={command.busy} onChange={event => { const file = event.target.files?.[0]; if (file) { setCode(""); void command.run(() => loadImage(file)); } event.target.value=""; }} /></label>{cameraAvailable && <div className="form-actions"><Button disabled={command.busy} onClick={() => setCamera(!camera)}><IconCamera size={18} />{camera ? "Stop camera" : "Scan with camera"}</Button></div>}{camera && <video ref={video} className="transfer-camera" muted playsInline aria-label="QR camera preview" />}{code && <p role="status">QR code loaded.</p>}</>}
      <div className="form-actions"><Button type="submit" tone="primary" busy={command.busy} disabled={mode === "import" && !code}><IconQrcode size={18} />{mode === "export" ? "Generate encrypted QR" : "Review settings"}</Button></div>
    </form>}
    {image && <div className="transfer-export"><img className="transfer-qr" src={image} alt="Encrypted connection transfer QR code" /><div className="transfer-export-footer"><a className="desk-button normal" download="runalytics-connections.png" href={image}><IconDownload size={18} />Download QR image</a><p className="muted">Enter the same passphrase on the receiving device. Delete the image after importing.</p></div></div>}
    {preview && <><h3>Replace connection settings?</h3><section aria-label="Account comparison"><h3>Source account check</h3>{identity?.details.map(detail => <p key={detail}>{detail}</p>)}{identity?.mismatch && <p role="alert">Import blocked: the connected accounts do not match.</p>}{identity?.unverified && !identity.mismatch && <label className="toggle-setting"><input type="checkbox" checked={acceptUnverified} onChange={event => setAcceptUnverified(event.target.checked)} />I trust this QR source and understand the account could not be fully verified.</label>}</section><dl className="definition-list"><div><dt>Calendar upload</dt><dd className="file-path">{preview.calendar.uploadUrl || "Not configured; existing value will be cleared"}</dd></div><div><dt>Subscription</dt><dd className="file-path">{preview.calendar.subscriptionUrl || "Not configured; existing value will be cleared"}</dd></div><div><dt>Calendar username</dt><dd>{preview.calendar.username || "None; existing value will be cleared"}</dd></div><div><dt>Calendar credential</dt><dd>{preview.calendar.password ? "Included (hidden); replaces saved credential" : "None; saved credential will be removed"}</dd></div><div><dt>Device sync token</dt><dd>{preview.githubToken ? "Included (hidden); replaces saved token" : "None; saved token will be removed"}</dd></div><div><dt>Strava Client ID</dt><dd>{preview.strava?.clientId ?? "Not included; existing Strava setup stays"}</dd></div>{preview.strava && <div><dt>Strava Client Secret</dt><dd>Included (hidden); replaces saved application credentials</dd></div>}<div><dt>Assistant endpoint</dt><dd className="file-path">{preview.assistant?.endpoint ?? "Not included; existing assistant stays"}</dd></div>{preview.assistant && <><div><dt>Assistant model</dt><dd>{preview.assistant.model || "Provider default"}</dd></div><div><dt>Assistant API key</dt><dd>{preview.assistant.apiKey ? "Included (hidden); replaces saved key" : "None; saved key will be removed"}</dd></div></>}</dl><p>Calendar publishing will remain paused. {preview.githubToken ? "Device sync will be enabled with the imported token." : "Device sync will remain paused without a token."} {preview.strava ? "Strava application credentials will be replaced; reconnect Strava and enable description updates separately." : "Strava settings will not change."} Plans are not included in the QR; existing plan conflict protections still apply.</p><div className="form-actions"><Button onClick={() => { setPreview(null); setCode(""); }}>Choose another QR</Button><Button tone="primary" busy={command.busy} disabled={!identity || identity.mismatch || (identity.unverified && !acceptUnverified)} onClick={() => void command.run(async () => {
      await requireWatchConnection();
      const latestIdentity = await checkIdentity(preview);
      setIdentity(latestIdentity);
      if (latestIdentity.mismatch || (latestIdentity.unverified && !acceptUnverified)) throw new Error("Accounts changed or could not be verified. Review the account comparison before importing.");
      await tauri.planSyncConfigure({enabled:false}, null);
      const current = await tauri.calendarPublishing();
      await tauri.saveCalendarPublishing({...current,enabled:false,uploadUrl:preview.calendar.uploadUrl,subscriptionUrl:preview.calendar.subscriptionUrl,username:preview.calendar.username,lastPublished:null,lastError:null}, preview.calendar.password ?? "");
      await tauri.planSyncConfigure({enabled:false}, preview.githubToken ?? "");
      if (preview.strava) await tauri.stravaConfigure(preview.strava.clientId, preview.strava.clientSecret);
      if (preview.assistant) {
        if (preview.assistant.apiKey) await tauri.setLlmApiKey(preview.assistant.apiKey); else await tauri.clearLlmApiKey();
        await patchSettings({llmEndpoint:preview.assistant.endpoint,llmModel:preview.assistant.model,llmEnabled:false});
      }
      if (preview.githubToken) {
        await tauri.planSyncConfigure({enabled:true}, null);
        window.dispatchEvent(new Event("runalytics:plan-sync-enabled"));
      }
      setPreview(null); setCode(""); onClose();
    }, preview.githubToken ? "Connections imported. Device sync is enabled; calendar publishing remains paused." : "Connections imported. Device sync and calendar publishing remain paused.")}>Replace connections</Button></div></>}
    </div>
  </Dialog>;
}