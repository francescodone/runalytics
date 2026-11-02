import { useEffect, useId, useState, type ReactNode } from "react";
import { AnimatePresence, LayoutGroup, MotionConfig, motion, useReducedMotion } from "motion/react";
import "./tokens.css";

const tokenStyle = getComputedStyle(document.documentElement);
const seconds = (name: string) => parseFloat(tokenStyle.getPropertyValue(name)) / 1000;
const curve = (name: string) => tokenStyle.getPropertyValue(name).trim().slice(13, -1).split(",").map(Number) as [number, number, number, number];

export const tempo = {
  press: seconds("--motion-press"),
  feedback: seconds("--motion-feedback"),
  change: seconds("--motion-change"),
  surface: seconds("--motion-surface"),
  exit: seconds("--motion-exit"),
  stagger: seconds("--motion-stagger"),
  notice: seconds("--notice-duration") * 1000,
  settle: curve("--motion-settle"),
  leave: curve("--motion-leave"),
  spring: { type: "spring", visualDuration: seconds("--motion-spring"), bounce: 0 } as const,
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
    const selector = "[data-entrance], .workspace-heading, .training-context, .desk-toolbar, .health-verdict, .health-analysis, .health-history, .session-tile, .activity-row, .roadmap-row, .settings-task-list > a, .settings-editor > header, .settings-editor > p, .settings-editor > section, .settings-editor > form, fieldset, .settings-section, .connection-summary, .form-grid, .form-actions, .milestones > *, .milestone-list > article, .desk-empty, .review-section, .review-heading, .plan-basis, .plan-roadmap, .adjustment-evidence, .route-figure, .setup-band, .prescribed-session, .month-board, .week-board, .mobile-date-ribbon, .pain-episode, .episode-chat, .proposal-prompt, .pace-next, .pace-insights-figure, .pace-session-detail, .race-estimates, .help-articles > details, .journal-tools, .session-management, .session-recording, .watch-predictions, .composer-summary, .dialog-surface > :not(header)";
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
        const parentRevealing = element.parentElement?.closest('[data-reveal="visible"]');
        element.style.setProperty("--reveal-delay", `${Math.min(order++, 3) * tempo.stagger}s`);
        element.dataset.reveal = reduced.matches || parentRevealing ? "settled" : "visible";
      }
    }, { threshold: 0 });
    const register = (root: ParentNode) => {
      const elements = [...root.querySelectorAll<HTMLElement>(selector)];
      if (root instanceof HTMLElement && root.matches(selector)) elements.unshift(root);
      for (const element of elements) {
        if (revealed.has(element) || pending.has(element)) continue;
        if (reduced.matches) { revealed.add(element); element.dataset.reveal = "settled"; continue; }
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
    const settleMotion = () => {
      if (reduced.matches) document.querySelectorAll<HTMLElement>('[data-reveal="visible"]').forEach(element => { element.dataset.reveal = "settled"; });
    };
    reduced.addEventListener("change", settleMotion);
    return () => { observer.disconnect(); mutations.disconnect(); reduced.removeEventListener("change", settleMotion); document.removeEventListener("animationend", settle); document.removeEventListener("animationcancel", settle); };
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
  return (
    <AnimatePresence mode="wait" initial={false}>
      <motion.div
        key={identity}
        className={`motion-change ${className}`}
        exit={reduced ? undefined : { opacity: 0, transition: { duration: tempo.exit, ease: tempo.leave } }}
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