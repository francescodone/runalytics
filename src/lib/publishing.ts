import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { tauri, el, fail, inTauri, fmtDateTime } from "./ipc";
import type { CalendarPublishing } from "./types";
import { withBusy } from "./feedback";

/// The Calendar subscription pane, hosted by Settings (it configures the
/// feed, it is not part of the month view). initPublishing() is a no-op on
/// pages without the pane, so the sync-completed listener only lives where
/// the controls are.
///
/// The GitHub token saved here is also what the plan sync section below
/// authenticates with, so a save that added or removed a credential dispatches
/// `runalytics:credential` and that section re-reads its own status.
let publishing: CalendarPublishing | null = null;

function showPublishing(config: CalendarPublishing): void {
  publishing = config;
  el<HTMLInputElement>("calendar-enabled").checked = config.enabled;
  el<HTMLInputElement>("calendar-upload-url").value = config.uploadUrl;
  el<HTMLInputElement>("calendar-read-url").value = config.subscriptionUrl;
  el<HTMLInputElement>("calendar-username").value = config.username;
  el<HTMLInputElement>("calendar-password").placeholder = config.hasPassword ? "Saved in Keychain" : "Password or bearer token";
  el<HTMLInputElement>("calendar-subscription-url").value = config.subscriptionUrl;
  const ready = !!config.lastPublished && !!config.subscriptionUrl;
  el<HTMLButtonElement>("calendar-copy").disabled = !ready;
  el<HTMLButtonElement>("calendar-subscribe").disabled = !ready;
  el("publish-status").textContent = config.lastError
    ? `${config.lastError}. Local calendar is up to date.`
    : !config.enabled
      ? "Remote publishing is off."
      : config.lastPublished
        ? `Published ${fmtDateTime(config.lastPublished)}`
        : "Not yet published.";
  el("publish-status").classList.toggle("error", !!config.lastError);
}

/// Reading the pane's state must never publish: `ics_path` is a read-only
/// path getter and `calendar_publishing` only loads the config. Publishing
/// happens on sync, on calendar edits, and through the Refresh button below.
async function refreshPublishing(publish: boolean): Promise<void> {
  const button = el<HTMLButtonElement>("ics-regenerate");
  try {
    await withBusy(button, publish ? "Refreshing calendar…" : "Loading calendar…", async () => {
      if (publish) await tauri.publishCalendar();
      el("ics-path").textContent = await tauri.icsPath();
      showPublishing(await tauri.calendarPublishing());
    });
  } catch (e) {
    if (!inTauri) {
      el("ics-path").textContent = "Available in the desktop app.";
      el("publish-status").textContent = "Publishing is available in the desktop app.";
    }
    else el("publish-status").textContent = String(e);
  } finally {
    if (!inTauri) button.disabled = true;
  }
}

export function initPublishing(): void {
  if (!document.getElementById("calendar-publishing-form")) return;
  if (!inTauri) {
    el<HTMLButtonElement>("calendar-save").disabled = true;
    el<HTMLButtonElement>("ics-reveal").disabled = true;
  }
  el("calendar-publishing-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    const button = el<HTMLButtonElement>("calendar-save");
    if (button.disabled) return;
    el("publish-status").textContent = "Saving calendar settings...";
    const password = el<HTMLInputElement>("calendar-password");
    const clear = el<HTMLInputElement>("calendar-clear-password");
    try {
      await withBusy(button, "Saving settings…", async () => {
        const config: CalendarPublishing = {
          enabled: el<HTMLInputElement>("calendar-enabled").checked,
          uploadUrl: el<HTMLInputElement>("calendar-upload-url").value.trim(),
          subscriptionUrl: el<HTMLInputElement>("calendar-read-url").value.trim(),
          username: el<HTMLInputElement>("calendar-username").value.trim(),
          hasPassword: false, lastPublished: null, lastError: null,
        };
        const secret = clear.checked ? "" : password.value || null;
        const next = await tauri.saveCalendarPublishing(config, secret);
        showPublishing(next);
        password.value = "";
        clear.checked = false;
        // The credential this form holds is the one plan sync authenticates
        // with, so its status line is stale the moment this save lands.
        window.dispatchEvent(new CustomEvent("runalytics:credential"));
      });
    } catch (error) { el("publish-status").textContent = String(error); }
  });
  el("calendar-copy").addEventListener("click", async () => {
    const url = publishing?.subscriptionUrl;
    if (!url) return;
    try {
      await withBusy(el<HTMLButtonElement>("calendar-copy"), "Copying…", () => navigator.clipboard.writeText(url));
      el("publish-status").textContent = "Subscription URL copied.";
    } catch { el("publish-status").textContent = "Clipboard unavailable. Select the subscription URL to copy it."; }
  });
  el("calendar-subscribe").addEventListener("click", async () => {
    const url = publishing?.subscriptionUrl;
    if (!url) return;
    try {
      await withBusy(el<HTMLButtonElement>("calendar-subscribe"), "Opening…", () => openUrl(url.replace(/^https:/, "webcal:")));
    } catch (e) { fail(e); }
  });
  el("ics-regenerate").addEventListener("click", () => void refreshPublishing(true));
  el("ics-reveal").addEventListener("click", async () => {
    const path = el("ics-path").textContent;
    if (!path) return;
    if (!inTauri) {
      alert("Reveal is only available in the desktop app.");
      return;
    }
    try {
      await withBusy(el<HTMLButtonElement>("ics-reveal"), "Opening Finder…", async () => {
        try {
          await revealItemInDir(path);
        } catch {
          // Fall back to showing the folder in a browser-style listing is not
          // possible; open the parent directory path as a last resort.
          await openUrl(`file://${path.split("/").slice(0, -1).join("/")}`);
        }
      });
    } catch (e) { fail(e); }
  });
  // A sync republishes the feed, so the published timestamp on the pane goes
  // stale the moment one lands anywhere in the app.
  for (const event of ["runalytics:sync-completed", "runalytics:calendar-updated"]) {
    window.addEventListener(event, () => {
      void tauri.calendarPublishing().then(showPublishing).catch(fail);
    });
  }
  void refreshPublishing(false);
}
