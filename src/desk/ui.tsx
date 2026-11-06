import { useEffect, useLayoutEffect, useRef, useState, type ButtonHTMLAttributes, type ReactNode } from "react";
import { IconArrowLeft, IconX, IconAlertCircle, IconLoader2, IconCheck, IconChevronRight } from "@tabler/icons-react";
import { motion, useAnimationControls, useDragControls, useIsPresent } from "motion/react";
import { markActionOrigin, Presence, tempo, useSurfaceMotion } from "./interaction";
import { lockPageScroll } from "./overlays";

export function Button({ children, tone = "normal", busy, className = "", onClick, ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { tone?: "normal" | "primary" | "danger"; busy?: boolean }) {
  const button = useRef<HTMLButtonElement>(null);
  const [result, setResult] = useState<"success" | "error" | null>(null);
  useEffect(() => {
    const element = button.current!;
    let timer: ReturnType<typeof setTimeout>;
    const feedback = (event: Event) => {
      clearTimeout(timer);
      setResult((event as CustomEvent<{ ok: boolean }>).detail.ok ? "success" : "error");
      timer = setTimeout(() => setResult(null), tempo.resultHold);
    };
    element.addEventListener("desk:action-result", feedback);
    return () => { clearTimeout(timer); element.removeEventListener("desk:action-result", feedback); };
  }, []);
  useEffect(() => { if (busy) setResult(null); }, [busy]);
  return <button type="button" {...props} ref={button} onClick={event => { markActionOrigin(event.currentTarget); onClick?.(event); }} disabled={props.disabled || busy} className={`desk-button ${tone} ${className}`} aria-busy={busy || undefined} data-result={result ?? undefined}>
    <span className="button-content">{children}</span>
    <Presence>{busy ? <motion.span key="pending" className="button-progress" aria-hidden="true" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: tempo.feedback }}><IconLoader2 size={18} className="spinner" /></motion.span>
      : result && <motion.span key={result} className="button-result" aria-hidden="true" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: tempo.feedback }}>{result === "success" ? <IconCheck size={20} /> : <IconAlertCircle size={20} />}</motion.span>}</Presence>
  </button>;
}
export function IconButton({ label, children, ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  useEffect(() => {
    if (!hovered && !focused) return;
    const dismiss = (event: KeyboardEvent) => { if (event.key === "Escape") setDismissed(true); };
    document.addEventListener("keydown", dismiss);
    return () => document.removeEventListener("keydown", dismiss);
  }, [hovered, focused]);
  return <Button {...props} className={`icon-button ${props.className ?? ""}`} aria-label={label}
    onPointerEnter={event => { if (event.pointerType !== "touch") { setHovered(true); setDismissed(false); } props.onPointerEnter?.(event); }}
    onPointerLeave={event => { setHovered(false); setDismissed(false); props.onPointerLeave?.(event); }}
    onFocus={event => { setFocused(true); setDismissed(false); props.onFocus?.(event); }}
    onBlur={event => { setFocused(false); setDismissed(false); props.onBlur?.(event); }}>
    {children}<span className="control-tooltip" data-dismissed={dismissed || undefined} aria-hidden="true">{label}</span>
  </Button>;
}
export function ErrorNotice({ error, retry }: { error: string | null; retry?: () => void }) {
  return <Presence>{error && <motion.div key="error" className="error-notice" role="alert" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: tempo.feedback }}><IconAlertCircle size={20} aria-hidden="true" /><p>{error}</p>{retry && <Button onClick={retry}>Try again</Button>}</motion.div>}</Presence>;
}
export function Loading({ label = "Loading your training" }: { label?: string }) {
  return <div className="desk-loading" role="status"><IconLoader2 className="spinner" aria-hidden="true" /><span>{label}</span></div>;
}
export function Empty({ title, children, action }: { title: string; children?: ReactNode; action?: ReactNode }) {
  return <div className="desk-empty"><h2>{title}</h2>{children && <p>{children}</p>}{action}</div>;
}
export function Dialog({ title, children, onClose, wide = false }: { title: string; children: ReactNode; onClose: () => void; wide?: boolean }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const onCloseRef = useRef(onClose);
  const surface = useSurfaceMotion("dialog");
  const present = useIsPresent();
  const drag = useDragControls();
  onCloseRef.current = onClose;
  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null;
    const element = dialog.current!;
    const unlock = lockPageScroll();
    element.showModal();
    const initialFocus = element.querySelector<HTMLElement>('input:not([type="hidden"]):not(:disabled),select:not(:disabled),textarea:not(:disabled)') ?? element.querySelector<HTMLElement>(".dialog-surface > header h2");
    initialFocus?.focus({ preventScroll: true });
    const cancel = (event: Event) => { event.preventDefault(); onCloseRef.current(); };
    element.addEventListener("cancel", cancel);
    return () => { element.removeEventListener("cancel", cancel); element.close(); unlock(); const target = opener?.isConnected && !opener.closest('[inert]') ? opener : document.querySelector<HTMLElement>(".workspace-heading h1"); target?.focus({ preventScroll: true }); };
  }, []);
  return <motion.dialog ref={dialog} aria-label={title} className={`desk-dialog ${wide ? "wide" : ""}`} data-state={present ? "open" : "closing"}
    initial={surface.initial} animate={surface.animate} exit={surface.exit}
    drag={surface.compact && !surface.reduced ? "y" : false} dragListener={false} dragControls={drag} dragConstraints={{ top: 0, bottom: 0 }} dragElastic={{ top: .025, bottom: .8 }} dragTransition={{ bounceStiffness: 500, bounceDamping: 42 }}
    onDragEnd={(_, info) => { if (info.offset.y > 96 || info.offset.y > 24 && info.velocity.y > 650) onClose(); }}
    onClick={event => { if (present && event.target === dialog.current) onClose(); }}>
    <div className="dialog-grip" aria-hidden="true" onPointerDown={event => { if (surface.compact && !surface.reduced) drag.start(event); }}><span /></div>
    <div className="dialog-surface"><header><h2 tabIndex={-1}>{title}</h2><IconButton label={`Close ${title}`} onClick={onClose}><IconX size={20} aria-hidden="true" /></IconButton></header>{children}</div>
  </motion.dialog>;
}
export function InfoDialog({ title, children, wide = false, className = "" }: { title: string; children: ReactNode; wide?: boolean; className?: string }) {
  const [open, setOpen] = useState(false);
  return <div className={`dialog-action ${className}`}><Button aria-haspopup="dialog" onClick={() => setOpen(true)}><span>{title}</span><IconChevronRight size={18} aria-hidden="true" /></Button><Presence>{open && <Dialog title={title} wide={wide} onClose={() => setOpen(false)}>{children}</Dialog>}</Presence></div>;
}

