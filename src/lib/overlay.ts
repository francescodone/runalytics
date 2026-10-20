/* Shared overlay: one native <dialog> lifecycle, presented as a sheet.

   Why a native <dialog>: showModal() already gives us the hard parts — a real
   focus trap, Escape-to-close, a ::backdrop scrim, the rest of the page made
   inert, and focus restored to the opener on close. Re-implementing those (as
   the old activity sheet did with manual `inert` + a hand-rolled Tab trap) is
   where modality bugs come from. So modality is native; this module adds only
   the two things the platform doesn't give us:

   1. A slide/drag presentation. The panel enters along the edge it leaves
      from, and can be grabbed mid-flight and thrown shut — the seam between
      dragging and animating is invisible because the spring inherits the
      pointer's release velocity (see motion.ts).
   2. A loading shell. open() shows the surface immediately; the caller fills
      it asynchronously and a stale response (the sheet closed or reopened
      meanwhile) can never repaint the wrong content — every load is stamped
      with a generation token. */

import {
  animateSpring,
  prefersReducedMotion,
  projectEndpoint,
  setAxis,
  readAxis,
  animatePresent,
  animateDismiss,
} from "./motion";

export interface Sheet {
  readonly root: HTMLDialogElement;
  readonly panel: HTMLElement;
  readonly body: HTMLElement;
  readonly title: HTMLElement;
  readonly subtitle: HTMLElement;
  setTitle(title: string, subtitle?: string): void;
  /** True while this sheet is the one on screen. */
  isOpen(): boolean;
  /**
   * Claim a load. Returns a token; pass it to `isCurrent` before touching the
   * DOM, so a response that lands after a close/reopen is dropped.
   */
  beginLoad(): number;
  isCurrent(token: number): boolean;
  open(): void;
  close(): void;
  destroy(): void;
}

interface SheetOptions {
  /** Accessible name for the dialog. */
  label: string;
  /** Called when the sheet has fully closed (map teardown, etc.). */
  onClosed?: () => void;
}

const AXIS = "(max-width: 760px)"; // below this, the sheet is a bottom sheet

