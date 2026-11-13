import { useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { animate } from "motion/mini";
import { BrandMark } from "./ui";
import { tempo } from "./motion";

export function BrandSplash() {
  const [visible, setVisible] = useState(() => !matchMedia("(prefers-reduced-motion: reduce)").matches);
  const host = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const element = host.current;
    if (!element) return;
    const root = document.documentElement;
    const preference = matchMedia("(prefers-reduced-motion: reduce)");
    const animations: ReturnType<typeof animate>[] = [];
    const icon = element.querySelector<HTMLElement>(".desk-brand-icon")!;
    const wordmark = element.querySelector<HTMLElement>(".brand-splash-wordmark")!;
    const curtain = element.querySelector<HTMLElement>(".brand-splash-curtain")!;
    const backdrop = element.querySelector<HTMLElement>(".brand-splash-backdrop")!;
    let stopped = false;
    let started = false;
    let arrived = false;
    let deadline: ReturnType<typeof setTimeout> | undefined;
    let destination: HTMLElement | null = null;
    const targetIcon = () => document.querySelector<HTMLElement>(".desk-appbar .desk-brand-icon, .onboarding-header .desk-brand-icon");
    const cleanup = () => {
      stopped = true;
      element.hidden = true;
      clearTimeout(deadline);
      observer.disconnect();
      for (const animation of animations) animation.cancel();
      delete root.dataset.brandLaunch;
      preference.removeEventListener("change", finish);
      window.removeEventListener("resize", finish);
      window.removeEventListener("hashchange", finish);
      window.removeEventListener("pagehide", finish);
      document.removeEventListener("visibilitychange", visibilityChanged);
      document.removeEventListener("keydown", finish, true);
      document.removeEventListener("pointerdown", finish, true);
    };
    const finish = () => {
      if (stopped) return;
      const header = targetIcon()?.closest<HTMLElement>(".desk-brand, .onboarding-header");
      header?.dispatchEvent(new Event("desk:reveal-settle", { bubbles: true }));
      cleanup();
      setVisible(false);
    };
    const play = (target: HTMLElement, keyframes: Parameters<typeof animate>[1], options: Parameters<typeof animate>[2] = {}) => {
      const animation = animate(target, keyframes, { duration: tempo.landing, ease: tempo.soft, ...options });
      animations.push(animation);
      return animation;
    };
    const dock = async () => {
      const target = targetIcon();
      if (stopped || !arrived || destination || !target) return;
      destination = target;
      const header = target.closest<HTMLElement>(".desk-brand, .onboarding-header")!;
      header.dispatchEvent(new Event("desk:reveal-settle", { bubbles: true }));
      const targetWordmark = header.querySelector<HTMLElement>(".brand-wordmark");
      const destinationIcon = target.getBoundingClientRect();
      const destinationText = targetWordmark?.getBoundingClientRect();
      const sourceIcon = icon.getBoundingClientRect();
      const sourceText = wordmark.getBoundingClientRect();
      const hasText = Boolean(destinationText?.width);
      const textLeft = hasText ? destinationText!.left : destinationIcon.left + destinationIcon.width / 2;
      const textTop = hasText ? destinationText!.top : destinationIcon.top + destinationIcon.height / 2;
      const textScaleX = hasText ? destinationText!.width / sourceText.width : .08;
      const textScaleY = hasText ? destinationText!.height / sourceText.height : .3;
      element.dataset.phase = "docking";
      await Promise.all([
        play(icon, { transform: ["translate3d(0px, 0px, 0) scale(1, 1)", `translate3d(${destinationIcon.left - sourceIcon.left}px, ${destinationIcon.top - sourceIcon.top}px, 0) scale(${destinationIcon.width / sourceIcon.width}, ${destinationIcon.height / sourceIcon.height})`] }),
        play(wordmark, { transform: ["translate3d(0px, 0px, 0) scale(1, 1)", `translate3d(${textLeft - sourceText.left}px, ${textTop - sourceText.top}px, 0) scale(${textScaleX}, ${textScaleY})`], opacity: [1, hasText ? 1 : 0] }),
        play(curtain, { opacity: [1, 0] }),
      ]);
      if (stopped) return;
      element.dataset.phase = "revealing";
      await play(backdrop, { opacity: [1, 0] });
      finish();
    };
    const observer = new MutationObserver(() => {
      if (destination && targetIcon() !== destination) finish();
      else void dock();
    });
    root.dataset.brandLaunch = "opening";
    observer.observe(document.getElementById("desk-root")!, { childList: true, subtree: true });
    preference.addEventListener("change", finish);
    window.addEventListener("resize", finish);
    window.addEventListener("hashchange", finish);
    window.addEventListener("pagehide", finish);
    document.addEventListener("visibilitychange", visibilityChanged);
    document.addEventListener("keydown", finish, true);
    document.addEventListener("pointerdown", finish, true);
    function visibilityChanged() {
      if (document.hidden) {
        if (started) finish();
      } else {
        start();
      }
    }
    function start() {
      if (stopped || started || document.hidden) return;
      started = true;
      deadline = setTimeout(finish, (tempo.landing * 4 + tempo.stagger * 12) * 1000);
      void document.fonts.ready.then(async () => {
      if (stopped) return;
      await Promise.all([
        play(icon, { opacity: [0, 1], transform: ["translate3d(0px, 10px, 0) scale(.82) rotate(-8deg)", "translate3d(0px, 0px, 0) scale(1) rotate(0deg)"] }),
        ...[...wordmark.children].map((letter, index) => play(letter as HTMLElement, { opacity: [0, 1], transform: ["translate3d(-.3em, .35em, 0) skewX(-12deg)", "translate3d(0px, 0px, 0) skewX(0deg)"] }, { delay: index * tempo.stagger })),
      ]);
      if (stopped) return;
      arrived = true;
      void dock();
      });
    }
    start();
    return cleanup;
  }, []);
  if (!visible) return null;
  return createPortal(<div ref={host} className="brand-splash" data-phase="intro" data-entrance="none" aria-hidden="true">
    <div className="brand-splash-backdrop" />
    <div className="brand-splash-curtain" />
    <div className="brand-splash-lockup"><BrandMark /><span className="brand-splash-wordmark">{[..."Runalytics"].map((letter, index) => <span key={index}>{letter}</span>)}</span></div>
  </div>, document.body);
}