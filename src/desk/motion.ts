import type { Transition, Variants } from "motion/react";
import "./tokens.css";

const tokenStyle = getComputedStyle(document.documentElement);
const seconds = (name: string) => parseFloat(tokenStyle.getPropertyValue(name)) / 1000;
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
export const slideUp = { opacity: 0, y: 14, scale: 1 };
export const slideDown = { opacity: 0, y: -14, scale: 1 };
export const staggerContainer = {
  hidden: {},
  visible: { transition: { staggerChildren: tempo.stagger } },
  exit: { transition: { staggerChildren: tempo.stagger, staggerDirection: 1 } },
} satisfies Variants;
export const staggerItem = {
  hidden: slideUp,
  visible: { opacity: 1, y: 0, scale: 1, transition: springTransition },
  exit: { ...fadeOut, y: -14, scale: 1 },
} satisfies Variants;
export const pageTransition = {
  initial: (_direction: number) => ({ opacity: 0, x: 0, y: 14 }),
  animate: { opacity: 1, x: 0, y: 0, transition: { ...springTransition, opacity: fadeIn.transition } },
  exit: (_direction: number) => ({ ...fadeOut, x: 0, y: -14 }),
} satisfies Variants;
export const modalTransition = {
  initial: { ...scaleIn, y: 16 },
  animate: { opacity: 1, x: 0, y: 0, scale: 1, transition: springTransition },
  exit: { ...fadeOut, y: -14, scale: 1 },
};

export const activityTransition = {
  initial: { opacity: 0, x: 0, y: 14, scale: 1 },
  animate: { opacity: 1, x: 0, y: 0, scale: 1, transition: { duration: tempo.review, ease: tempo.soft } },
  exit: { opacity: 0, x: 0, y: -14, scale: 1, transition: { duration: tempo.reviewExit, ease: tempo.soft } },
};

export function entranceFor(_identity: string, index: number, _unlock: boolean) {
  const delay = Math.max(0, index) * tempo.landingStagger;
  return {
    initial: { opacity: 0, x: 0, y: 14, scale: 1 },
    animate: { opacity: 1, x: 0, y: 0, scale: 1 },
    transition: { ...springTransition, delay, opacity: { duration: tempo.landing, delay, ease: tempo.soft } },
    delay,
  };
}