export function createSheet(options: SheetOptions): Sheet {
  const root = document.createElement("dialog");
  root.className = "sheet-dialog";
  root.setAttribute("aria-modal", "true");
  root.setAttribute("aria-labelledby", "sheet-dialog-title");
  root.innerHTML = `
    <div class="sheet-panel" role="document">
      <header class="sheet-grab">
        <div class="sheet-grab-copy">
          <h2 id="sheet-dialog-title" class="sheet-title"></h2>
          <p class="sheet-sub section-intro"></p>
        </div>
        <button class="sheet-close" type="button" data-close aria-label="Close">
          <svg viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18"/></svg>
        </button>
      </header>
      <div class="sheet-body"></div>
    </div>`;

  const panel = root.querySelector<HTMLElement>(".sheet-panel")!;
  const grab = root.querySelector<HTMLElement>(".sheet-grab")!;
  const body = root.querySelector<HTMLElement>(".sheet-body")!;
  const title = root.querySelector<HTMLElement>(".sheet-title")!;
  const subtitle = root.querySelector<HTMLElement>(".sheet-sub")!;

  let shown = false;
  let dismissing = false;
  let loadToken = 0;
  let opener: HTMLElement | null = null;

  const bottom = () => window.matchMedia(AXIS).matches;
  const axis = () => (bottom() ? "y" : "x") as "x" | "y";
  // How far the panel travels to leave: its own extent along the drag axis.
  const dismissDistance = () =>
    bottom() ? panel.offsetHeight : panel.offsetWidth;

  function setTitle(t: string, sub = ""): void {
    title.textContent = t;
    subtitle.textContent = sub;
    subtitle.hidden = sub === "";
  }

  function park(): void {
    // Put the panel off its ENTRY edge before the first paint: a side sheet
    // waits off the right, a bottom sheet waits below. Both then spring to 0.
    setAxis(panel, axis(), dismissDistance());
  }

  function settleIn(): void {
    root.classList.add("is-open");
    void animateSpring(panel, axis(), 0, {
      from: readAxis(panel, axis()),
      duration: 0.34,
      bounce: 0,
    });
  }

  function open(): void {
    if (shown) return;
    shown = true;
    dismissing = false;
    opener = document.activeElement as HTMLElement | null;
    park();
    document.body.appendChild(root);
    root.showModal();
    settleIn();
  }

  function finishClose(): void {
    if (!shown) return;
    shown = false;
    root.classList.remove("is-open");
    root.close();
    root.remove();
    // Native dialog restores focus to the opener; if it was removed from the
    // DOM while the sheet was up, fall back to the main content.
    const target =
      opener && document.contains(opener)
        ? opener
        : (document.querySelector<HTMLElement>("#app") ?? document.body);
    target.focus();
    opener = null;
    options.onClosed?.();
  }

  function close(): void {
    if (!shown || dismissing) return;
    dismissing = true;
    // Both surfaces dismiss in the +axis direction: a side sheet off the
    // right, a bottom sheet off the bottom. Same sign as the entry, so the
    // exit retraces the entry path.
    const to = dismissDistance();
    if (prefersReducedMotion()) {
      setAxis(panel, axis(), to);
      finishClose();
      return;
    }
    void animateSpring(panel, axis(), to, {
      from: readAxis(panel, axis()),
      duration: 0.28,
      bounce: 0,
    }).then(finishClose);
  }

  // --- drag to dismiss -----------------------------------------------------
  // Only the header/handle is draggable, so charts, the map, the body map and
  // scrolling content keep their own gestures. A re-grab mid-spring reads the
  // live transform and inherits velocity, so reversing a dismissal is smooth.
  let dragging = false;
  let startPointer = 0;
  let startPos = 0;
  let lastTime = 0;
  let lastPos = 0;
  let velocity = 0;
  let activeId = -1;

  grab.addEventListener("pointerdown", (e) => {
    if ((e.target as HTMLElement).closest("[data-close]")) return;
    dragging = true;
    activeId = e.pointerId;
    grab.setPointerCapture(e.pointerId);
    const a = axis();
    startPointer = a === "x" ? e.clientX : e.clientY;
    startPos = readAxis(panel, a);
    lastPos = startPointer;
    lastTime = e.timeStamp;
    velocity = 0;
    panel.classList.add("is-dragging");
  });

  grab.addEventListener("pointermove", (e) => {
    if (!dragging || e.pointerId !== activeId) return;
    const a = axis();
    const p = a === "x" ? e.clientX : e.clientY;
    let delta = p - startPointer;
    // Rubber-band past the open boundary: dragging back INTO the content
    // (negative on either axis) resists instead of following 1:1.
    if (delta < 0) delta *= 0.35;
    const pos = startPos + delta;
    setAxis(panel, a, pos);
    const dt = e.timeStamp - lastTime;
    if (dt > 0) velocity = (p - lastPos) / (dt / 1000); // px/s
    lastPos = p;
    lastTime = e.timeStamp;
  });

  function endDrag(e: PointerEvent): void {
    if (!dragging || e.pointerId !== activeId) return;
    dragging = false;
    panel.classList.remove("is-dragging");
    const a = axis();
    const current = readAxis(panel, a);
    const max = dismissDistance();
    // Project where the throw is going, clamp into [open, dismissed], and pick
    // the nearer snap. Decide from the projected point, not the release point.
    const projected = projectEndpoint(current, velocity);
    const clamped = Math.min(max, Math.max(0, projected));
    const commit = Math.abs(clamped - max) < Math.abs(clamped - 0);
    if (commit) {
      dismissing = true;
      void animateSpring(panel, a, max, {
        from: current,
        velocity,
        duration: 0.4,
        bounce: 0.18,
      }).then(finishClose);
    } else {
      void animateSpring(panel, a, 0, {
        from: current,
        velocity,
        duration: 0.36,
        bounce: 0,
      });
    }
  }

  grab.addEventListener("pointerup", endDrag);
  grab.addEventListener("pointercancel", (e) => {
    if (!dragging || e.pointerId !== activeId) return;
    dragging = false;
    panel.classList.remove("is-dragging");
    void animateSpring(panel, axis(), 0, {
      from: readAxis(panel, axis()),
      duration: 0.3,
      bounce: 0,
    });
  });

  // --- native modality hooks ----------------------------------------------
  root.addEventListener("click", (e) => {
    if ((e.target as HTMLElement).closest("[data-close]")) {
      close();
      return;
    }
    // A click that lands on the dialog itself (not the panel) is a backdrop
    // click — the transparent area around the panel.
    if (e.target === root) close();
  });

  // Escape fires `cancel` before `close`; intercept it so the exit animates.
  root.addEventListener("cancel", (e) => {
    e.preventDefault();
    close();
  });

  return {
    root,
    panel,
    body,
    title,
    subtitle,
    setTitle,
    isOpen: () => shown,
    beginLoad: () => ++loadToken,
    isCurrent: (token) => token === loadToken && shown,
    open,
    close,
    destroy: () => {
      if (shown) {
        shown = false;
        root.close();
      }
      root.remove();
    },
  };
}

