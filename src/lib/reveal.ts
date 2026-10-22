/* Dashboard motion that is driven by where the runner is on the page, not by
   a state change: panels arrive as they are scrolled to, the group heading
   pins while its family of panels passes, and the readiness figure counts up
   once when it first renders.

   Everything here is progressive enhancement: the CSS hooks are only armed
   when these functions run (and each one bails to the final state under
   reduced motion), so a page without this module still shows everything. */

import { prefersReducedMotion } from "./motion";

/// Panels below the fold keep their entrance until they are reached, instead
/// of burning it off-screen. One observer for the whole dashboard; each
/// panel reveals once and is forgotten. Safe to call after every re-render.
let io: IntersectionObserver | null = null;
let focusSafetyArmed = false;
export function initScrollReveal(): void {
  // The dashboard owns #dash; pages without it (Plan) reveal the panels that
  // are direct children of the page body instead.
  const scope = document.getElementById("dash") ?? document.getElementById("app");
  if (!scope) return;
  const panels = [...scope.querySelectorAll<HTMLElement>(".panel")];
  if (!panels.length) return;

  if (prefersReducedMotion() || !("IntersectionObserver" in window)) {
    for (const p of panels) p.classList.add("revealed");
    return;
  }

  // Arm the CSS only now: if this module never runs, no panel is hidden.
  scope.classList.add("reveal-ready");

  // Keyboard users must never tab into a panel that is still invisible:
  // the first focus inside the dashboard reveals everything at once.
  if (!focusSafetyArmed) {
    focusSafetyArmed = true;
    document.addEventListener(
      "focusin",
      () => {
        for (const p of scope.querySelectorAll<HTMLElement>(".panel")) {
          p.classList.add("revealed");
        }
      },
      { once: true },
    );
  }

  if (!io) {
    io = new IntersectionObserver(
      (records) => {
        for (const r of records) {
          if (!r.isIntersecting) continue;
          r.target.classList.add("revealed");
          io?.unobserve(r.target);
        }
      },
      // Reveal slightly before the panel enters, so it has settled by the
      // time it is comfortably readable rather than mid-move.
      { rootMargin: "0px 0px -12% 0px", threshold: 0 },
    );
  }
  for (const p of panels) if (!p.classList.contains("revealed")) io.observe(p);
}

/// The group heading pins to the top of the viewport while its panels are on
/// screen; the `.pinned` class switches on the glass material (a sticky rule
/// alone cannot say WHEN it is stuck), and the scroll fraction drives the
/// progress rule drawn in the heading's own tail.
let progressArmed = false;
export function initGroupProgress(): void {
  if (progressArmed) return;
  progressArmed = true;

  const heads = [...document.querySelectorAll<HTMLElement>("#dash .group > .group-head")];
  // The page's own scroll rail — a hairline at the top of the window showing
  // how far down the page the runner is. Created here so every page gets it
  // from the one call; transform (not width) keeps the update on the
  // compositor.
  let rail = document.getElementById("page-progress");
  if (!rail) {
    rail = document.createElement("div");
    rail.id = "page-progress";
    rail.setAttribute("aria-hidden", "true");
    document.body.append(rail);
  }

  let queued = false;
  const update = () => {
    queued = false;
    if (rail) {
      const max = document.documentElement.scrollHeight - window.innerHeight;
      const done = max > 0 ? window.scrollY / max : 0;
      rail.style.transform = `scaleX(${Math.min(1, Math.max(0, done)).toFixed(4)})`;
    }
    for (const head of heads) {
      const group = head.parentElement;
      if (!group) continue;
      const rect = head.getBoundingClientRect();
      const groupRect = group.getBoundingClientRect();
      // The sticky offset is the toolbar height on narrow screens, 0 on
      // wide ones — read it rather than assuming, so "stuck" means stuck
      // wherever the head is allowed to rest.
      const restTop = parseFloat(getComputedStyle(head).top) || 0;
      // Stuck = pinned at its resting line and the group still has content below it.
      const pinned = rect.top <= restTop + 1 && groupRect.bottom > rect.bottom + 1;
      head.classList.toggle("pinned", pinned);
      const span = group.offsetHeight;
      const done = span > 0 ? (window.scrollY - group.offsetTop) / span : 0;
      head.style.setProperty("--group-progress", Math.min(1, Math.max(0, done)).toFixed(3));
    }
  };

  const onScroll = () => {
    if (queued) return;
    queued = true;
    window.requestAnimationFrame(update);
  };
  window.addEventListener("scroll", onScroll, { passive: true });
  window.addEventListener("resize", onScroll);
  update();
}

/// Count a metric up from zero to its value on the first paint. The final
/// value is already in the DOM, so this is decoration, never data.
export function countUp(scope: HTMLElement): void {
  const nodes = scope.querySelectorAll<HTMLElement>(".count-up[data-count-to]");
  if (!nodes.length) return;

  const DURATION = 700;
  for (const node of nodes) {
    const target = Number(node.dataset.countTo);
    if (!Number.isFinite(target)) continue;
    if (prefersReducedMotion()) {
      node.textContent = String(Math.round(target));
      continue;
    }
    const start = performance.now();
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / DURATION);
      // Decelerate like the rest of the app's motion (ease-out cubic).
      const eased = 1 - Math.pow(1 - t, 3);
      node.textContent = String(Math.round(target * eased));
      if (t < 1) window.requestAnimationFrame(tick);
    };
    window.requestAnimationFrame(tick);
  }
}
