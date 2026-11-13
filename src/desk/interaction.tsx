import { useEffect, useId, useLayoutEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import { AnimatePresence, LayoutGroup, MotionConfig, motion, useIsPresent, useReducedMotion, type AnimationPlaybackControls } from "motion/react";
import { animate as animateNative } from "motion/mini";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { entranceFor, modalTransition, motionOrder, motionVisible, tempo } from "./motion";
import "./tokens.css";

export { tempo } from "./motion";

let initiatingControl: HTMLButtonElement | null = null;
export function markActionOrigin(control: HTMLButtonElement) {
  initiatingControl = control;
  setTimeout(() => { if (initiatingControl === control) initiatingControl = null; }, 0);
}
export function actionOrigin() {
  const active = document.activeElement;
  return initiatingControl ?? (active instanceof HTMLButtonElement ? active : active?.closest("form")?.querySelector<HTMLButtonElement>('button[type="submit"]') ?? null);
}

export function MotionSystem({ children }: { children: ReactNode }) {
  useEffect(() => {
    const selector = ".desk-toolbar, .route-controls, .choice-row, .pace-mode, .setup-band, .proposal-prompt, .desk-notice";
    const preferences = ["(prefers-reduced-motion: reduce)", "(prefers-reduced-transparency: reduce)", "(prefers-contrast: more)", "(forced-colors: active)"].map(query => matchMedia(query));
    const surfaces = new Map<HTMLElement, { animation: Animation | null; visible: boolean }>();
    const enabled = () => !preferences.some(preference => preference.matches);
    const position = () => `${15 + Math.random() * 70}% ${20 + Math.random() * 60}%, ${15 + Math.random() * 70}% ${20 + Math.random() * 60}%, 50% 50%`;
    const drift = (element: HTMLElement, state: { animation: Animation | null; visible: boolean }, from = "35% 45%, 65% 55%, 50% 50%") => {
      const frames = [from, ...Array.from({ length: 5 }, position)];
      const animation = element.animate(frames.map(backgroundPosition => ({ backgroundPosition, easing: "ease-in-out" })), {
        id: "material-aura", duration: 18000 + Math.random() * 12000, fill: "forwards",
      });
      state.animation = animation;
      animation.onfinish = () => {
        if (surfaces.get(element) !== state || !enabled()) return;
        drift(element, state, frames[frames.length - 1]);
        animation.cancel();
      };
      if (!state.visible || document.hidden) animation.pause();
    };
    const sync = () => {
      for (const [element, state] of surfaces) {
        if (!enabled()) { state.animation?.cancel(); state.animation = null; }
        else if (!state.animation && state.visible && !document.hidden) drift(element, state);
        else if (state.visible && !document.hidden) state.animation?.play();
        else state.animation?.pause();
      }
    };
    const observer = new IntersectionObserver(entries => {
      for (const entry of entries) {
        const state = surfaces.get(entry.target as HTMLElement);
        if (state) state.visible = entry.isIntersecting;
      }
      sync();
    });
    const register = (root: ParentNode) => {
      const elements = [...root.querySelectorAll<HTMLElement>(selector)];
      if (root instanceof HTMLElement && root.matches(selector)) elements.unshift(root);
      for (const element of elements) {
        if (surfaces.has(element)) continue;
        surfaces.set(element, { animation: null, visible: false });
        observer.observe(element);
      }
    };
    const mutations = new MutationObserver(records => {
      for (const [element, state] of surfaces) {
        if (element.isConnected) continue;
        state.animation?.cancel();
        observer.unobserve(element);
        surfaces.delete(element);
      }
      for (const record of records) for (const node of record.addedNodes) if (node instanceof HTMLElement) register(node);
    });
    register(document);
    mutations.observe(document.body, { childList: true, subtree: true });
    for (const preference of preferences) preference.addEventListener("change", sync);
    document.addEventListener("visibilitychange", sync);
    return () => {
      mutations.disconnect();
      observer.disconnect();
      for (const state of surfaces.values()) state.animation?.cancel();
      for (const preference of preferences) preference.removeEventListener("change", sync);
      document.removeEventListener("visibilitychange", sync);
    };
  }, []);
  useEffect(() => {
    const selector = 'button, a[href], summary, [role="button"], .choice-row label, .palette-options label';
    const coarse = matchMedia("(pointer: coarse)");
    let touched: Element | null = null;
    const clear = () => { touched = null; };
    const start = (event: PointerEvent) => {
      touched = event.pointerType === "touch" && event.target instanceof Element ? event.target.closest(selector) : null;
    };
    const click = (event: MouseEvent) => {
      const control = event.target instanceof Element ? event.target.closest(selector) : null;
      if (!coarse.matches || !event.isTrusted || event.detail === 0 || !control || control !== touched) return;
      clear();
      if (control.closest('[inert], [aria-disabled="true"]') || control.matches(':disabled, :has(:disabled), .is-dragging')) return;
      if (isTauri()) {
        void invoke("plugin:haptics|impact_feedback", { style: "light" }).catch(() => {});
      } else {
        try { navigator.vibrate?.(8); } catch {}
      }
    };
    document.addEventListener("pointerdown", start, true);
    document.addEventListener("pointercancel", clear, true);
    document.addEventListener("click", click, true);
    window.addEventListener("blur", clear);
    return () => {
      document.removeEventListener("pointerdown", start, true);
      document.removeEventListener("pointercancel", clear, true);
      document.removeEventListener("click", click, true);
      window.removeEventListener("blur", clear);
    };
  }, []);
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
      ], { duration: tempo.surface * 1000, easing: `cubic-bezier(${tempo.soft.join(",")})` });
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
  useLayoutEffect(() => {
    const selector = "[data-entrance], h1, h2, h3, h4, .section-heading, .desk-metric, .definition-list > div, .data-table-scroll, .desk-navigation > a, .desk-utilities > button, .workspace-heading, .training-context, .desk-toolbar, .health-verdict, .health-context > *, .health-history > *, .trend-chart, .session-tile, .activity-row, .roadmap-row, .settings-task-list > a, .settings-editor > header, .settings-editor > p, fieldset:not(.onboarding-options), .connection-summary, .form-grid, .form-actions, .consistency-week, .milestone-list > article, .desk-empty, .review-section, .review-heading, .plan-basis, .adjustment-evidence, .route-figure, .setup-band, .prescribed-session, .month-labels, .month-day, .day-lane > header, .mobile-date-ribbon, .pain-episode > header, .episode-notes, .episode-actions, .episode-chat > form, .chat-message, .proposal-prompt, .pace-next, .pace-insights-figure, .pace-session-detail, .race-estimates, .dialog-action > .desk-button, .journal-tools, .session-management, .session-recording, .watch-predictions, .composer-summary, .dialog-surface > header, .review-tabs, .pace-mode, .pace-session-picker, .schedule-summary, .onboarding-header, .onboarding-progress, .onboarding-step > label, .onboarding-step > p, .onboarding-options > legend:not(.sr-only), .onboarding-options > label, .onboarding-step summary, .onboarding-preview > li";
    const pending = new Set<HTMLElement>();
    const revealed = new WeakSet<HTMLElement>();
    const running = new Map<HTMLElement, { animation: AnimationPlaybackControls; opacity: string; transform?: string }>();
    const order = new WeakMap<HTMLElement, number>();
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    const compact = matchMedia("(max-width: 800px)");
    const started = performance.now();
    let unlocking = true;
    let sequence = 0;
    let landingFrame = 0;
    let revealFrame = 0;
    const landingRoots = new Set<HTMLElement>();
    const settleElement = (element: HTMLElement) => {
      const current = running.get(element);
      if (current) {
        running.delete(element);
        current.animation.cancel();
        element.style.opacity = current.opacity;
        if (current.transform !== undefined) element.style.transform = current.transform;
      }
      observer.unobserve(element);
      pending.delete(element);
      revealed.add(element);
      element.dataset.reveal = "settled";
      element.dispatchEvent(new CustomEvent("desk:reveal", { bubbles: true, detail: { delay: 0 } }));
    };
    const revealVisible = () => {
      revealFrame = 0;
      const visible = motionOrder([...pending].filter(element => motionVisible(element)));
      for (const [index, element] of visible.entries()) {
        const unlock = unlocking && performance.now() - started < tempo.landing * 1000;
        if (reduced.matches || element.contains(document.activeElement) && !document.activeElement?.matches("h1, h2, h3, h4")) { settleElement(element); continue; }
        observer.unobserve(element);
        pending.delete(element);
        revealed.add(element);
        const preset = entranceFor(`${element.className}:${order.get(element)}`, Math.max(0, index), unlock);
        const delay = preset.delay;
        const opacity = element.style.opacity;
        const transform = element.matches("[data-motion-layout]") ? undefined : element.style.transform;
        element.dataset.reveal = "visible";
        element.dataset.entranceKind = unlock ? "unlock" : "component";
        element.style.setProperty("--reveal-delay", `${delay}s`);
        const animation = animateNative(element, {
          opacity: [preset.initial.opacity, 1],
          ...(element.matches("[data-motion-layout]") ? {} : { transform: [`translate3d(0px, ${preset.initial.y}px, 0)`, "translate3d(0px, 0px, 0)"] }),
        }, { duration: tempo.landing, ease: tempo.soft, delay });
        running.set(element, { animation, opacity, transform });
        element.dispatchEvent(new CustomEvent("desk:reveal", { bubbles: true, detail: { delay } }));
        void animation.then(() => { if (running.get(element)?.animation === animation) settleElement(element); });
      }
    };
    const scheduleReveals = () => { if (!revealFrame) revealFrame = requestAnimationFrame(revealVisible); };
    const observer = new IntersectionObserver(scheduleReveals, { threshold: [0, tempo.viewportThreshold], rootMargin: `0px 0px -${tempo.viewportInset}px 0px` });
    const register = (root: ParentNode, replay = false) => {
      const elements = [...root.querySelectorAll<HTMLElement>(selector)];
      if (root instanceof HTMLElement && root.matches(selector)) elements.unshift(root);
      for (const element of elements) {
        if (element.closest('[inert], [data-transition-state="exiting"], [data-step-state="exiting"]') || !element.checkVisibility()) continue;
        if (element.closest('.sr-only, [data-entrance="none"]')) continue;
        if (element.querySelector(".desk-inspector, .desk-dialog, .notification-viewport")) continue;
        const ancestor = element.parentElement?.closest<HTMLElement>('[data-reveal]:not([data-reveal="container"])');
        if (ancestor && (elements.includes(ancestor) || pending.has(ancestor) || running.has(ancestor))) continue;
        if (replay) { settleElement(element); revealed.delete(element); }
        if (revealed.has(element) || pending.has(element)) continue;
        if (ancestor) ancestor.dataset.reveal = "container";
        for (const descendant of element.querySelectorAll<HTMLElement>("[data-reveal]")) {
          settleElement(descendant);
          revealed.delete(descendant);
          delete descendant.dataset.reveal;
        }
        order.set(element, sequence++);
        if (reduced.matches || element.contains(document.activeElement) && !document.activeElement?.matches("h1, h2, h3, h4")) { settleElement(element); continue; }
        element.dataset.reveal = "waiting";
        pending.add(element);
        observer.observe(element);
      }
      scheduleReveals();
    };
    const mutations = new MutationObserver(records => {
      for (const element of pending) {
        if (!element.isConnected) { observer.unobserve(element); pending.delete(element); }
      }
      for (const element of running.keys()) if (!element.isConnected) settleElement(element);
      for (const record of records) {
        for (const node of record.addedNodes) if (node instanceof HTMLElement) register(node);
        if (record.type === "attributes" && record.target instanceof HTMLElement) register(record.target);
      }
    });
    register(document);
    mutations.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["open", "hidden", "inert", "data-state"] });
    const toggle = (event: Event) => { if (event.target instanceof HTMLElement) register(event.target); };
    document.addEventListener("toggle", toggle, true);
    document.addEventListener("scroll", scheduleReveals, { capture: true, passive: true });
    window.addEventListener("resize", scheduleReveals);
    const landView = (event: Event) => {
      if (!(event.target instanceof HTMLElement) || reduced.matches) return;
      landingRoots.add(event.target);
      cancelAnimationFrame(landingFrame);
      cancelAnimationFrame(revealFrame);
      landingFrame = requestAnimationFrame(() => {
        const roots = [...landingRoots].filter(root => root.isConnected && ![...landingRoots].some(other => other !== root && other.contains(root)));
        landingRoots.clear();
        for (const root of roots) register(root, true);
      });
    };
    document.addEventListener("desk:view-arrive", landView);
    const settleReveals = (event: Event) => {
      if (!(event.target instanceof HTMLElement)) return;
      if (event.target.matches(selector)) settleElement(event.target);
      event.target.querySelectorAll<HTMLElement>(selector).forEach(settleElement);
    };
    document.addEventListener("desk:reveal-settle", settleReveals);
    const settleInteraction = (event: Event) => {
      if (!(event.target instanceof Element)) return;
      if (event.type === "focusin" && event.target.matches("h1, h2, h3, h4")) return;
      for (let element = event.target.closest<HTMLElement>('[data-reveal="waiting"], [data-reveal="visible"]'); element; element = element.parentElement?.closest<HTMLElement>('[data-reveal="waiting"], [data-reveal="visible"]') ?? null) settleElement(element);
    };
    document.addEventListener("focusin", settleInteraction);
    document.addEventListener("pointerdown", settleInteraction, true);
    const endUnlock = () => {
      unlocking = false;
      for (const element of running.keys()) settleElement(element);
    };
    window.addEventListener("hashchange", endUnlock);
    const settleMotion = () => {
      if (reduced.matches) document.querySelectorAll<HTMLElement>('[data-reveal="waiting"], [data-reveal="visible"]').forEach(settleElement);
      else { for (const element of running.keys()) settleElement(element); scheduleReveals(); }
    };
    reduced.addEventListener("change", settleMotion);
    compact.addEventListener("change", settleMotion);
    return () => {
      cancelAnimationFrame(landingFrame);
      landingRoots.clear();
      document.querySelectorAll<HTMLElement>('[data-reveal="waiting"], [data-reveal="visible"]').forEach(settleElement);
      observer.disconnect(); mutations.disconnect(); reduced.removeEventListener("change", settleMotion);
      compact.removeEventListener("change", settleMotion);
      window.removeEventListener("hashchange", endUnlock);
      document.removeEventListener("desk:view-arrive", landView);
      document.removeEventListener("desk:reveal-settle", settleReveals);
      document.removeEventListener("toggle", toggle, true);
      document.removeEventListener("scroll", scheduleReveals, true);
      window.removeEventListener("resize", scheduleReveals);
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
  const reduced = useReducedMotion();
  const tail = useCascadeExit(host, open, !reduced);
  const [opened, setOpened] = useState(open);
  useEffect(() => { if (open) setOpened(true); }, [open]);
  useLayoutEffect(() => {
    if (!open && host.current?.contains(document.activeElement)) {
      [...document.querySelectorAll<HTMLElement>("[aria-controls]")].find(control => control.getAttribute("aria-controls") === id)?.focus({ preventScroll: true });
    }
  }, [id, open, children]);
  return <div ref={host} id={id} className="motion-disclosure" style={{ "--disclosure-tail": `${tail}s` } as React.CSSProperties} data-state={open ? "open" : "closed"} inert={!open} aria-hidden={!open}>
    <div className="motion-disclosure-clip"><div className={`motion-disclosure-content ${className}`}>{!mountOnOpen || open || opened ? children : null}</div></div>
  </div>;
}

