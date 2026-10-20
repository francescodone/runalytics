/* Shared async-action feedback.
   Every button that round-trips to the backend should look the same while it
   works: disabled so it can't double-fire, a spinner, a live label, and a
   guaranteed restore when the call settles — even if the caller throws.

   The plan page already had this shape (`withBusy`); it lived there because
   only the plan needed the `plan://sync` stage feed. The PRESENTATION is
   generic, so it lives here; the caller passes a `progress` callback for
   operation-specific stages and this module owns the DOM and cleanup. */

const SPINNER = `<svg class="btn-spinner" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" aria-hidden="true"><path d="M12 3a9 9 0 1 0 9 9" opacity="0.9"/></svg>`;

/// Run `action` with the button showing a busy state. The label starts at
/// `base`; a `progress` callback can replace it (e.g. backend stages). The
/// button's original label, disabled state and aria-busy are always restored.
export async function withBusy<T>(
  button: HTMLButtonElement,
  base: string,
  action: (report: (stage: string, percent?: number | null) => void) => Promise<T>,
): Promise<T> {
  const original = button.textContent ?? base;
  const hadDisabled = button.disabled;
  let stage = base;
  let percent: number | null = null;

  const paint = () => {
    const pct = percent != null ? ` ${Math.round(percent)}%` : "";
    button.innerHTML = `${SPINNER}<span>${escapeText(stage + pct)}</span>`;
  };

  button.disabled = true;
  button.classList.add("btn-busy");
  button.setAttribute("aria-busy", "true");
  paint();

  const report = (nextStage: string, nextPercent: number | null = null) => {
    stage = nextStage || base;
    percent = nextPercent;
    paint();
  };

  try {
    return await action(report);
  } finally {
    button.classList.remove("btn-busy");
    button.removeAttribute("aria-busy");
    button.disabled = hadDisabled;
    button.textContent = original;
  }
}

/// A brief "Saved" confirmation on a save button, reverting to `label` after a
/// beat. Unlike a fixed timer that lies about persistence, callers await the
/// real write first and only then call this.
export function confirmSaved(button: HTMLButtonElement, label = "Save"): void {
  button.textContent = "Saved";
  button.classList.add("btn-saved");
  window.setTimeout(() => {
    button.textContent = label;
    button.classList.remove("btn-saved");
  }, 1500);
}

function escapeText(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
