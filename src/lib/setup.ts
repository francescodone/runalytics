// The Setup pane in Settings: connect → sync → plan, one step at a time.
//
// It used to be an overlay on the dashboard, which meant the first thing a new
// user met was a dashboard with nothing on it, covered by a form. Setup is not
// dashboard content — it is configuration with an end — so it lives in Settings
// now, and the dashboard shows whatever data exists instead.
//
// Step 3 deliberately does NOT contain a plan form. Creating a plan happens on
// the Plan page and nowhere else; two forms would be two definitions of a plan,
// and they drift. This step says what a plan is for and hands over.

import { tauri, fail } from "./ipc";
import { icons } from "./icons";
import { setBusy, withBusy } from "./feedback";
import { fetchFlowState, type FlowState } from "./flow";

export function setupComplete(state: FlowState): boolean {
  return state.connected && state.lastSync != null && state.plan != null;
}

interface Step {
  label: string;
  done: boolean;
}

/// The three steps, in the order that makes them possible.
export function setupSteps(state: FlowState): Step[] {
  return [
    { label: "Connect your watch", done: state.connected },
    { label: "Sync your history", done: state.lastSync != null },
    { label: "Create your first plan", done: state.plan != null },
  ];
}

/// Renders the step list and the one step to act on into the Setup pane.
/// `onAdvance` runs whenever a step completes, so the caller can re-read the
/// nav gate — finishing setup is what makes the other tabs appear.
export function renderSetup(
  state: FlowState,
  stepsHost: HTMLElement,
  panel: HTMLElement,
  onAdvance: () => void,
): void {
  const steps = setupSteps(state);
  let syncSummary = "";

  const paintSteps = (current: number) => {
    stepsHost.innerHTML = steps
      .map(
        (s, i) =>
          `<li class="setup-step ${s.done ? "done" : i === current ? "current" : ""}">
          <span class="dot" aria-hidden="true">${s.done ? "✓" : i + 1}</span>${s.label}
        </li>`,
      )
      .join("");
  };

  /// Watch the backend until the sign-in it is running lands, then move on.
  /// Shared by the button and by the resumed pane, because on mobile the page
  /// that pressed the button does not survive the sign-in (see `renderConnect`).
  let pollTimer = 0;
  const startPolling = (btn: HTMLButtonElement, status: HTMLElement) => {
    // Re-rendering this pane would otherwise leave the previous poller running
    // against detached nodes for the rest of its deadline.
    window.clearInterval(pollTimer);
    const busy = setBusy(btn, "Waiting for sign-in…");
    const deadline = Date.now() + 5 * 60_000;
    pollTimer = window.setInterval(async () => {
      try {
        const st = await tauri.providerStatus();
        if (st.find((s) => s.provider === "coros")?.connected) {
          window.clearInterval(pollTimer);
          busy.restore();
          steps[0].done = true;
          onAdvance();
          render();
          return;
        }
        // The sign-in can end in failure after this polling started — on mobile
        // the exchange happens once the user is already back here — so the pane
        // would otherwise sit on "still waiting" for the full five minutes.
        const notice = await tauri.corosSigninNotice();
        if (notice && notice !== "pending") {
          window.clearInterval(pollTimer);
          status.textContent = notice;
          busy.restore();
          btn.textContent = "Try again";
        } else if (Date.now() > deadline) {
          window.clearInterval(pollTimer);
          status.textContent = "Still waiting on the sign-in. Finish it, then try again.";
          busy.restore();
          btn.textContent = "Try again";
        }
      } catch {
        // Transient status poll failure — keep waiting for the deadline.
      }
    }, 2500);
  };

  const renderConnect = () => {
    panel.innerHTML = `
      <h3>Connect your COROS watch</h3>
      <p>Sign in to COROS — Runalytics uses the official API and keeps everything on this device.</p>
      <button type="button" class="big">Connect COROS</button>
      <p class="setup-status" role="status"></p>`;
    const btn = panel.querySelector<HTMLButtonElement>("button")!;
    const status = panel.querySelector<HTMLElement>(".setup-status")!;

    // On mobile the sign-in runs in this webview, so the page that started it is
    // destroyed and this pane loads again afterwards — with the sign-in either
    // still in flight or already over. Ask the backend which, and pick the
    // thread back up, rather than offering a button for something already done.
    void (async () => {
      let notice: string | null = null;
      try {
        notice = await tauri.corosSigninNotice();
      } catch {
        // No backend to ask (plain browser) — the button below says so anyway.
      }
      if (notice === "pending") {
        startPolling(btn, status);
        status.textContent = "Finishing your COROS sign-in…";
        return;
      }
      if (notice) {
        status.textContent = notice;
        btn.textContent = "Try again";
        return;
      }
      // A sign-in that ended in success never leaves a notice, and this page
      // only renders step 1 while the backend still says "not connected", so
      // there is nothing further to recover here.
    })();

    btn.addEventListener("click", async () => {
      const busy = setBusy(btn, "Waiting for sign-in…");
      status.textContent = "Finish the sign-in when it opens.";
      try {
        await tauri.connectProvider("coros");
      } catch (e) {
        status.textContent = "";
        busy.restore();
        fail(e);
        return;
      }
      // Desktop: the browser tab holds the sign-in and this page survives, so
      // the same call resolves once the code has been exchanged. Mobile never
      // gets here — the await dies with the page, which the resumed pane above
      // is the recovery path for.
      // Hand the button over to the poller, which sets its own busy state;
      // restoring first keeps the real label as what gets restored afterwards.
      busy.restore();
      startPolling(btn, status);
    });
  };

  const renderSync = () => {
    panel.innerHTML = `
      <h3>Sync your history</h3>
      <p>One pull brings your runs, sleep, heart rate and training load here. The first sync can take a minute.</p>
      <button type="button" class="big">Sync now</button>
      <p class="setup-status" role="status"></p>`;
    const btn = panel.querySelector<HTMLButtonElement>("button")!;
    const status = panel.querySelector<HTMLElement>(".setup-status")!;
    btn.addEventListener("click", async () => {
      status.textContent = "Syncing…";
      try {
        const r = await withBusy(btn, "Syncing history…", () => tauri.syncNow("coros"));
        syncSummary = `${r.activitiesNew} new run${r.activitiesNew === 1 ? "" : "s"} · ${r.healthDays} days of health data`;
        steps[1].done = true;
        onAdvance();
        render();
      } catch (e) {
        status.textContent = "";
        fail(e);
      }
    });
  };

  const renderPlan = () => {
    panel.innerHTML = `
      <h3>Create your first plan</h3>
      <p>A plan is what the calendar, the injury page and the achievements all describe, which is
      why the rest of the app stays closed until one exists.${
        syncSummary ? ` Your ${syncSummary} will set its starting volume.` : ""
      }</p>
      <p class="setup-hint">
        On the Plan page, pick what it is for — a race to build toward, a distance to improve on,
        or the mileage you already have to hold — and Runalytics writes the weeks.
      </p>
      <a href="plan.html" class="btn big">Create a plan</a>`;
  };

  const renderFinished = () => {
    panel.innerHTML = `
      <div class="setup-finished">
        <div class="setup-trophy medallion lg" aria-hidden="true">${icons.trophy}</div>
        <h3>You're all set</h3>
        <p>Watch connected, data synced and a plan on the calendar. Plan, Calendar, Injuries and
        Achievements are in the sidebar now.</p>
        <a href="index.html" class="btn big">Go to my dashboard</a>
      </div>`;
  };

  const render = () => {
    const current = steps.findIndex((s) => !s.done);
    paintSteps(current);
    if (current === -1) renderFinished();
    else if (current === 0) renderConnect();
    else if (current === 1) renderSync();
    else renderPlan();
  };

  render();
}

/// Read the flow state the Setup pane needs. Returns null outside Tauri or when
/// the backend is unreachable, so the pane can say so rather than show three
/// steps that all claim to be unfinished.
export async function fetchSetupState(): Promise<FlowState | null> {
  try {
    return await fetchFlowState();
  } catch {
    return null;
  }
}