export function SelectionGroup({ children }: { children: ReactNode }) {
  return <LayoutGroup id={useId()}>{children}</LayoutGroup>;
}

export function SelectionMarker({ identity = "selection" }: { identity?: string }) {
  const reduced = useReducedMotion();
  return reduced ? <span className="selection-marker" aria-hidden="true" /> : <motion.span className="selection-marker" aria-hidden="true" layoutId={identity} initial={false} transition={tempo.spring} />;
}

function visibleReveals(root: HTMLElement | null) {
  return motionOrder([...(root?.querySelectorAll<HTMLElement>('[data-reveal="settled"], [data-reveal="visible"]') ?? [])].filter(element => motionVisible(element, true)));
}

export function useCascadeExit(host: RefObject<HTMLElement | null>, present: boolean, enabled: boolean) {
  useLayoutEffect(() => {
    const root = host.current;
    if (present || !enabled || !root || root.parentElement?.closest('[data-transition-state="exiting"], [data-state="closing"]')) return;
    const animations = visibleReveals(root).map((element, index) => {
      const transform = element.style.transform;
      const opacity = element.style.opacity;
      const animation = animateNative(element, { opacity: 0, ...(element.matches("[data-motion-layout]") ? {} : { transform: `translate3d(0px, ${tempo.exitDistance}px, 0)` }) }, { duration: tempo.exit, delay: index * tempo.stagger, ease: tempo.soft });
      return () => { animation.cancel(); element.style.transform = transform; element.style.opacity = opacity; };
    });
    return () => animations.forEach(stop => stop());
  }, [host, present, enabled]);
  return enabled ? Math.max(0, visibleReveals(host.current).length - 1) * tempo.stagger : 0;
}

