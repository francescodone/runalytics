import { tauri } from "./ipc";

const GATED_HREFS = ["plan.html", "calendar.html", "injury.html", "achievements.html"];

export interface NavGate {
  /** True once connect + sync + a first plan are all done. */
  unlocked: boolean;
}

function link(href: string): HTMLAnchorElement | null {
  return document.querySelector<HTMLAnchorElement>(`#nav a.nav-link[href="${href}"]`);
}

export function applyNavGate(gate: NavGate, _onPlanPage = false): void {
  const nav = document.getElementById("nav");
  if (!nav) return;
  for (const href of GATED_HREFS) {
    const anchor = link(href);
    if (!anchor) continue;
    anchor.hidden = false;
  }
  nav.dataset.planReady = String(gate.unlocked);
}

export async function initNavGate(onPlanPage = false): Promise<NavGate> {
  let gate: NavGate = { unlocked: true };
  try {
    const [statuses, current] = await Promise.all([
      tauri.providerStatus(),
      tauri.currentPlan().catch(() => null),
    ]);
    const coros = statuses.find((s) => s.provider === "coros");
    gate = {
      unlocked: Boolean(
        coros?.connected && coros.lastSync != null && current?.[0] != null,
      ),
    };
  } catch {
    /* keep the full nav */
  }
  applyNavGate(gate, onPlanPage);
  return gate;
}

export async function requirePlanOrRedirect(self: "plan" | "gated"): Promise<boolean> {
  await initNavGate(self === "plan");
  return false;
}
