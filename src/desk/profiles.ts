import { listen } from "@tauri-apps/api/event";
import { inTauri, setProfileScope, tauri } from "../lib/ipc";
import type { AccountSession } from "../lib/types";
import { activateProfileStorage } from "./profile-storage";

let session: AccountSession = { profileKey: null, legacyAvailable: false, legacyProfile: false };
let pending: Promise<void> | null = null;
export const accountSession = () => session;

function activate(next: AccountSession) {
  activateProfileStorage(localStorage, next);
  activateProfileStorage(sessionStorage, next);
  setProfileScope(next.profileKey);
  session = next;
}

export async function initializeAccount() {
  const next = inTauri ? await tauri.accountSession() : session;
  activate(next);
  if (inTauri) await listen("account://changed", () => { void refreshAccount().catch(() => undefined); }).catch(() => undefined);
  return next;
}

export function refreshAccount(force = false): Promise<void> {
  if (pending) return pending;
  pending = (async () => {
    const next = await tauri.accountSession();
    if (!force && next.profileKey === session.profileKey && next.legacyProfile === session.legacyProfile) { session = next; return; }
    window.dispatchEvent(new Event("runalytics:account-changing"));
    activate(next);
    history.replaceState(null, "", `${location.pathname}${location.search}#training`);
    location.reload();
  })().catch(error => {
    window.dispatchEvent(new CustomEvent("runalytics:account-error", { detail: String(error) }));
    throw error;
  }).finally(() => { pending = null; });
  return pending;
}
