import { tauri, el, fail, inTauri, fmtDateTime } from "./ipc";
import type { PlanSync, PlanSyncStatus } from "./types";
import { withBusy } from "./feedback";

/// Plan sync, hosted inside the Calendar subscription pane.
///
/// Two devices never exchange addresses: both derive the same channel code from
/// the COROS account, so this block's job is to say which of the two
/// prerequisites is still missing rather than to collect a URL. It no longer
/// collects a GitHub token either — publishing the calendar to a gist already
/// holds one, and both features authenticate with it. initPlanSync() is a no-op
/// on pages without the block.
let status: PlanSyncStatus | null = null;

function when(iso: string | null): string {
  return iso ? fmtDateTime(iso) : "never";
}

function describe(config: PlanSync): string {
  if (config.lastError) return config.lastError;
  if (!config.enabled) return "Plan sync is off.";
  if (!status?.accountLinked)
    return "Connect COROS first — it is how your two devices recognise each other.";
  if (!config.hasToken)
    return config.canPublish
      ? "Save a GitHub token with gist scope in the publishing section above — it is what publishes the plan to a private gist."
      : "Save the same GitHub token in the publishing section above — it is how this device finds the gist your computer published.";
  if (config.canPublish)
    return `Publishing automatically. Last published ${when(config.lastPushed)}.`;
  return `Last imported ${when(config.lastPulled)}.`;
}

function show(next: PlanSyncStatus): void {
  status = next;
  const config = next.config;
  el<HTMLInputElement>("sync-enabled").checked = config.enabled;
  el("sync-status").textContent = describe(config);
  el("sync-status").classList.toggle("error", !!config.lastError);
  el("sync-code").textContent = next.code
    ? `Channel ${next.code.slice(0, 8)}…${next.code.slice(-4)}`
    : "Channel unavailable until COROS is connected.";
  // The two actions are mutually exclusive by design: one machine writes the
  // plan, the other imports it. Showing both invites the wrong press.
  el<HTMLButtonElement>("sync-push").hidden = !config.canPublish;
  el<HTMLButtonElement>("sync-pull").hidden = config.canPublish;
  el<HTMLButtonElement>("sync-pull").disabled = !config.enabled || !config.hasToken;
  el<HTMLButtonElement>("sync-push").disabled = !config.enabled || !config.hasToken;
}

async function refresh(): Promise<void> {
  try {
    show(await tauri.planSyncStatus());
  } catch (error) {
    if (!inTauri) {
      el("sync-status").textContent = "Plan sync is available in the app.";
      return;
    }
    el("sync-status").textContent = String(error);
  }
}

/// Import the published plan. A different plan already on this device is not
/// destroyed silently — the runner confirms the replacement first.
/// Returns the sentence to show, or null when the caller should stop.
async function pull(force: boolean): Promise<string | null> {
  const outcome = await tauri.planSyncPull(force);
  if (outcome.conflict !== null) {
    if (!confirm("This will replace the plan already on this device with the one from your computer.")) {
      return "Import cancelled — your local plan is unchanged.";
    }
    return pull(true);
  }
  if (!outcome.installed) {
    return "This device already has the newest version of that plan.";
  }
  // The plan changed underneath every view.
  window.dispatchEvent(new CustomEvent("runalytics:calendar-updated"));
  return `Imported ${outcome.workouts} workouts — ${outcome.planName}.`;
}

export function initPlanSync(): void {
  if (!document.getElementById("sync-enabled")) return;
  const enabled = el<HTMLInputElement>("sync-enabled");
  if (!inTauri) enabled.disabled = true;

  // The only setting left in this block is the switch itself — the credential
  // it needs is the shared GitHub token saved above, so there is nothing else
  // to save and no save button to press. Toggling writes immediately, which
  // also means a runner cannot leave the pane with an unsaved switch.
  enabled.addEventListener("change", async () => {
    const wanted = enabled.checked;
    el("sync-status").textContent = "Saving…";
    try {
      show(await tauri.planSyncConfigure({ enabled: wanted }, null));
    } catch (error) {
      enabled.checked = !wanted;
      el("sync-status").textContent = String(error);
      el("sync-status").classList.add("error");
    }
  });

  el("sync-push").addEventListener("click", async () => {
    try {
      await withBusy(el<HTMLButtonElement>("sync-push"), "Publishing…", async () => {
        // The command persists what it did, so the honest read is a fresh one.
        await tauri.planSyncPush();
        return tauri.planSyncStatus();
      }).then(show);
    } catch (error) {
      el("sync-status").textContent = String(error);
      el("sync-status").classList.add("error");
    }
  });

  el("sync-pull").addEventListener("click", async () => {
    try {
      const [message, next] = await withBusy(
        el<HTMLButtonElement>("sync-pull"),
        "Importing…",
        async () => {
          const outcome = await pull(false);
          return [outcome, await tauri.planSyncStatus()] as const;
        },
      );
      // Refresh first so the timestamps are current, then say what happened —
      // in that order, or the refresh would erase the answer. Both go after
      // withBusy, whose restore would undo the disabled state show() sets.
      show(next);
      if (message) el("sync-status").textContent = message;
    } catch (error) {
      el("sync-status").textContent = String(error);
      el("sync-status").classList.add("error");
    }
  });

  // A sync can publish or import a plan from anywhere in the app, which makes
  // the timestamps on this pane stale. `runalytics:credential` is the calendar
  // form above telling us the shared GitHub token changed.
  for (const event of [
    "runalytics:sync-completed",
    "runalytics:calendar-updated",
    "runalytics:credential",
  ]) {
    window.addEventListener(event, () => {
      void tauri.planSyncStatus().then(show).catch(fail);
    });
  }
  void refresh();
}
