import { t } from "./i18n";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { motion, useReducedMotion } from "motion/react";
import { IconX } from "@tabler/icons-react";
import { tempo } from "./interaction";
import { fadeOut, slideUp } from "./motion";

const scrollLocks = new Set<symbol>();
let originalOverflow = "";
let workspaceScroll: { element: HTMLElement; overflow: string } | undefined;
export function lockPageScroll() {
  const lock = Symbol();
  if (!scrollLocks.size) {
    originalOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const element = document.getElementById("workspace");
    if (element) {
      workspaceScroll = { element, overflow: element.style.overflow };
      element.style.overflow = "hidden";
    }
  }
  scrollLocks.add(lock);
  return () => {
    scrollLocks.delete(lock);
    if (!scrollLocks.size) {
      document.body.style.overflow = originalOverflow;
      if (workspaceScroll) workspaceScroll.element.style.overflow = workspaceScroll.overflow;
      workspaceScroll = undefined;
    }
  };
}

export function NotificationViewport({ children }: { children: ReactNode }) {
  const [target, setTarget] = useState<HTMLElement>(document.body);
  const viewport = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const update = () => {
      const dialogs = [...document.querySelectorAll<HTMLDialogElement>("dialog[open]")];
      setTarget(dialogs[dialogs.length - 1] ?? document.body);
    };
    const observer = new MutationObserver(update);
    observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["open"] });
    update();
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    const element = viewport.current;
    if (!element?.showPopover) return;
    let previousFocus: HTMLElement | null = null;
    const update = () => {
      const open = element.matches(":popover-open");
      if (element.childElementCount && !open) {
        previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        element.showPopover();
      } else if (!element.childElementCount && open) {
        element.hidePopover();
        if (target instanceof HTMLDialogElement && target.open) {
          const focus = previousFocus?.isConnected && target.contains(previousFocus) ? previousFocus : target.querySelector<HTMLElement>(".dialog-surface > header h2");
          focus?.focus({ preventScroll: true });
        }
      }
    };
    const observer = new MutationObserver(update);
    observer.observe(element, { childList: true });
    update();
    return () => observer.disconnect();
  }, [target]);
  return createPortal(<div ref={viewport} className="notification-viewport" popover={"showPopover" in HTMLElement.prototype ? "manual" : undefined} aria-label={t("Notifications")} onClick={event => event.stopPropagation()} onPointerDown={event => event.stopPropagation()}>{children}</div>, target);
}

export function Notification({ children, label, onDismiss, dismissDisabled, countdown = true, countdownKey, elementRef, actions }: { children: ReactNode; label?: string; onDismiss: () => void; dismissDisabled?: boolean; countdown?: boolean; countdownKey?: string; elementRef?: React.Ref<HTMLDivElement>; actions?: ReactNode }) {
  const reduced = useReducedMotion();
  return <motion.div ref={elementRef} layout={reduced ? false : "position"} className={`desk-notice${label ? " desk-stages" : ""}`} role="status" aria-label={label} aria-atomic="true"
    initial={reduced ? { opacity: 0 } : slideUp} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ ...fadeOut, y: reduced ? 0 : tempo.exitDistance, transition: reduced ? { duration: 0 } : fadeOut.transition }} transition={reduced ? { duration: 0 } : tempo.spring}>
    <div className="notice-content">{children}</div>{actions}
    <button type="button" className="desk-button icon-button notice-close" aria-label={t("Dismiss notification")} disabled={dismissDisabled} onClick={onDismiss}><IconX size={18} aria-hidden="true" />{countdown && <svg key={countdownKey} className="stage-ring" viewBox="0 0 44 44" aria-hidden="true"><circle cx="22" cy="22" r="19" /></svg>}</button>
  </motion.div>;
}