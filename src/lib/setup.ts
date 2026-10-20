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

  const renderConnect = () => {
    panel.innerHTML = `
      <h3>Connect your COROS watch</h3>
      <p>Sign in to COROS in your browser — Runalytics uses the official API and keeps everything on this Mac.</p>
      <button type="button" class="big">Connect COROS</button>
      <p class="setup-status" role="status"></p>`;
    const btn = panel.querySelector<HTMLButtonElement>("button")!;
    const status = panel.querySelector<HTMLElement>(".setup-status")!;
    btn.addEventListener("click", async () => {
      btn.disabled = true;
      status.textContent = "Opening your browser… finish the sign-in there.";
      try {
        await tauri.connectProvider("coros");
      } catch (e) {
        status.textContent = "";
        btn.disabled = false;
        fail(e);
        return;
      }
      // OAuth completes in the external browser — poll until the backend
      // reports the connection, then move to the next step automatically.
      const deadline = Date.now() + 5 * 60_000;
      const timer = window.setInterval(async () => {
        try {
          const st = await tauri.providerStatus();
          if (st.find((s) => s.provider === "coros")?.connected) {
            window.clearInterval(timer);
            steps[0].done = true;
            onAdvance();
            render();
          } else if (Date.now() > deadline) {
            window.clearInterval(timer);
            status.textContent =
              "Still waiting on the sign-in. Finish it in the browser, then try again.";
            btn.disabled = false;
            btn.textContent = "Try again";
          }
        } catch {
          // Transient status poll failure — keep waiting for the deadline.
        }
      }, 2500);
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
      btn.disabled = true;
      status.textContent = "Syncing…";
      try {
        const r = await tauri.syncNow("coros");
        syncSummary = `${r.activitiesNew} new run${r.activitiesNew === 1 ? "" : "s"} · ${r.healthDays} days of health data`;
        steps[1].done = true;
        onAdvance();
        render();
      } catch (e) {
        status.textContent = "";
        btn.disabled = false;
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
