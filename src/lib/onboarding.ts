// First-run tutorial: a full-screen, Apple-style walkthrough shown once on
// the dashboard. One idea per slide, moved through one at a time; the user
// can skip at any point. Completion is remembered in localStorage so the
// app opens directly afterwards. Settings has a "Replay tutorial" button
// that calls startOnboarding() again.

import { icons } from "./icons";
import { createModal } from "./overlay";
import { animateSlideIn } from "./motion";

const STORAGE_KEY = "runalytics.onboarding.v1";

interface Slide {
  icon: string;
  title: string;
  body: string;
}

const SLIDES: Slide[] = [
  {
    icon: icons.run,
    title: "Welcome to Runalytics",
    body: "Training analytics for runners. Every number comes from your own watch — and stays on your Mac.",
  },
  {
    icon: icons.watch,
    title: "Connect your watch",
    body: "Sign in to COROS once from Settings. Runalytics uses the official API and keeps the token on this machine.",
  },
  {
    icon: icons.sync,
    title: "Sync your history",
    body: "One pull brings your runs, sleep, heart rate and training load. Do it again any time from Settings.",
  },
  {
    icon: icons.calendar,
    title: "Plan a race",
    body: "Pick a distance and a race date — Runalytics builds a week-by-week plan and pushes it to your watch.",
  },
  {
    icon: icons.chart,
    title: "Track your progress",
    body: "The dashboard shows how ready you are today; the calendar shows what to run next. One step at a time.",
  },
];

/// True when the tutorial has never been completed on this machine.
export function onboardingPending(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) !== "done";
  } catch {
    return false;
  }
}

/// Marks the tutorial as completed (used by tests / replay flows).
export function completeOnboarding(): void {
  try {
    localStorage.setItem(STORAGE_KEY, "done");
  } catch {
    // Storage unavailable (private mode): the tutorial will simply show again.
  }
}

/// Shows the tutorial overlay. Does nothing if one is already open.
export function startOnboarding(onDone: () => void = () => undefined): void {
  if (document.getElementById("onboarding")) return;

  let index = 0;
  // The card is the tutorial's frame; the slide content inside it is replaced
  // per step. Wrapping the content in .ob-slide lets each step slide in from
  // the direction of travel (forward = from the right) while the card and its
  // Skip button stay put.
  const card = document.createElement("div");
  card.className = "ob-card";
  card.innerHTML = `<button class="ob-skip" type="button">Skip tour</button><div class="ob-slide"></div>`;

  // Native <dialog> via the shared controller: focus trap, Escape, backdrop
  // and focus restore all come from there. Any dismissal (Skip, Escape, or a
  // click outside) completes the tour — same semantics as the old Skip button.
  const modal = createModal({
    card,
    label: "Welcome tutorial",
    className: "onboarding-modal",
    onClosed: () => {
      completeOnboarding();
      onDone();
    },
  });

  const slide = card.querySelector<HTMLElement>(".ob-slide")!;
  card.querySelector(".ob-skip")!.addEventListener("click", () => modal.close());

  const render = (from: "next" | "back" = "next") => {
    const s = SLIDES[index];
    const last = index === SLIDES.length - 1;
    slide.innerHTML = `
      <div class="ob-icon medallion lg" aria-hidden="true">${s.icon}</div>
      <h2 class="ob-title">${s.title}</h2>
      <p class="ob-body">${s.body}</p>
      <div class="ob-dots" aria-hidden="true">
        ${SLIDES.map((_, i) => `<span class="ob-dot${i === index ? " active" : ""}"></span>`).join("")}
      </div>
      <div class="ob-actions">
        ${index > 0 ? `<button class="ob-back secondary" type="button">Back</button>` : ""}
        <button class="ob-next big" type="button">${last ? "Get started" : "Continue"}</button>
      </div>`;

    slide.querySelector(".ob-back")?.addEventListener("click", () => {
      index -= 1;
      render("back");
    });
    slide.querySelector(".ob-next")!.addEventListener("click", () => {
      if (last) modal.close();
      else {
        index += 1;
        render("next");
      }
    });
    slide.querySelector<HTMLElement>(".ob-next")!.focus();
    void animateSlideIn(slide, from);
  };

  // Register the singleton before opening so a second call is a no-op.
  modal.root.id = "onboarding";
  modal.root.addEventListener("close", () => modal.root.removeAttribute("id"), { once: true });
  render();
  modal.open();
}