function ChangeContents({ children, className, stagger, direction, compact, identity }: { children: ReactNode; className: string; stagger: boolean; direction: number; compact: boolean; identity: string | number }) {
  const reduced = useReducedMotion();
  const present = useIsPresent();
  const host = useRef<HTMLDivElement>(null);
  const tail = useCascadeExit(host, present, !reduced && !compact);
  const previousIdentity = useRef(identity);
  useLayoutEffect(() => {
    const changed = previousIdentity.current !== identity;
    previousIdentity.current = identity;
    const element = host.current;
    if (!changed || !compact || reduced || !element) return;
    element.dispatchEvent(new Event("desk:view-arrive", { bubbles: true }));
  }, [identity, compact, reduced]);
  return <motion.div ref={host} className={`motion-change ${className}`} data-motion-direction={direction} data-stagger={stagger || undefined} data-transition-state={present ? "present" : "exiting"} inert={!present}
    initial={false} animate={{ opacity: 1 }} exit={{ opacity: reduced || compact ? 0 : 1, transition: { duration: reduced || compact ? 0 : tempo.exit + tail } }}>
    {children}
  </motion.div>;
}

export function Change({ children, identity, className = "", stagger = false, direction = 1 }: { children: ReactNode; identity: string | number; className?: string; stagger?: boolean; direction?: number }) {
  const compact = useCompactLayout();
  const immediate = compact || isTauri();
  const [transition, setTransition] = useState({ identity, key: identity });
  if (transition.identity !== identity) setTransition({ identity, key: immediate ? transition.key : identity });
  return (
    <AnimatePresence mode={immediate ? "sync" : "wait"} initial={false} custom={direction}>
      <ChangeContents key={transition.key} identity={identity} compact={immediate} className={className} stagger={stagger} direction={direction}>{children}</ChangeContents>
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

export function useSurfaceMotion(_kind: "dialog" | "inspector", host: RefObject<HTMLElement | null>) {
  const compact = useCompactLayout();
  const reduced = useReducedMotion();
  const present = useIsPresent();
  const tail = useCascadeExit(host, present, !reduced);
  const offset = reduced ? {} : { x: 0, y: tempo.distance, scale: 1 };
  return {
    compact,
    reduced,
    initial: { opacity: reduced ? 1 : 0, ...offset },
    animate: { ...modalTransition.animate, transition: reduced ? { duration: 0 } : tempo.spring },
    exit: { opacity: 0, ...(reduced ? {} : { x: 0, y: tempo.exitDistance, scale: 1 }), transition: { duration: reduced ? 0 : tempo.exit, delay: tail, ease: tempo.soft } },
  };
}