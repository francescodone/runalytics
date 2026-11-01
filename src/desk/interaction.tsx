import { useEffect, useId, useState, type ReactNode, type RefObject } from "react";
import { AnimatePresence, LayoutGroup, MotionConfig, motion, useReducedMotion, useSpring } from "motion/react";

export const tempo = {
  press: .09,
  feedback: .18,
  change: .3,
  surface: .42,
  exit: .2,
  settle: [ .16, 1, .3, 1 ] as const,
  leave: [ .4, 0, .8, .4 ] as const,
  spring: { type: "spring", visualDuration: .36, bounce: 0 } as const,
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
  useEffect(() => {
    const selector = ".health-verdict, .health-analysis, .health-history, .session-tile, .activity-row, .roadmap-row, .settings-task-list > a, .settings-editor, .milestones > *, .desk-empty, .review-section, .plan-basis, .plan-roadmap, .adjustment-evidence, .route-figure, .setup-band, .prescribed-session, .month-board, .week-board";
    const pending = new Set<HTMLElement>();
    const revealed = new WeakSet<HTMLElement>();
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    const observer = new IntersectionObserver(entries => {
      let order = 0;
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        const element = entry.target as HTMLElement;
        observer.unobserve(element);
        pending.delete(element);
        revealed.add(element);
        element.style.setProperty("--reveal-delay", `${Math.min(order++, 4) * 45}ms`);
        element.dataset.reveal = "visible";
      }
    }, { threshold: .08 });
    const register = (root: ParentNode) => {
      const elements = [...root.querySelectorAll<HTMLElement>(selector)];
      if (root instanceof HTMLElement && root.matches(selector)) elements.unshift(root);
      for (const element of elements) {
        if (revealed.has(element) || pending.has(element)) continue;
        if (reduced.matches) { revealed.add(element); continue; }
        pending.add(element);
        observer.observe(element);
      }
    };
    const mutations = new MutationObserver(records => {
      for (const element of pending) {
        if (!element.isConnected) { observer.unobserve(element); pending.delete(element); }
      }
      for (const record of records) {
        for (const node of record.addedNodes) if (node instanceof HTMLElement) register(node);
      }
    });
    register(document);
    mutations.observe(document.body, { childList: true, subtree: true });
    const settle = (event: AnimationEvent) => {
      if (event.animationName === "content-arrive" && event.target instanceof HTMLElement) event.target.dataset.reveal = "settled";
    };
    document.addEventListener("animationend", settle);
    document.addEventListener("animationcancel", settle);
    return () => { observer.disconnect(); mutations.disconnect(); document.removeEventListener("animationend", settle); document.removeEventListener("animationcancel", settle); };
  }, []);
  // Scroll-edge cues: a list or tab strip that still has room to travel marks
  // the open edges, and CSS wears an inner shadow on exactly those edges, so
  // the runner can see which way there is more to see before they try to
  // scroll. Attributes rather than classes because the marks are pure
  // presentation and must never collide with a component's own class logic.
  useEffect(() => {
    const selector = ".data-table-scroll, .lap-chart-scroll, .review-tabs, .horizon-switch, .plan-roadmap, .activity-library";
    const mark = (element: HTMLElement) => {
      const slackX = element.scrollWidth - element.clientWidth;
      const slackY = element.scrollHeight - element.clientHeight;
      element.dataset.scrollLeft = element.scrollLeft > 2 ? "1" : "";
      element.dataset.scrollRight = element.scrollLeft < slackX - 2 ? "1" : "";
      element.dataset.scrollTop = element.scrollTop > 2 ? "1" : "";
      element.dataset.scrollBottom = element.scrollTop < slackY - 2 ? "1" : "";
    };
    const refresh = () => document.querySelectorAll<HTMLElement>(selector).forEach(mark);
    const onScroll = (event: Event) => {
      const element = event.target;
      if (element instanceof HTMLElement && element.matches(selector)) mark(element);
    };
    let frame = 0;
    const mutations = new MutationObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(refresh);
    });
    refresh();
    document.addEventListener("scroll", onScroll, { capture: true, passive: true });
    window.addEventListener("resize", refresh);
    mutations.observe(document.body, { childList: true, subtree: true });
    return () => {
      cancelAnimationFrame(frame);
      document.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", refresh);
      mutations.disconnect();
    };
  }, []);
  return <MotionConfig reducedMotion="user" transition={tempo.spring}>{children}</MotionConfig>;
}

export function useActionDepth(control: RefObject<HTMLButtonElement | null>, enabled: boolean) {
  const rotateX = useSpring(0, { stiffness: 300, damping: 30, mass: .6 });
  const rotateY = useSpring(0, { stiffness: 300, damping: 30, mass: .6 });
  const lift = useSpring(0, { stiffness: 300, damping: 30, mass: .6 });
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
  // Switching what is mounted (a horizon, a health tab, a selected day) reads as
  // one surface dissolving into the next: the outgoing pane blurs away, then the
  // incoming pane sharpens in. The arrival is a CSS keyframe rather than an
  // inline motion style on purpose — a settled inline filter would turn this
  // wrapper into the containing block for the mobile full-screen inspector.
  return (
    <AnimatePresence mode="wait" initial={false}>
      <motion.div
        key={identity}
        className={`motion-change ${className}`}
        exit={reduced ? undefined : { opacity: 0, filter: "blur(8px)", transition: { duration: tempo.exit, ease: "easeIn" } }}
      >
        {children}
      </motion.div>
    </AnimatePresence>
  );
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
  const offset = reduced ? {} : kind === "inspector" ? { x: compact ? "100%" : 16 } : { y: compact ? "100%" : 16, scale: compact ? 1 : .97 };
  return {
    compact,
    reduced,
    initial: { opacity: reduced || compact ? 1 : 0, ...offset },
    animate: { opacity: 1, x: 0, y: 0, scale: 1, transition: reduced ? { duration: 0 } : tempo.spring },
    exit: { opacity: compact && !reduced ? 1 : 0, ...(reduced ? {} : kind === "inspector" ? { x: compact ? "100%" : 8 } : { y: compact ? 48 : 8, scale: compact ? 1 : .985 }), transition: { duration: reduced ? 0 : tempo.exit, ease: tempo.leave } },
  };
}