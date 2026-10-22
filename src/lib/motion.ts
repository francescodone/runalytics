/* Shared motion vocabulary.
   One place decides whether the runner asked for less motion, how a spring is
   tuned, and how programmatic scrolling behaves — so a page never hardcodes
   `behavior: "smooth"` or a cubic-bezier that ignores the preference.

   Springs come from the `motion` library rather than a hand-rolled integrator:
   a spring re-targets from the element's CURRENT on-screen value and carries
   velocity through, which is exactly what an interrupted drag needs. A CSS
   transition cannot do that, which is why gesture-driven surfaces animate here
   and static state changes stay in CSS. */

import { animate } from "motion";

/// Live check of the OS "reduce motion" setting — read at the moment of use,
/// not cached, so toggling it mid-session takes effect on the next interaction.
export function prefersReducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/// Apple's two-parameter spring, expressed in the damping/response vocabulary
/// from *Designing Fluid Interfaces*. `bounce` maps to (1 - damping): 0 is
/// critically damped (no overshoot), ~0.2 overshoots gently. `duration` is the
/// response — how long to settle, in seconds.
///
/// Default UI springs are critically damped; bounce is reserved for a gesture
/// that actually carried momentum (a flicked sheet), never a click.
export const SPRING = {
  /// A control settling into place: no overshoot.
  settle: { type: "spring", bounce: 0, duration: 0.34 } as const,
  /// A sheet released from a flick: a touch of bounce, because momentum preceded it.
  momentum: { type: "spring", bounce: 0.18, duration: 0.4 } as const,
} as const;

const axisAnimations = new WeakMap<HTMLElement, Partial<Record<"x" | "y", { stop(): void }>>>();

function stopAxis(el: HTMLElement, property: "x" | "y"): void {
  const active = axisAnimations.get(el);
  active?.[property]?.stop();
  if (active) delete active[property];
}

/// Apple's momentum projection (from the *Designing Fluid Interfaces* sample):
/// where a gesture is GOING, not where the finger stopped. `velocity` is px/s;
/// `decelerationRate` ≈ 0.998 for normal scroll feel. 1000 px/s projects
/// ~499px — the magnitude is pinned by a test so a unit slip can't hide.
export function projectEndpoint(
  current: number,
  velocity: number,
  decelerationRate = 0.998,
): number {
  return current + (velocity / 1000) * (decelerationRate / (1 - decelerationRate));
}

/// Clamp a projected target into [min, max] and pick the nearer snap point.
/// Used by the sheet: project, clamp, then decide commit-vs-reverse from the
/// projected resting point rather than the release point.
export function nearestSnap(
  projected: number,
  snaps: number[],
  min: number,
  max: number,
): number {
  const clamped = Math.min(max, Math.max(min, projected));
  let best = snaps[0];
  let bestDist = Infinity;
  for (const s of snaps) {
    const d = Math.abs(s - clamped);
    if (d < bestDist) {
      bestDist = d;
      best = s;
    }
  }
  return best;
}

/// Animate a numeric transform on an element with a spring, honouring reduced
/// motion. Returns a promise that resolves when the animation settles (or
/// immediately, under reduced motion, after snapping to the target).
///
/// `keyframes` is a single target value for the given property; `velocity` is
/// the pointer's release velocity in px/s, handed straight to the spring so the
/// drag→animate seam disappears.
export function animateSpring(
  el: HTMLElement,
  property: "x" | "y",
  target: number,
  opts: { from?: number; velocity?: number; bounce?: number; duration?: number } = {},
): Promise<void> {
  if (prefersReducedMotion()) {
    setAxis(el, property, target);
    return Promise.resolve();
  }
  // Pass an explicit [from, to] keyframe pair rather than a bare target: the
  // caller may have been writing the transform by hand during a drag, and we
  // must not rely on motion re-parsing that inline value to find its start.
  const from = opts.from ?? readAxis(el, property);
  stopAxis(el, property);
  const controls = animate(el, { [property]: [from, target] }, {
    type: "spring",
    bounce: opts.bounce ?? 0,
    duration: opts.duration ?? 0.34,
    velocity: opts.velocity ?? 0,
  });
  let resolveStopped!: () => void;
  const stopped = new Promise<void>((resolve) => { resolveStopped = resolve; });
  const current = { stop: () => { controls.stop(); resolveStopped(); } };
  const active = axisAnimations.get(el) ?? {};
  active[property] = current;
  axisAnimations.set(el, active);
  return Promise.race([controls.finished.then(() => undefined).catch(() => undefined), stopped]).finally(() => {
    if (active[property] === current) delete active[property];
  });
}

