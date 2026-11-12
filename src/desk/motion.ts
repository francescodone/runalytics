import type { Transition, Variants } from "motion/react";
import "./tokens.css";

const tokenStyle = getComputedStyle(document.documentElement);
const seconds = (name: string) => {
  const value = tokenStyle.getPropertyValue(name).trim();
  return parseFloat(value) / (value.endsWith("ms") ? 1000 : 1);
};
const curve = (name: string) => tokenStyle.getPropertyValue(name).trim().slice(13, -1).split(",").map(Number) as [number, number, number, number];

export const springTransition = { type: "tween", duration: seconds("--motion-surface"), ease: curve("--motion-soft") } as const satisfies Transition;

export const tempo = {
  press: seconds("--motion-press"),
  feedback: seconds("--motion-feedback"),
  resultHold: seconds("--motion-result-hold") * 1000,
  change: seconds("--motion-change"),
  surface: seconds("--motion-surface"),
  exit: seconds("--motion-exit"),
  stagger: seconds("--motion-stagger"),
  landing: seconds("--motion-landing"),
  landingStagger: seconds("--motion-landing-stagger"),
  count: seconds("--motion-count"),
  review: seconds("--motion-review"),
  reviewExit: seconds("--motion-review-exit"),
  distance: parseFloat(tokenStyle.getPropertyValue("--motion-distance-enter")),
  exitDistance: parseFloat(tokenStyle.getPropertyValue("--motion-distance-exit")),
  surfaceDistance: parseFloat(tokenStyle.getPropertyValue("--motion-distance-surface")),
  viewportInset: parseFloat(tokenStyle.getPropertyValue("--motion-viewport-inset")),
  viewportThreshold: parseFloat(tokenStyle.getPropertyValue("--motion-viewport-threshold")),
  soft: curve("--motion-soft"),
  countEase: curve("--motion-count-ease"),
  notice: seconds("--notice-duration") * 1000,
  settle: curve("--motion-settle"),
  leave: curve("--motion-leave"),
  spring: springTransition,
};

export const fadeIn = { opacity: 1, transition: { duration: tempo.change, ease: tempo.settle } };
export const fadeOut = { opacity: 0, transition: { duration: tempo.exit, ease: tempo.leave } };
export const scaleIn = { opacity: 0, scale: .995 };
export const slideUp = { opacity: 0, y: tempo.distance, scale: 1 };
export const slideDown = { opacity: 0, y: tempo.exitDistance, scale: 1 };
export const staggerContainer = {
  hidden: {},
  visible: { transition: { staggerChildren: tempo.stagger } },
  exit: { transition: { staggerChildren: tempo.stagger, staggerDirection: 1 } },
} satisfies Variants;
export const staggerItem = {
  hidden: slideUp,
  visible: { opacity: 1, y: 0, scale: 1, transition: springTransition },
  exit: { ...fadeOut, y: tempo.exitDistance, scale: 1 },
} satisfies Variants;
export const pageTransition = {
  initial: (_direction: number) => ({ opacity: 0, x: 0, y: tempo.distance }),
  animate: { opacity: 1, x: 0, y: 0, transition: { ...springTransition, opacity: fadeIn.transition } },
  exit: (_direction: number) => ({ ...fadeOut, x: 0, y: tempo.exitDistance }),
} satisfies Variants;
export const modalTransition = {
  initial: { ...scaleIn, y: tempo.surfaceDistance },
  animate: { opacity: 1, x: 0, y: 0, scale: 1, transition: springTransition },
  exit: { ...fadeOut, y: tempo.exitDistance, scale: 1 },
};

export const activityTransition = {
  initial: { opacity: 0, x: 0, y: tempo.distance, scale: 1 },
  animate: { opacity: 1, x: 0, y: 0, scale: 1, transition: { duration: tempo.review, ease: tempo.soft } },
  exit: { opacity: 0, x: 0, y: tempo.exitDistance, scale: 1, transition: { duration: tempo.reviewExit, ease: tempo.soft } },
};

export function entranceFor(_identity: string, index: number, _unlock: boolean) {
  const delay = Math.max(0, index) * tempo.landingStagger;
  return {
    initial: { opacity: 0, x: 0, y: tempo.distance, scale: 1 },
    animate: { opacity: 1, x: 0, y: 0, scale: 1 },
    transition: { ...springTransition, delay, opacity: { duration: tempo.landing, delay, ease: tempo.soft } },
    delay,
  };
}

export function motionVisible(element: HTMLElement, departing = false) {
  if (!element.isConnected || !element.checkVisibility() || element.closest('[hidden]') || !departing && element.closest('[inert], [data-transition-state="exiting"], [data-step-state="exiting"]')) return false;
  const bounds = element.getBoundingClientRect();
  let top = 0, left = 0, right = innerWidth, bottom = innerHeight - tempo.viewportInset;
  for (let parent = element.parentElement; parent; parent = parent.parentElement) {
    const style = getComputedStyle(parent);
    const clip = parent.getBoundingClientRect();
    if (/auto|scroll|hidden|clip/.test(style.overflowY)) { top = Math.max(top, clip.top + parent.clientTop); bottom = Math.min(bottom, clip.top + parent.clientTop + parent.clientHeight); }
    if (/auto|scroll|hidden|clip/.test(style.overflowX)) { left = Math.max(left, clip.left + parent.clientLeft); right = Math.min(right, clip.left + parent.clientLeft + parent.clientWidth); }
  }
  return bounds.width > 0 && bounds.height > 0 && Math.min(bounds.bottom, bottom) - Math.max(bounds.top, top) >= Math.min(bounds.height * tempo.viewportThreshold, Math.max(0, bottom - top) * tempo.viewportThreshold) && Math.min(bounds.right, right) > Math.max(bounds.left, left);
}

export function motionOrder(elements: Iterable<HTMLElement>) {
  return [...elements].map(element => ({ element, bounds: element.getBoundingClientRect() })).sort((first, second) => first.bounds.top - second.bounds.top || first.bounds.left - second.bounds.left || (first.element.compareDocumentPosition(second.element) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1)).map(({ element }) => element);
}