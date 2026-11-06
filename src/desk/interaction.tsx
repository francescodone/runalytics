import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { AnimatePresence, MotionConfig, motion, useIsPresent, useReducedMotion } from "motion/react";
import "./tokens.css";

const tokenStyle = getComputedStyle(document.documentElement);
const seconds = (name: string) => parseFloat(tokenStyle.getPropertyValue(name)) / 1000;
const curve = (name: string) => tokenStyle.getPropertyValue(name).trim().slice(13, -1).split(",").map(Number) as [number, number, number, number];

export const tempo = {
  press: seconds("--motion-press"),
  feedback: seconds("--motion-feedback"),
  resultHold: seconds("--motion-result-hold") * 1000,
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
    const selector = "button, .desk-navigation a, .horizon-switch a, .settings-task-list a, .choice-row label, .palette-options label, summary";
    const fine = matchMedia("(hover: hover) and (pointer: fine)");
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    let active: HTMLElement | null = null;
    let pressed: HTMLElement | null = null;
    let frame = 0;
    let pointerX = 0;
    let pointerY = 0;
    const pulses = new Map<HTMLElement, Animation>();
    const reset = () => {
      if (!active) return;
      active.removeAttribute("data-aura-tracking");
      active.style.removeProperty("--aura-tilt-x");
      active.style.removeProperty("--aura-tilt-y");
      active = null;
    };
    const targetControl = (target: EventTarget | null) => {
      const element = target instanceof Element ? target.closest<HTMLElement>(selector) : null;
      return element && !element.matches(':disabled, [aria-disabled="true"], :has(:disabled), .is-dragging') ? element : null;
    };
    const position = (element: HTMLElement, clientX: number, clientY: number) => {
      const bounds = element.getBoundingClientRect();
      const horizontal = Math.max(0, Math.min(1, (clientX - bounds.left) / Math.max(1, bounds.width)));
      const vertical = Math.max(0, Math.min(1, (clientY - bounds.top) / Math.max(1, bounds.height)));
      element.style.setProperty("--aura-x", `${horizontal * 100}%`);
      element.style.setProperty("--aura-y", `${vertical * 100}%`);
      element.style.setProperty("--aura-tilt-x", `${(0.5 - vertical) * 1.2}deg`);
      element.style.setProperty("--aura-tilt-y", `${(horizontal - 0.5) * 1.2}deg`);
    };
    const move = (event: PointerEvent) => {
      if (!fine.matches || event.pointerType === "touch") return;
      const control = targetControl(event.target);
      if (control !== active) { reset(); active = control; }
      pointerX = event.clientX;
      pointerY = event.clientY;
      if (frame || !active) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        if (!active?.isConnected) { reset(); return; }
        position(active, pointerX, pointerY);
        active.dataset.auraTracking = "true";
      });
    };
    const release = () => {
      pressed?.removeAttribute("data-aura-pressed");
      pressed = null;
    };
    const press = (event: PointerEvent) => {
      if (event.button !== 0) return;
      const control = targetControl(event.target);
      if (!control) return;
      release();
      pressed = control;
      control.dataset.auraPressed = "true";
      position(control, event.clientX, event.clientY);
      pulses.get(control)?.cancel();
      if (reduced.matches) return;
      const pulse = control.animate([
        { backgroundSize: "100% 100%" },
        { backgroundSize: "155% 155%", offset: .35 },
        { backgroundSize: "100% 100%" },
      ], { duration: tempo.surface * 1000, easing: "ease-out" });
      pulses.set(control, pulse);
      pulse.finished.then(() => { if (pulses.get(control) === pulse) pulses.delete(control); }, () => {});
    };
    const leave = (event: PointerEvent) => { if (!event.relatedTarget) reset(); };
    const focus = (event: FocusEvent) => {
      const control = targetControl(event.target);
      if (!control?.matches(":focus-visible")) return;
      reset();
      control.style.setProperty("--aura-x", "50%");
      control.style.setProperty("--aura-y", "50%");
    };
    const clear = () => {
      reset();
      release();
      for (const pulse of pulses.values()) pulse.cancel();
      pulses.clear();
    };
    document.addEventListener("pointermove", move, { passive: true });
    document.addEventListener("pointerdown", press, true);
    document.addEventListener("pointerup", release, true);
    document.addEventListener("pointerout", leave);
    document.addEventListener("pointercancel", clear);
    document.addEventListener("focusin", focus);
    window.addEventListener("blur", clear);
    reduced.addEventListener("change", clear);
    return () => {
      cancelAnimationFrame(frame);
      clear();
      document.removeEventListener("pointermove", move);
      document.removeEventListener("pointerdown", press, true);
      document.removeEventListener("pointerup", release, true);
      document.removeEventListener("pointerout", leave);
      document.removeEventListener("pointercancel", clear);
      document.removeEventListener("focusin", focus);
      window.removeEventListener("blur", clear);
      reduced.removeEventListener("change", clear);
    };
  }, []);
  useEffect(() => {
    const selector = "[data-entrance], .workspace-heading, .training-context, .desk-toolbar, .health-verdict, .health-context > *, .health-history > *, .trend-chart, .session-tile, .activity-row, .roadmap-row, .settings-task-list > a, .settings-editor > header, .settings-editor > p, fieldset, .connection-summary, .form-grid, .form-actions, .milestone-list > article, .desk-empty, .review-section, .review-heading, .plan-basis, .adjustment-evidence, .route-figure, .setup-band, .prescribed-session, .month-labels, .month-day, .day-lane > header, .mobile-date-ribbon, .pain-episode > header, .episode-notes, .episode-actions, .episode-chat > h3, .episode-chat > form, .chat-message, .proposal-prompt, .pace-next, .pace-insights-figure, .pace-session-detail, .race-estimates, .dialog-action > .desk-button, .journal-tools, .session-management, .session-recording, .watch-predictions, .composer-summary, .dialog-surface > header";
    const pending = new Set<HTMLElement>();
    const revealed = new WeakSet<HTMLElement>();
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    const compact = matchMedia("(max-width: 800px)");
    let sequence = 0;
    const settleElement = (element: HTMLElement) => {
      observer.unobserve(element);
      pending.delete(element);
      revealed.add(element);
      element.dataset.reveal = "settled";
    };
    const observer = new IntersectionObserver(entries => {
      let order = 0;
      for (const entry of entries) {
        if (!entry.isIntersecting || entry.intersectionRatio < .08) continue;
        const element = entry.target as HTMLElement;
        if (!pending.has(element)) continue;
        observer.unobserve(element);
        pending.delete(element);
        revealed.add(element);
        const group = element.closest(".motion-change[data-stagger]");
        const staggerIndex = group ? [...group.querySelectorAll('[data-reveal]:not([data-reveal="container"])')].indexOf(element) : order++;
        element.style.setProperty("--reveal-delay", `${Math.min(Math.max(0, staggerIndex), 3) * tempo.stagger}s`);
        element.dataset.reveal = reduced.matches || compact.matches || element.contains(document.activeElement) ? "settled" : "visible";
      }
    }, { threshold: .08, rootMargin: "0px 0px -24px 0px" });
    const register = (root: ParentNode) => {
      const elements = [...root.querySelectorAll<HTMLElement>(selector)];
      if (root instanceof HTMLElement && root.matches(selector)) elements.unshift(root);
      for (const element of elements) {
        if (revealed.has(element) || pending.has(element)) continue;
        if (element.closest(".motion-disclosure")) continue;
        if (element.querySelector(`${selector}, .desk-inspector, .desk-dialog, .notification-viewport`)) continue;
        for (let ancestor = element.parentElement?.closest<HTMLElement>("[data-reveal]"); ancestor; ancestor = ancestor.parentElement?.closest<HTMLElement>("[data-reveal]")) {
          settleElement(ancestor);
          ancestor.dataset.reveal = "container";
        }
        element.style.setProperty("--scatter-direction", String(sequence++ % 3 - 1));
        if (reduced.matches || compact.matches || element.contains(document.activeElement) || element.matches(".activity-row, .roadmap-row")) { settleElement(element); continue; }
        element.dataset.reveal = "waiting";
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
      if (["content-arrive", "status-arrive"].includes(event.animationName) && event.target instanceof HTMLElement && event.target.dataset.reveal === "visible") settleElement(event.target);
    };
    const settleInteraction = (event: Event) => {
      if (!(event.target instanceof Element)) return;
      for (let element = event.target.closest<HTMLElement>('[data-reveal="waiting"], [data-reveal="visible"]'); element; element = element.parentElement?.closest<HTMLElement>('[data-reveal="waiting"], [data-reveal="visible"]') ?? null) settleElement(element);
    };
    document.addEventListener("animationend", settle);
    document.addEventListener("animationcancel", settle);
    document.addEventListener("focusin", settleInteraction);
    document.addEventListener("pointerdown", settleInteraction, true);
    const settleMotion = () => {
      if (reduced.matches || compact.matches) document.querySelectorAll<HTMLElement>('[data-reveal="waiting"], [data-reveal="visible"]').forEach(settleElement);
    };
    reduced.addEventListener("change", settleMotion);
    compact.addEventListener("change", settleMotion);
    return () => {
      document.querySelectorAll<HTMLElement>('[data-reveal="waiting"], [data-reveal="visible"]').forEach(settleElement);
      observer.disconnect(); mutations.disconnect(); reduced.removeEventListener("change", settleMotion);
      compact.removeEventListener("change", settleMotion);
      document.removeEventListener("animationend", settle); document.removeEventListener("animationcancel", settle);
      document.removeEventListener("focusin", settleInteraction); document.removeEventListener("pointerdown", settleInteraction, true);
    };
  }, []);
  // Scroll-edge cues: a list or tab strip that still has room to travel marks
  // the open edges, and CSS wears an inner shadow on exactly those edges, so
  // the runner can see which way there is more to see before they try to
  // scroll. Attributes rather than classes because the marks are pure
  // presentation and must never collide with a component's own class logic.
  useEffect(() => {
    const selector = ".data-table-scroll, .lap-chart-scroll, .review-tabs, .horizon-switch, .pace-session-picker, .plan-roadmap, .activity-library";
    const mark = (element: HTMLElement) => {
      const slackX = element.scrollWidth - element.clientWidth;
      const slackY = element.scrollHeight - element.clientHeight;
      element.dataset.scrollLeft = element.scrollLeft > 2 ? "1" : "";
      element.dataset.scrollRight = element.scrollLeft < slackX - 2 ? "1" : "";
      element.dataset.scrollTop = element.scrollTop > 2 ? "1" : "";
      element.dataset.scrollBottom = element.scrollTop < slackY - 2 ? "1" : "";
      element.style.setProperty("--scroll-left-opacity", String(Math.min(1, Math.max(0, element.scrollLeft) / 14)));
      element.style.setProperty("--scroll-right-opacity", String(Math.min(1, Math.max(0, slackX - element.scrollLeft) / 14)));
      element.style.setProperty("--scroll-top-opacity", String(Math.min(1, Math.max(0, element.scrollTop) / 32)));
      element.style.setProperty("--scroll-bottom-opacity", String(Math.min(1, Math.max(0, slackY - element.scrollTop) / 32)));
    };
    const observed = new Set<Element>();
    let frame = 0;
    const refresh = () => {
      for (const element of observed) if (!element.isConnected) { resize.unobserve(element); observed.delete(element); }
      document.querySelectorAll<HTMLElement>(selector).forEach(element => {
        if (!observed.has(element)) { observed.add(element); resize.observe(element); }
        if (element.matches(".plan-roadmap, .activity-library")) {
          const workspace = element.closest<HTMLElement>(".desk-workspace");
          if (workspace) {
            const navigation = document.querySelector(".desk-navigation")?.getBoundingClientRect();
            const bottom = Math.min(workspace.getBoundingClientRect().bottom - 24, matchMedia("(max-width: 800px)").matches && navigation ? navigation.top - 12 : innerHeight);
            let available = Math.max(Math.min(240, workspace.clientHeight / 2), bottom - element.getBoundingClientRect().top - workspace.scrollTop);
            const studio = element.closest<HTMLElement>(".plan-studio");
            const basis = studio?.querySelector<HTMLElement>(".plan-basis");
            const evidence = studio?.querySelector<HTMLElement>(".adjustment-evidence");
            if (studio && basis && evidence && matchMedia("(min-width: 1200px)").matches) {
              const overhead = element.getBoundingClientRect().top - studio.getBoundingClientRect().top;
              const gap = parseFloat(getComputedStyle(studio).rowGap) || 0;
              available = Math.max(available, basis.offsetHeight - overhead - evidence.offsetHeight - gap);
              for (const target of [basis, evidence]) if (!observed.has(target)) { observed.add(target); resize.observe(target); }
            }
            element.style.setProperty("--list-available-height", `${Math.round(available)}px`);
            for (const target of [workspace, element.parentElement!]) if (!observed.has(target)) { observed.add(target); resize.observe(target); }
          }
        }
        mark(element);
      });
    };
    const schedule = () => { cancelAnimationFrame(frame); frame = requestAnimationFrame(refresh); };
    const resize = new ResizeObserver(schedule);
    const onScroll = (event: Event) => {
      const element = event.target;
      if (element instanceof HTMLElement && element.matches(selector)) mark(element);
    };
    const mutations = new MutationObserver(schedule);
    refresh();
    document.addEventListener("scroll", onScroll, { capture: true, passive: true });
    window.addEventListener("resize", refresh);
    mutations.observe(document.body, { childList: true, subtree: true });
    return () => {
      cancelAnimationFrame(frame);
      document.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", refresh);
      mutations.disconnect();
      resize.disconnect();
    };
  }, []);
  return <MotionConfig reducedMotion="user" transition={tempo.spring}>{children}</MotionConfig>;
}