export function Inspector({ title, children, onClose, modal = false, origin }: { title: string; children: ReactNode; onClose: () => void; modal?: boolean; origin?: HTMLElement | null }) {
  const host = useRef<HTMLElement>(null);
  const close = useRef(onClose);
  const surface = useSurfaceMotion("inspector");
  const drag = useDragControls();
  const present = useIsPresent();
  const morphControls = useAnimationControls();
  const [collapsed, setCollapsed] = useState<{ x: number; y: number; scaleX: number; scaleY: number; borderRadius: string } | null>(null);
  const morph = Boolean((modal || !surface.compact) && origin && !surface.reduced);
  useLayoutEffect(() => {
    if (!morph || !origin?.isConnected) return;
    const element = host.current!;
    const source = origin.getBoundingClientRect();
    const target = element.getBoundingClientRect();
    if (!source.width || !source.height || !target.width || !target.height) return;
    const shape = {
      x: source.x + source.width / 2 - target.x - target.width / 2,
      y: source.y + source.height / 2 - target.y - target.height / 2,
      scaleX: source.width / target.width,
      scaleY: source.height / target.height,
      borderRadius: getComputedStyle(origin).borderRadius,
    };
    const visibility = origin.style.visibility;
    const radius = getComputedStyle(element).borderRadius;
    setCollapsed(shape);
    if (modal) origin.style.visibility = "hidden";
    morphControls.set({ ...shape, opacity: 1 });
    void morphControls.start({ x: 0, y: 0, scaleX: 1, scaleY: 1, borderRadius: radius, opacity: 1, transition: tempo.spring });
    return () => { origin.style.visibility = visibility; };
  }, [morph, origin, morphControls, modal]);
  close.current = onClose;
  useEffect(() => {
    const opener = origin ?? document.activeElement as HTMLElement | null;
    const element = host.current!;
    const mobile = matchMedia("(max-width: 800px)");
    const inertElements = new Map<HTMLElement, boolean>();
    let unlock: (() => void) | undefined;
    const restore = () => { inertElements.forEach((inert, node) => { node.inert = inert; }); inertElements.clear(); unlock?.(); unlock = undefined; };
    const update = () => {
      restore();
      if (!modal && !mobile.matches) return;
      for (let ancestor: HTMLElement | null = element; ancestor?.parentElement; ancestor = ancestor.parentElement) {
        for (const sibling of ancestor.parentElement.children) {
          if (sibling instanceof HTMLElement && sibling !== ancestor && !sibling.matches(".notification-viewport, .inspector-backdrop")) { inertElements.set(sibling, sibling.inert); sibling.inert = true; }
        }
      }
      unlock = lockPageScroll();
    };
    const keys = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); close.current(); }
      if (event.key !== "Tab" || (!modal && !mobile.matches)) return;
      const focusable = [...element.querySelectorAll<HTMLElement>('button:not(:disabled),a[href],input:not(:disabled),select:not(:disabled),textarea:not(:disabled),summary,[tabindex="0"]')].filter(node => node.getClientRects().length);
      const first = focusable[0], last = focusable[focusable.length - 1];
      if (event.shiftKey && (document.activeElement === first || document.activeElement === element)) { event.preventDefault(); last?.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    };
    update(); mobile.addEventListener("change", update); element.addEventListener("keydown", keys); element.focus({ preventScroll: true });
    return () => { restore(); mobile.removeEventListener("change", update); element.removeEventListener("keydown", keys); if (opener?.isConnected) opener.focus({ preventScroll: true }); };
  }, [title, modal, origin]);
  return <>{modal && <div className="inspector-backdrop" aria-hidden="true" data-state={present ? "open" : "closing"} onClick={() => { if (present) onClose(); }} />}
  <motion.aside className="desk-inspector" role={modal ? "dialog" : undefined} aria-modal={modal || undefined} aria-label={title} ref={host} tabIndex={-1} data-state={present ? "open" : "closing"}
    initial={morph ? false : surface.initial} animate={morph ? morphControls : surface.animate} exit={morph && collapsed ? { ...collapsed, opacity: 1, transition: { duration: tempo.exit, ease: tempo.leave } } : surface.exit}
    drag={surface.compact && !surface.reduced ? "x" : false} dragListener={false} dragControls={drag} dragConstraints={{ left: 0, right: 0 }} dragElastic={{ left: .025, right: .8 }} dragTransition={{ bounceStiffness: 500, bounceDamping: 42 }}
    onDragEnd={(_, info) => { if (info.offset.x > Math.min(120, innerWidth * .28) || info.offset.x > 24 && info.velocity.x > 650) onClose(); }}>
    <motion.div initial={morph ? { opacity: 0 } : false} animate={{ opacity: 1 }} exit={morph ? { opacity: 0 } : undefined} transition={{ duration: tempo.feedback, delay: present && morph ? tempo.press : 0 }}>
    <div className={`inspector-bar${modal ? " modal-bar" : ""}`} onPointerDown={event => { if (surface.compact && !surface.reduced && !(event.target as HTMLElement).closest("button")) drag.start(event); }}>{modal ? <><span>{title}</span><IconButton label={`Close ${title}`} onClick={onClose}><IconX size={20} aria-hidden="true" /></IconButton></> : <><IconButton label="Back" onClick={onClose}><IconArrowLeft size={20} aria-hidden="true" /></IconButton><span>{title}</span></>}</div>{children}
    </motion.div>
  </motion.aside></>;
}
export function Metric({ label, value, unit }: { label: string; value: ReactNode; unit?: string }) {
  return <div className="desk-metric"><dt>{label}</dt><dd>{value}{unit && <small>{unit}</small>}</dd></div>;
}