/// Write a single-axis translate without disturbing the other axis, so a sheet
/// sliding on X never fights a transform on Y.
export function setAxis(el: HTMLElement, property: "x" | "y", value: number): void {
  stopAxis(el, property);
  const px = `${Math.round(value * 100) / 100}px`;
  if (property === "x") el.style.transform = `translateX(${px})`;
  else el.style.transform = `translateY(${px})`;
}

/// Read the current on-screen translate, including an in-flight spring, so
/// grabbing a moving sheet starts exactly where it is rather than jumping.
export function readAxis(el: HTMLElement, property: "x" | "y"): number {
  const computed = getComputedStyle(el);
  if (computed.transform !== "none") {
    const matrix = new DOMMatrixReadOnly(computed.transform);
    return property === "x" ? matrix.m41 : matrix.m42;
  }
  if (computed.translate !== "none") {
    const [x, y = "0px"] = computed.translate.split(" ");
    return parseFloat(property === "x" ? x : y) || 0;
  }
  const m = el.style.transform.match(
    property === "x" ? /translateX\((-?[\d.]+)px\)/ : /translateY\((-?[\d.]+)px\)/,
  );
  return m ? parseFloat(m[1]) : 0;
}

/// Scroll an element into view the way the runner asked for motion: smooth by
/// default, instant under reduced motion. Replaces the raw
/// `scrollIntoView({ behavior: "smooth" })` calls that ignored the preference.
export function scrollIntoViewMotion(
  el: Element,
  opts: ScrollIntoViewOptions = {},
): void {
  el.scrollIntoView({
    behavior: prefersReducedMotion() ? "auto" : "smooth",
    ...opts,
  });
}

/// A short scroll for programmatic navigation (pane switches, "jump to result").
export function scrollToMotion(
  el: Element,
  opts: ScrollToOptions & { block?: ScrollLogicalPosition } = {},
): void {
  const behavior: ScrollBehavior = prefersReducedMotion() ? "auto" : "smooth";
  el.scrollIntoView({ behavior, ...opts });
}

/// Present a centered surface the way iOS presents a card: it grows into place
/// from a hair smaller and fades up, so it reads as coming toward you rather
/// than blinking on. The exit is the reverse and quicker — dismissals should
/// never feel heavier than presentations. Under reduced motion both are a
/// plain show/hide (no scale, no fade), per the platform contract.
export function animatePresent(el: HTMLElement): Promise<void> {
  if (prefersReducedMotion()) {
    el.style.opacity = "1";
    el.style.transform = "none";
    return Promise.resolve();
  }
  const controls = animate(
    el,
    { opacity: [0, 1], scale: [0.96, 1] },
    { duration: 0.28, ease: "easeOut" },
  );
  return controls.finished.then(() => undefined).catch(() => undefined);
}

/// The dismissal half of `animatePresent`. Kept separate (rather than a
/// boolean) so callers can await it and tear the element down on completion.
export function animateDismiss(el: HTMLElement): Promise<void> {
  if (prefersReducedMotion()) {
    el.style.opacity = "0";
    return Promise.resolve();
  }
  const controls = animate(
    el,
    { opacity: [1, 0], scale: [1, 0.96] },
    { duration: 0.2, ease: "easeIn" },
  );
  return controls.finished.then(() => undefined).catch(() => undefined);
}

/// Slide a freshly-rendered panel in from a direction, for step-by-step
/// surfaces (the tutorial) where content replaces content and the direction of
/// travel tells you whether you went forward or back. Reduced motion: a plain
/// opacity fade with no travel.
export function animateSlideIn(el: HTMLElement, from: "next" | "back"): Promise<void> {
  if (prefersReducedMotion()) {
    const fade = animate(el, { opacity: [0, 1] }, { duration: 0.15 });
    return fade.finished.then(() => undefined).catch(() => undefined);
  }
  const offset = from === "next" ? 24 : -24;
  const controls = animate(
    el,
    { opacity: [0, 1], x: [offset, 0] },
    { duration: 0.3, ease: "easeOut" },
  );
  return controls.finished.then(() => undefined).catch(() => undefined);
}

export function animatePaneChange(el: HTMLElement, from: "next" | "back"): Promise<void> {
  const controls = prefersReducedMotion()
    ? animate(el, { opacity: [0.85, 1] }, { duration: 0.12 })
    : animate(el, { opacity: [0.65, 1], x: [from === "next" ? 12 : -12, 0] }, SPRING.settle);
  return controls.finished.then(() => undefined).catch(() => undefined);
}