export function Presence({ children }: { children: ReactNode }) {
  return <AnimatePresence initial={false}>{children}</AnimatePresence>;
}

export function Disclosure({ children, id, open, className = "", mountOnOpen = false }: { children: ReactNode; id: string; open: boolean; className?: string; mountOnOpen?: boolean }) {
  const host = useRef<HTMLDivElement>(null);
  const [opened, setOpened] = useState(open);
  useEffect(() => { if (open) setOpened(true); }, [open]);
  useLayoutEffect(() => {
    if (!open && host.current?.contains(document.activeElement)) {
      [...document.querySelectorAll<HTMLElement>("[aria-controls]")].find(control => control.getAttribute("aria-controls") === id)?.focus({ preventScroll: true });
    }
  }, [id, open]);
  return <div ref={host} id={id} className="motion-disclosure" data-state={open ? "open" : "closed"} inert={!open} aria-hidden={!open}>
    <div className="motion-disclosure-clip"><div className={`motion-disclosure-content ${className}`}>{!mountOnOpen || open || opened ? children : null}</div></div>
  </div>;
}

export function SelectionGroup({ children }: { children: ReactNode }) {
  return <>{children}</>;
}

export function SelectionMarker({ identity }: { identity?: string }) {
  return identity ? <motion.span className="selection-marker" aria-hidden="true" layoutId={identity} initial={false} transition={tempo.spring} /> : <span className="selection-marker" aria-hidden="true" />;
}