/* ------------------------------------------------------------------ *
 * Centered modal
 * ------------------------------------------------------------------ *
 * The sheet slides in from an edge and is thrown shut; a modal is a focused
 * question that appears *over* the page and leaves the same way. Both are
 * native <dialog>s — the difference is only presentation and dismissal, so
 * they share the parts that matter (native modality, animated Escape, focus
 * restore) and differ in the parts that don't.
 *
 * Before this, workout-detail and onboarding each re-implemented Escape and
 * backdrop-click by hand, and onboarding had no focus trap at all because it
 * was a plain <div role="dialog">. Those bugs live here once now. */

export interface Modal {
  readonly root: HTMLDialogElement;
  readonly card: HTMLElement;
  isOpen(): boolean;
  open(): void;
  close(): void;
  destroy(): void;
}

interface ModalOptions {
  /// The card element (already populated). The controller appends it to a
  /// fresh <dialog> and owns the dialog's lifecycle.
  card: HTMLElement;
  /** Accessible name for the dialog. */
  label: string;
  /** Extra class on the <dialog> for surface-specific styling. */
  className?: string;
  /** Called after the modal has fully closed and been removed. */
  onClosed?: () => void;
}

export function createModal(options: ModalOptions): Modal {
  const root = document.createElement("dialog");
  root.className = `modal-dialog${options.className ? ` ${options.className}` : ""}`;
  root.setAttribute("aria-modal", "true");
  root.setAttribute("aria-label", options.label);
  root.append(options.card);

  let shown = false;
  let dismissing = false;
  let opener: HTMLElement | null = null;

  function finishClose(): void {
    if (!shown) return;
    shown = false;
    root.close();
    root.remove();
    const target =
      opener && document.contains(opener)
        ? opener
        : (document.querySelector<HTMLElement>("#app") ?? document.body);
    target.focus();
    opener = null;
    options.onClosed?.();
  }

  function close(): void {
    if (!shown || dismissing) return;
    dismissing = true;
    void animateDismiss(options.card).then(() => {
      dismissing = false;
      finishClose();
    });
  }

  function open(): void {
    if (shown) return;
    shown = true;
    dismissing = false;
    opener = document.activeElement as HTMLElement | null;
    document.body.appendChild(root);
    root.showModal();
    void animatePresent(options.card);
  }

  // Escape fires `cancel` first; intercept so the exit animates like the button.
  root.addEventListener("cancel", (e) => {
    e.preventDefault();
    close();
  });

  root.addEventListener("click", (e) => {
    // A click on the dialog element itself (not the card) is a backdrop click.
    if (e.target === root) close();
  });

  return {
    root,
    card: options.card,
    isOpen: () => shown,
    open,
    close,
    destroy: () => {
      if (shown) {
        shown = false;
        root.close();
      }
      root.remove();
    },
  };
}
