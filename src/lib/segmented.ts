/* Segmented-control pills.
   The track (`.segmented`) paints two pills of its own — a grey one that
   follows the pointer and an accent one that marks the selection — and
   slides them from entry to entry. The entries never fill themselves, so a
   three-option switch still reads as one control travelling along a track.

   All the motion lives in CSS: this module measures the entries and hands
   their geometry over as custom properties (--seg-x/-y/-w/-h for the accent
   pill, --seg-hx/-hy/-hw/-hh for the pointer pill), which lets the browser
   interpolate the travel on the shared spring curve. Nothing here animates. */

type Geometry = { x: number; y: number; w: number; h: number };

// The track is `position: relative`, so it is the entries' offsetParent and
// their offsets are already measured from its padding edge — the same origin
// the pills' absolute box starts from. offset* are layout numbers too, so the
// hover/press transforms the entries carry don't leak into the measurement.
function geometryOf(btn: HTMLElement): Geometry {
  return { x: btn.offsetLeft, y: btn.offsetTop, w: btn.offsetWidth, h: btn.offsetHeight };
}

function write(
  track: HTMLElement,
  prefix: string,
  g: Geometry,
  opacity?: number,
): void {
  const s = track.style;
  s.setProperty(`--${prefix}-x`, `${g.x}px`);
  s.setProperty(`--${prefix}-y`, `${g.y}px`);
  s.setProperty(`--${prefix}-w`, `${g.w}px`);
  s.setProperty(`--${prefix}-h`, `${g.h}px`);
  if (opacity !== undefined) s.setProperty(`--${prefix}-opacity`, `${opacity}`);
}

// The accent pill hides when the control has no selection yet, so the track
// shows bare labels rather than a pill parked on the first entry.
function syncActive(track: HTMLElement): void {
  const active = track.querySelector<HTMLElement>("button.active");
  if (!active || active.offsetWidth === 0) {
    track.style.setProperty("--seg-opacity", "0");
    return;
  }
  write(track, "seg", geometryOf(active), 1);
}

function syncHover(track: HTMLElement, btn: HTMLElement | null): void {
  if (!btn || btn.offsetWidth === 0) {
    track.style.setProperty("--seg-hover-opacity", "0");
    return;
  }
  write(track, "seg-hover", geometryOf(btn), 1);
}

function entryOf(track: HTMLElement, target: EventTarget | null) {
  const el = (target as HTMLElement | null)?.closest?.("button");
  return el && track.contains(el) ? (el as HTMLElement) : null;
}

function observe(track: HTMLElement): void {
  // The selection moves whenever app code flips `.active` — dashboard and
  // settings already do that, so watching the class keeps them untouched.
  new MutationObserver(() => syncActive(track)).observe(track, {
    attributes: true,
    attributeFilter: ["class"],
    subtree: true,
  });

  // Re-measure on any layout change: a window resize, a label that grows,
  // or the track coming back from `hidden` (where every measurement is 0).
  const remeasure = () => {
    if (track.offsetWidth === 0) return;
    syncActive(track);
    syncHover(track, entryOf(track, track.querySelector("button:hover")));
  };
  const ro = new ResizeObserver(remeasure);
  ro.observe(track);
  track.querySelectorAll("button").forEach((b) => ro.observe(b));

  track.addEventListener("pointerover", (e) =>
    syncHover(track, entryOf(track, e.target)),
  );
  // Leaving the track hides the pill — unless an entry still holds keyboard
  // focus, in which case the pill goes back to it rather than vanishing.
  track.addEventListener("pointerleave", () =>
    syncHover(
      track,
      track.contains(document.activeElement)
        ? (document.activeElement as HTMLElement)
        : null,
    ),
  );
  // Keyboard focus drives the same pill, so tabbing feels like hovering.
  track.addEventListener("focusin", (e) => syncHover(track, entryOf(track, e.target)));
  // Only hide when focus actually leaves the track: moving between two entries
  // of the same control keeps the pill on the newly focused one.
  track.addEventListener("focusout", (e) => {
    const next = e.relatedTarget as Node | null;
    if (!next || !track.contains(next)) syncHover(track, null);
  });
}

/**
 * Wire up every segmented control under `root` (idempotent). Call once per
 * page, after the control's markup exists.
 */
export function initSegmented(root: ParentNode = document): void {
  root
    .querySelectorAll<HTMLElement>(".segmented")
    .forEach((track) => {
      if (track.classList.contains("seg-ready")) return;
      // Measured before the pills are allowed to show, so the first paint
      // lands on the selection instead of sliding in from the corner.
      syncActive(track);
      // Park the pointer pill on the selection too, hidden: the first hover
      // then fades in on place instead of travelling in from the left edge.
      const first = track.querySelector<HTMLElement>("button.active");
      if (first) write(track, "seg-hover", geometryOf(first), 0);
      observe(track);
      requestAnimationFrame(() => track.classList.add("seg-ready"));
    });
}

// Webfont metrics can shift a label a pixel or two after first paint.
if (typeof document !== "undefined") {
  void document.fonts?.ready.then(() => {
    document.querySelectorAll<HTMLElement>(".segmented").forEach((track) => {
      if (track.offsetWidth === 0) return;
      syncActive(track);
    });
  });
}
