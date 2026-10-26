// Nav-bar sync control + startup auto-sync wiring.
//
// The Rust side auto-syncs when the app opens and emits `sync://started` /
// `sync://completed` around every sync (auto or manual). Because the pages
// are separate HTML documents (plain links, full reloads), each page:
//   1. mounts the chip and asks `sync_status` whether a sync is already
//      in flight (covers navigating mid-sync), and
//   2. listens for the events to start/stop the spinner.
// On completion a `runalytics:sync-completed` window event is dispatched so
// the current page can refresh its data.
//
// The chip doubles as the one-tap Sync button: syncing is a first-class
// action on every page, not a detour through Settings. While idle it reads
// "Sync"; while syncing it spins and is inert.

import { listen } from "@tauri-apps/api/event";
import { fail, inTauri, tauri } from "./ipc";
import { icons } from "./icons";

const SPINNER = `
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor"
    stroke-width="2.4" stroke-linecap="round" aria-hidden="true">
    <path d="M12 3a9 9 0 1 0 9 9" opacity="0.9"/>
  </svg>`;

function mount(): HTMLButtonElement | null {
  const nav = document.getElementById("nav");
  if (!nav) return null;
  let chip = document.getElementById("sync-chip") as HTMLButtonElement | null;
  if (!chip) {
    chip = document.createElement("button");
    chip.id = "sync-chip";
    chip.type = "button";
    chip.setAttribute("aria-live", "polite");
    chip.title = "Sync now";
    chip.hidden = true;
    // On the desktop rail the chip lives in the nav footer, directly above
    // the pinned Settings row (navigation.ts builds the footer). Elsewhere
    // it falls back to before the spacer / the end of the nav, which on the
    // mobile toolbar puts it next to the menu button.
    const footer = nav.querySelector("#nav-footer");
    if (footer) footer.insertBefore(chip, footer.firstChild);
    else {
      const spacer = nav.querySelector(".nav-spacer");
      if (spacer) nav.insertBefore(chip, spacer);
      else nav.appendChild(chip);
    }
  }
  return chip;
}

function paint(chip: HTMLButtonElement, syncing: boolean): void {
  chip.classList.toggle("syncing", syncing);
  chip.classList.toggle("btn-busy", syncing);
  if (syncing) {
    chip.innerHTML = `${SPINNER}<span class="sync-chip-label">Syncing</span>`;
    chip.disabled = true;
    chip.setAttribute("aria-label", "Syncing your data");
    chip.setAttribute("aria-busy", "true");
  } else {
    chip.innerHTML = `${icons.sync}<span class="sync-chip-label">Sync</span>`;
    chip.disabled = false;
    chip.removeAttribute("aria-label");
    chip.removeAttribute("aria-busy");
  }
  chip.hidden = false;
}

/// Call once per page. No-op outside Tauri (plain browser has no backend).
/// The Rust events drive the chip; `sync_status` covers a sync that was
/// already running when this page mounted.
export function initSyncIndicator(): void {
  if (!inTauri) return;
  const chip = mount();
  if (!chip) return;

  const setSyncing = (v: boolean) => paint(chip, v);
  let changedSinceStatusRequest = false;

  chip.addEventListener("click", async () => {
    changedSinceStatusRequest = true;
    setSyncing(true);
    try {
      await tauri.syncNow("coros");
      // The sync://completed event paints the chip and refreshes the page.
    } catch (e) {
      fail(e);
      setSyncing(false);
    }
  });

  // Catch a sync already running when this page mounts; otherwise the chip
  // stays visible as the idle Sync button.
  void tauri
    .syncStatus()
    .then((status) => { if (!changedSinceStatusRequest) setSyncing(status); })
    .catch(() => { if (!changedSinceStatusRequest) setSyncing(false); });

  void listen("sync://started", () => {
    changedSinceStatusRequest = true;
    setSyncing(true);
  }).catch(() => undefined);
  void listen<{ ok: boolean }>("sync://completed", () => {
    changedSinceStatusRequest = true;
    setSyncing(false);
    window.dispatchEvent(new CustomEvent("runalytics:sync-completed"));
  }).catch(() => undefined);
}
