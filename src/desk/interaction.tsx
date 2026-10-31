import { useEffect, useId, useRef, useState, type ReactNode, type RefObject } from "react";
import { AnimatePresence, LayoutGroup, MotionConfig, motion, useReducedMotion, useSpring } from "motion/react";

export const tempo = {
  press: .08,
  feedback: .14,
  change: .22,
  surface: .32,
  exit: .18,
  settle: [ .22, 1, .36, 1 ] as const,
  leave: [ .4, 0, 1, 1 ] as const,
  spring: { type: "spring", visualDuration: .28, bounce: 0 } as const,
};

let initiatingControl: HTMLButtonElement | null = null;
export function markActionOrigin(control: HTMLButtonElement) {
  initiatingControl = control;
  queueMicrotask(() => { if (initiatingControl === control) initiatingControl = null; });
}
export function actionOrigin() {
  const active = document.activeElement;
  return initiatingControl ?? (active instanceof HTMLButtonElement ? active : active?.closest("form")?.querySelector<HTMLButtonElement>('button[type="submit"]') ?? null);
}

export function MotionSystem({ children }: { children: ReactNode }) {
  return <MotionConfig reducedMotion="user" transition={tempo.spring}>{children}</MotionConfig>;
}

export function useActionDepth(control: RefObject<HTMLButtonElement | null>, enabled: boolean) {
  const rotateX = useSpring(0, { stiffness: 400, damping: 32, mass: .5 });
  const rotateY = useSpring(0, { stiffness: 400, damping: 32, mass: .5 });
  const lift = useSpring(0, { stiffness: 400, damping: 32, mass: .5 });
  useEffect(() => {
    const element = control.current;
    if (!element || !enabled) return;
    const media = matchMedia("(hover: hover) and (pointer: fine) and (prefers-reduced-motion: no-preference)");
    const reset = () => { rotateX.set(0); rotateY.set(0); lift.set(0); };
    const stop = () => { rotateX.jump(0); rotateY.jump(0); lift.jump(0); };
    const move = (event: PointerEvent) => {
      if (!media.matches || event.pointerType !== "mouse" || event.buttons || element.disabled) { stop(); return; }
      const bounds = element.getBoundingClientRect();
      const horizontal = Math.max(-1, Math.min(1, (event.clientX - bounds.left) / bounds.width * 2 - 1));
      const vertical = Math.max(-1, Math.min(1, (event.clientY - bounds.top) / bounds.height * 2 - 1));
      rotateX.set(-vertical * 1.5);
      rotateY.set(horizontal * 1.5);
      lift.set(-1);
    };
    element.addEventListener("pointerenter", move);
    element.addEventListener("pointermove", move);
    element.addEventListener("pointerup", move);
    element.addEventListener("pointerdown", stop);
    element.addEventListener("pointerleave", reset);
    element.addEventListener("pointercancel", stop);
    element.addEventListener("blur", reset);
    window.addEventListener("blur", stop);
    media.addEventListener("change", stop);
    return () => {
      element.removeEventListener("pointerenter", move);
      element.removeEventListener("pointermove", move);
      element.removeEventListener("pointerup", move);
      element.removeEventListener("pointerdown", stop);
      element.removeEventListener("pointerleave", reset);
      element.removeEventListener("pointercancel", stop);
      element.removeEventListener("blur", reset);
      window.removeEventListener("blur", stop);
      media.removeEventListener("change", stop);
      stop();
    };
  }, [control, enabled, rotateX, rotateY, lift]);
  return { rotateX, rotateY, y: lift, transformPerspective: 600 };
}

export function Presence({ children }: { children: ReactNode }) {
  return <AnimatePresence initial={false}>{children}</AnimatePresence>;
}

export function SelectionGroup({ children }: { children: ReactNode }) {
  return <LayoutGroup id={useId()}>{children}</LayoutGroup>;
}

export function SelectionMarker() {
  const reduced = useReducedMotion();
  return <motion.span className="selection-marker" aria-hidden="true" layoutId="selection" initial={false} transition={reduced ? { duration: 0 } : tempo.spring} />;
}

export function Change({ children, identity, className = "" }: { children: ReactNode; identity: string | number; className?: string }) {
  const reduced = useReducedMotion();
  const mounted = useRef(false);
  useEffect(() => { mounted.current = true; }, []);
  return <motion.div key={identity} className={`motion-change ${className}`} initial={mounted.current ? { opacity: .65 } : false} animate={{ opacity: 1 }} transition={{ duration: reduced ? .08 : tempo.change, ease: tempo.settle }}>{children}</motion.div>;
}

export function useCompactLayout() {
  const [compact, setCompact] = useState(() => matchMedia("(max-width: 800px)").matches);
  useEffect(() => {
    const media = matchMedia("(max-width: 800px)");
    const update = () => setCompact(media.matches);
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);
  return compact;
}

export function useSurfaceMotion(kind: "dialog" | "inspector") {
  const compact = useCompactLayout();
  const reduced = useReducedMotion();
  const offset = reduced ? {} : kind === "inspector" ? { x: compact ? "100%" : 18 } : { y: compact ? "100%" : 12, scale: compact ? 1 : .985 };
  return {
    compact,
    reduced,
    initial: { opacity: reduced || compact ? 1 : .35, ...offset },
    animate: { opacity: 1, x: 0, y: 0, scale: 1, transition: reduced ? { duration: .08 } : tempo.spring },
    exit: { opacity: compact && !reduced ? 1 : 0, ...offset, transition: { duration: reduced ? .08 : tempo.exit, ease: tempo.leave } },
  };
}