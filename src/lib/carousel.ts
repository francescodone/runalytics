// Card-row carousels.
//
// The dashboard's device-data rows (readiness sub-scores, recovery signals,
// fitness) are one horizontal line of cards. When they fit they fill the row;
// when the window narrows — or a row simply has more cards than fit — the row
// scrolls sideways with snap instead of wrapping to a second line and growing
// the page. A pair of edge buttons pages the row and a row of dots under it
// shows how far along it is; each hides at its own end, and both hide entirely
// when there is nothing to scroll.
//
// The buttons are injected here (not authored in HTML) so every carousel is
// one `<div class="carousel">` wrapping its `.cards` — and so they stay inside
// the production CSP (no inline script). They live as siblings of `.cards`, so
// re-rendering the cards' innerHTML (a range change) never removes them; we
// just re-measure. All the visual motion lives in CSS.

import { icons } from "./icons";

const reduceMotion = () =>
  window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;

function trackOf(carousel: HTMLElement): HTMLElement | null {
  return carousel.querySelector<HTMLElement>(".cards");
}

function dotsOf(carousel: HTMLElement): HTMLElement | null {
  return carousel.querySelector<HTMLElement>(".carousel-dots");
}

/// How far the track can scroll; 0 when there is nothing to scroll.
function maxScroll(track: HTMLElement): number {
  return Math.max(0, track.scrollWidth - track.clientWidth);
}

/// How far one dot (or one button press) travels: most of a viewport, so the
/// next card lands where the first was with a little context kept on screen.
function pageStep(track: HTMLElement): number {
  return Math.max(track.clientWidth * 0.8, 200);
}

/// How many stops the row has — one per screenful of cards, so the first and
/// last dot are always reachable. Deliberately NOT one per card: a row usually
/// shows most of its cards at once (four signals, three visible), so four dots
/// would share ~120px of scroll, several of them would map to the same
/// position, and some could never light. A dot you can't reach is worse than a
/// dot that stands for a page.
function pageCount(track: HTMLElement): number {
  const max = maxScroll(track);
  return max > 0 ? Math.max(2, Math.ceil(max / pageStep(track)) + 1) : 1;
}

/// Which dot is lit: scroll progress across the stops, 0 at the start and the
/// last one at the end.
function activeIndex(track: HTMLElement): number {
  const n = pageCount(track);
  if (n <= 1) return 0;
  const max = maxScroll(track);
  if (max <= 0) return 0;
  return Math.min(n - 1, Math.round((track.scrollLeft / max) * (n - 1)));
}

/// Reflect scrollability + position in classes the CSS reads. Called after
/// every render, on scroll, and on resize — the overflow state depends on the
/// window width, which the CSS alone cannot know.
function refresh(carousel: HTMLElement): void {
  const track = trackOf(carousel);
  if (!track) return;
  const overflow = track.scrollWidth - track.clientWidth > 4;
  carousel.classList.toggle("can-scroll", overflow);
  carousel.classList.toggle(
    "at-start",
    track.scrollLeft <= 2,
  );
  carousel.classList.toggle(
    "at-end",
    track.scrollLeft + track.clientWidth >= track.scrollWidth - 2,
  );
  syncDots(carousel, track);
}

/// One dot per stop. The cards are re-rendered on every range change, so the
/// dot row is rebuilt whenever the count moves and reused otherwise.
function syncDots(carousel: HTMLElement, track: HTMLElement): void {
  const n = pageCount(track);
  let dots = dotsOf(carousel);
  if (!dots) {
    dots = document.createElement("div");
    dots.className = "carousel-dots";
    carousel.appendChild(dots);
  }
  if (dots.children.length !== n) {
    dots.innerHTML = Array.from(
      { length: n },
      (_, i) =>
        `<button type="button" data-dot="${i}" aria-label="Go to cards ${i + 1} of ${n}"></button>`,
    ).join("");
  }
  markActive(carousel, track);
}

function markActive(carousel: HTMLElement, track: HTMLElement): void {
  const dots = dotsOf(carousel);
  if (!dots) return;
  const active = activeIndex(track);
  [...dots.children].forEach((dot, i) => {
    const btn = dot as HTMLButtonElement;
    if (i === active) btn.setAttribute("aria-current", "true");
    else btn.removeAttribute("aria-current");
  });
}

function makeButton(carousel: HTMLElement, dir: -1 | 1): HTMLButtonElement {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = `carousel-btn ${dir < 0 ? "prev" : "next"}`;
  btn.setAttribute("aria-label", dir < 0 ? "Scroll cards left" : "Scroll cards right");
  btn.innerHTML = dir < 0 ? icons.chevronLeft : icons.chevronRight;
  btn.addEventListener("click", () => {
    const track = trackOf(carousel);
    if (!track) return;
    track.scrollBy({ left: dir * pageStep(track), behavior: reduceMotion() ? "auto" : "smooth" });
  });
  carousel.appendChild(btn);
  return btn;
}

function enhance(carousel: HTMLElement): void {
  if (carousel.dataset.carouselReady) return;
  carousel.dataset.carouselReady = "1";
  makeButton(carousel, -1);
  makeButton(carousel, 1);
  trackOf(carousel)?.addEventListener("scroll", () => refresh(carousel), { passive: true });
  // One delegated listener covers dots that don't exist yet — the card count
  // changes with the range, and syncDots() rebuilds the row each time.
  const dots = document.createElement("div");
  dots.className = "carousel-dots";
  carousel.appendChild(dots);
  dots.addEventListener("click", (e) => {
    const btn = (e.target as HTMLElement).closest<HTMLButtonElement>("[data-dot]");
    const track = trackOf(carousel);
    if (!btn || !track) return;
    const n = pageCount(track);
    // Same progress mapping activeIndex() reads, so pressing a dot always
    // lands on a position that lights that dot.
    track.scrollTo({
      left: (Number(btn.dataset.dot) / Math.max(1, n - 1)) * maxScroll(track),
      behavior: reduceMotion() ? "auto" : "smooth",
    });
  });
}

let resizeWired = false;
function wireResize(): void {
  if (resizeWired) return;
  resizeWired = true;
  window.addEventListener(
    "resize",
    () => document.querySelectorAll<HTMLElement>(".carousel").forEach(refresh),
    { passive: true },
  );
}

/// Wire up (once) and re-measure every `.carousel` under `root`. Safe to call
/// after each render — enhancement is idempotent, measurement is cheap.
export function initCarousels(root: ParentNode = document): void {
  const carousels = root.querySelectorAll<HTMLElement>(".carousel");
  carousels.forEach((c) => {
    enhance(c);
    refresh(c);
  });
  if (carousels.length) wireResize();
}
