import assert from "node:assert/strict";
import { test } from "node:test";
import { activateProfileStorage } from "../src/desk/profile-storage.ts";

class MemoryStorage {
  values = new Map();
  failure = null;
  get length() { return this.values.size; }
  key(index) { return [...this.values.keys()][index] ?? null; }
  getItem(key) { return this.values.get(key) ?? null; }
  setItem(key, value) { if (this.failure?.(key, value)) throw new Error("Storage full"); this.values.set(key, value); }
  removeItem(key) { this.values.delete(key); }
}
const session = (profileKey, legacyProfile = false) => ({ profileKey, legacyProfile, legacyAvailable: false });
const personal = storage => Object.fromEntries([...storage.values].filter(([key]) => key.startsWith("runalytics.") && !key.startsWith("runalytics.profile.") && key !== "runalytics.nav.v1"));

test("logout and account B never inherit account A's preferences or drafts; A restores them", () => {
  const storage = new MemoryStorage();
  activateProfileStorage(storage, session("account-a"));
  storage.setItem("runalytics.runnerName", "Ada");
  storage.setItem("runalytics.onboarding.v1", JSON.stringify({ name: "Ada", step: "pace", threshold: "4:40" }));
  storage.setItem("runalytics.language", "it");
  storage.setItem("runalytics.earned-achievements", '["first-run"]');
  const saved = personal(storage);
  activateProfileStorage(storage, session(null));
  assert.deepEqual(personal(storage), {});
  activateProfileStorage(storage, session("account-b"));
  assert.deepEqual(personal(storage), {});
  storage.setItem("runalytics.runnerName", "Bea");
  activateProfileStorage(storage, session("account-a"));
  assert.deepEqual(personal(storage), saved);
  activateProfileStorage(storage, session("account-b"));
  assert.equal(storage.getItem("runalytics.runnerName"), "Bea");
});

test("unowned legacy data is quarantined until its owner explicitly adopts it", () => {
  const storage = new MemoryStorage();
  storage.setItem("runalytics.runnerName", "Previous runner");
  storage.setItem("runalytics.onboarding.complete", "1");
  activateProfileStorage(storage, session(null));
  activateProfileStorage(storage, session("account-b"));
  assert.deepEqual(personal(storage), {});
  activateProfileStorage(storage, session("account-b", true));
  assert.equal(storage.getItem("runalytics.runnerName"), "Previous runner");
  activateProfileStorage(storage, session("account-c", true));
  assert.deepEqual(personal(storage), {});
});

test("a verified legacy owner adopts pre-existing preferences and reloads do not overwrite edits", () => {
  const storage = new MemoryStorage();
  storage.setItem("runalytics.runnerName", "Ada");
  activateProfileStorage(storage, session("account-a", true));
  storage.setItem("runalytics.runnerName", "Ada updated");
  activateProfileStorage(storage, session("account-a", true));
  assert.equal(storage.getItem("runalytics.runnerName"), "Ada updated");
  activateProfileStorage(storage, session("account-b"));
  assert.equal(storage.getItem("runalytics.runnerName"), null);
});

test("an interrupted storage rewrite recovers without mixing or losing either profile", () => {
  const storage = new MemoryStorage();
  activateProfileStorage(storage, session("account-b"));
  storage.setItem("runalytics.runnerName", "Bea");
  activateProfileStorage(storage, session("account-a"));
  storage.setItem("runalytics.runnerName", "Ada");
  storage.setItem("runalytics.onboarding.v1", "saved draft");
  storage.failure = (key, value) => key === "runalytics.runnerName" && value === "Bea";
  assert.throws(() => activateProfileStorage(storage, session("account-b")), /Storage full/);
  storage.failure = null;
  activateProfileStorage(storage, session("account-b"));
  assert.equal(storage.getItem("runalytics.runnerName"), "Bea");
  assert.equal(storage.getItem("runalytics.onboarding.v1"), null);
  activateProfileStorage(storage, session("account-a"));
  assert.equal(storage.getItem("runalytics.runnerName"), "Ada");
  assert.equal(storage.getItem("runalytics.onboarding.v1"), "saved draft");
});

test("session drafts are scoped while unrelated storage and navigation metadata survive", () => {
  const storage = new MemoryStorage();
  storage.setItem("another-app", "untouched");
  storage.setItem("runalytics.nav.v1", "1");
  activateProfileStorage(storage, session("account-a"));
  storage.setItem("runalytics.setup-transfer", "encrypted-qr-draft");
  activateProfileStorage(storage, session("account-b"));
  assert.equal(storage.getItem("runalytics.setup-transfer"), null);
  activateProfileStorage(storage, session("account-a"));
  assert.equal(storage.getItem("runalytics.setup-transfer"), "encrypted-qr-draft");
  assert.equal(storage.getItem("another-app"), "untouched");
  assert.equal(storage.getItem("runalytics.nav.v1"), "1");
});

test("a corrupt destination is rejected before changing the currently open profile", () => {
  const storage = new MemoryStorage();
  activateProfileStorage(storage, session("account-a"));
  storage.setItem("runalytics.runnerName", "Ada");
  storage.setItem("runalytics.profile.data.account-b", "{broken");
  assert.throws(() => activateProfileStorage(storage, session("account-b")));
  assert.equal(storage.getItem("runalytics.runnerName"), "Ada");
});

test("late IPC replies cannot update another profile, while account-change commands still resolve", async t => {
  const previous = { window: globalThis.window, sessionStorage: globalThis.sessionStorage };
  t.after(() => { for (const [key, value] of Object.entries(previous)) { if (value === undefined) delete globalThis[key]; else globalThis[key] = value; } });
  globalThis.sessionStorage = new MemoryStorage();
  let release;
  const calls = [];
  globalThis.window = { __TAURI_INTERNALS__: { invoke: async (command, args) => {
    calls.push({ command, args });
    if (command === "get_settings") return new Promise(resolve => { release = resolve; });
    return null;
  } } };
  const { tauri, setProfileScope } = await import("../src/lib/ipc.ts");
  setProfileScope("account-a");
  const stale = tauri.getSettings();
  setProfileScope("account-b");
  release({ theme: "dark" });
  await assert.rejects(stale, /COROS profile changed/);
  await tauri.saveSettings({ theme: "light" });
  assert.deepEqual(calls, [
    { command: "get_settings", args: { profileScope: "account-a" } },
    { command: "save_settings", args: { settings: { theme: "light" }, profileScope: "account-b" } },
  ]);
  globalThis.window.__TAURI_INTERNALS__.invoke = async () => { setProfileScope("account-c"); return "connected"; };
  assert.equal(await tauri.connectProvider("coros"), "connected");
});