function ChangeContents({ children, className, stagger }: { children: ReactNode; className: string; stagger: boolean }) {
  const reduced = useReducedMotion();
  const present = useIsPresent();
  return <motion.div className={`motion-change ${className}`} data-stagger={stagger || undefined} data-transition-state={present ? "present" : "exiting"} inert={!present}
    initial={{ opacity: reduced ? 1 : 0 }} animate={{ opacity: 1, transition: { duration: reduced ? 0 : tempo.change, ease: tempo.settle } }} exit={{ opacity: 0, transition: { duration: reduced ? 0 : tempo.exit + (stagger ? 3 * tempo.stagger : 0), ease: tempo.leave } }}>
    {children}
  </motion.div>;
}

function CompactChange({ children, identity, className }: { children: ReactNode; identity: string | number; className: string }) {
  const host = useRef<HTMLDivElement>(null);
  const previousIdentity = useRef(identity);
  const reduced = useReducedMotion();
  useLayoutEffect(() => {
    const changed = previousIdentity.current !== identity;
    previousIdentity.current = identity;
    if (!changed || reduced) return;
    const animation = host.current?.animate([{ opacity: .92 }, { opacity: 1 }], { duration: tempo.feedback * 1000, easing: "ease-out" });
    return () => animation?.cancel();
  }, [identity, reduced]);
  return <div ref={host} className={`motion-change ${className}`} data-transition-state="present">{children}</div>;
}

export function Change({ children, identity, className = "", stagger = false }: { children: ReactNode; identity: string | number; className?: string; stagger?: boolean }) {
  const compact = useCompactLayout();
  if (compact) return <CompactChange identity={identity} className={className}>{children}</CompactChange>;
  return (
    <AnimatePresence mode="wait" initial={false}>
      <ChangeContents key={identity} className={className} stagger={stagger}>{children}</ChangeContents>
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