import type { AccountSession } from "../lib/types";

const metadata = "runalytics.profile.";
const currentKey = `${metadata}current`;
const journalKey = `${metadata}transition`;
const legacyKey = `${metadata}legacy`;
const legacyOwnerKey = `${metadata}legacy-owner`;
const archiveKey = (profile: string) => `${metadata}data.${profile}`;
const scoped = (key: string) => key.startsWith("runalytics.") && !key.startsWith(metadata) && key !== "runalytics.nav.v1";
type Snapshot = Record<string, string>;

function decode(value: string | null): Snapshot {
  if (value === null) return {};
  const parsed: unknown = JSON.parse(value);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed) || Object.entries(parsed).some(([key, value]) => !scoped(key) || typeof value !== "string")) throw new Error("Saved profile preferences could not be read.");
  return parsed as Snapshot;
}

function snapshot(storage: Storage): Snapshot {
  const values: Snapshot = {};
  for (let index = 0; index < storage.length; index++) {
    const key = storage.key(index);
    if (key && scoped(key)) values[key] = storage.getItem(key)!;
  }
  return values;
}

function replace(storage: Storage, values: Snapshot) {
  for (const key of Object.keys(snapshot(storage))) if (!Object.prototype.hasOwnProperty.call(values, key)) storage.removeItem(key);
  for (const [key, value] of Object.entries(values)) storage.setItem(key, value);
}

function recover(storage: Storage) {
  const pending = storage.getItem(journalKey);
  if (!pending) return;
  const journal: unknown = JSON.parse(pending);
  if (!journal || typeof journal !== "object" || !("source" in journal) || !("profile" in journal) || !("legacyOwner" in journal) || (journal.profile !== null && typeof journal.profile !== "string") || (journal.legacyOwner !== null && typeof journal.legacyOwner !== "string")) throw new Error("The profile change could not be recovered.");
  const source = decode(JSON.stringify(journal.source));
  replace(storage, source);
  if (journal.profile === null) storage.removeItem(currentKey); else storage.setItem(currentKey, journal.profile);
  if (journal.legacyOwner === null) storage.removeItem(legacyOwnerKey); else storage.setItem(legacyOwnerKey, journal.legacyOwner);
  storage.removeItem(journalKey);
}

export function activateProfileStorage(storage: Storage, session: AccountSession) {
  recover(storage);
  const previous = storage.getItem(currentKey);
  const next = session.profileKey ?? "signed-out";
  const source = snapshot(storage);
  const legacyOwner = storage.getItem(legacyOwnerKey);
  const adoptLegacy = session.profileKey !== null && session.legacyProfile && legacyOwner === null;
  if (previous === next && !adoptLegacy) return;
  let destination = decode(storage.getItem(archiveKey(next)));
  if (adoptLegacy) destination = storage.getItem(legacyKey) !== null ? decode(storage.getItem(legacyKey)) : previous === null ? source : destination;

  // Save the outgoing data and a recovery journal before changing any live keys.
  if (previous !== null) storage.setItem(archiveKey(previous), JSON.stringify(source));
  else if (!adoptLegacy && Object.keys(source).length) {
    if (storage.getItem(legacyKey) !== null) throw new Error("Unassigned local preferences need to be recovered before switching profiles.");
    storage.setItem(legacyKey, JSON.stringify(source));
  }
  storage.setItem(journalKey, JSON.stringify({ source, profile: previous, legacyOwner }));
  replace(storage, destination);
  storage.setItem(currentKey, next);
  if (adoptLegacy) storage.setItem(legacyOwnerKey, next);
  storage.removeItem(journalKey);
}
