import { Component, useId, useLayoutEffect, useRef, useState, type CSSProperties, type MouseEvent, type ReactElement, type ReactNode, type RefObject, type SVGProps } from "react";
import UseAnimations, { type Animation, type AnimationProps, type EventProps } from "react-useanimations";
import { useIsPresent } from "motion/react";
import { iconControl, tempo, useIconPlaybackEligibility, type IconInteraction } from "./interaction";
import { motionVisible } from "./motion";
import { catalogIcons } from "./icon-catalog";
import { CustomGlyph, customMotions, type CustomIconName } from "./custom-icons";
import posters from "./icon-posters.json";

type CatalogIconName = keyof typeof catalogIcons;
type Status = "pending" | "success" | "error";
export type IconName = CatalogIconName | CustomIconName;
export type AnimatedIconName = Exclude<IconName, Status>;
type Activation = (event: Event | MouseEvent<HTMLElement>) => void;

function isCatalog(name: IconName): name is CatalogIconName { return name in catalogIcons; }

function StaticGlyph({ name, size, state }: { name: IconName; size: number; state?: boolean }) {
  const identity = useId().replace(/\W/g, "");
  if (!isCatalog(name)) return <CustomGlyph name={name} size={size} />;
  const key = catalogIcons[name].poster + (state && ["checkbox", "radio", "toggle"].includes(name) ? "On" : "");
  const poster = posters[key as keyof typeof posters];
  return <svg {...poster.attributes as SVGProps<SVGSVGElement>} width={size} height={size} aria-hidden="true" focusable="false" data-icon-poster="useanimations" dangerouslySetInnerHTML={{ __html: poster.body.split("ua-poster-").join(`${identity}-`) }} />;
}

class PlayerBoundary extends Component<{ children: ReactNode; fallback: ReactNode; onFailure: () => void }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  componentDidCatch() { this.props.onFailure(); }
  render() { return this.state.failed ? this.props.fallback : this.props.children; }
}

function PlayerFrame({ events, attributes, activation }: { events: EventProps; attributes: AnimationProps; activation: RefObject<Activation | null> }) {
  useLayoutEffect(() => {
    const activate: Activation = event => events.onClick(event as MouseEvent<HTMLElement>);
    activation.current = activate;
    return () => { if (activation.current === activate) activation.current = null; };
  }, [activation, events]);
  return <span {...attributes} />;
}

function speedFor(animation: Animation, target: number) {
  const data = animation.animationData as { ip: number; op: number; fr: number };
  const duration = (data.op - data.ip) / data.fr;
  return Number.isFinite(duration) && target > 0 ? Math.max(1, duration / target) : 1;
}

function NativePlayer({ name, size, activation, autoplay }: { name: IconName; size: number; activation: RefObject<Activation | null>; autoplay: boolean }) {
  const host = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    const animations = new Set<globalThis.Animation>();
    const cancel = () => { for (const animation of animations) animation.cancel(); animations.clear(); };
    const replay = () => {
      cancel();
      const recipes = isCatalog(name) ? [{ part: "", frames: [{ transform: "translateX(0px)" }, { transform: "translateX(.6px)" }, { transform: "translateX(0px)" }] }] : customMotions[name];
      for (const recipe of recipes) {
        const selector = recipe.part ? `[data-icon-part="${recipe.part}"]` : "svg";
        for (const part of host.current?.querySelectorAll<SVGElement>(selector) ?? []) {
          // Keep gait cadence even; joint curves supply the acceleration within each step.
          const animation = part.animate(recipe.frames, { duration: (name === "close" ? tempo.iconResult : tempo.iconReplay) * 1000, easing: name === "run" ? "linear" : `cubic-bezier(${tempo.soft.join(",")})` });
          animations.add(animation);
          animation.onfinish = () => { animations.delete(animation); animation.cancel(); };
        }
      }
    };
    const activate: Activation = event => { if (!event.defaultPrevented) replay(); };
    activation.current = activate;
    if (autoplay) replay();
    return () => { cancel(); if (activation.current === activate) activation.current = null; };
  }, [name, size, activation, autoplay]);
  return <span ref={host} data-custom-animation="" >{isCatalog(name) ? <StaticGlyph name={name} size={size} /> : <CustomGlyph name={name} size={size} />}</span>;
}

function controlState(host: HTMLElement | null) {
  const control = host && iconControl(host);
  const input = control?.querySelector<HTMLInputElement>('input[type="checkbox"], input[type="radio"]');
  if (input) return input.checked;
  if (control?.tagName === "SUMMARY" && control.parentElement instanceof HTMLDetailsElement) return control.parentElement.open;
  const expanded = control?.getAttribute("aria-expanded");
  return expanded === "true" ? true : expanded === "false" ? false : undefined;
}

type IconPlaybackProps = { name: IconName; size: number; activation: RefObject<Activation | null>; status?: Status; play?: boolean; className?: string; style?: CSSProperties; state?: boolean; reveal?: boolean; managed?: boolean };
function IconPlayback({ name, size, activation, status, play = false, className = "", style, state, reveal = true, managed = false }: IconPlaybackProps) {
  const host = useRef<HTMLSpanElement>(null);
  const present = useIsPresent();
  const [actualState, setActualState] = useState<boolean | undefined>(state);
  const [interrupted, setInterrupted] = useState(false);
  const [failed, setFailed] = useState<IconName | null>(null);
  const played = useRef(false);
  const trigger = ({ reason, event }: IconInteraction) => {
    if (reason === "state") { setActualState(controlState(host.current)); return; }
    if (status || stateful || !event) return;
    played.current = true;
    activation.current?.(event);
  };
  const eligible = useIconPlaybackEligibility(host, trigger, managed);
  const selected = state ?? actualState;
  const stateful = name === "checkbox" || name === "radio" || name === "toggle";
  const expanded = name === "chevronDown" && selected !== undefined;
  const pending = status === "pending";
  const animate = Boolean(eligible && present && failed !== name && (!status || pending || play && !interrupted));
  const autoplay = Boolean(status || reveal && !interrupted && !stateful && host.current && !iconControl(host.current));
  useLayoutEffect(() => {
    if (animate && autoplay) played.current = true;
    if (eligible === false && (status && !pending || played.current)) setInterrupted(true);
  }, [eligible, status, pending, animate, autoplay]);
  useLayoutEffect(() => () => { setInterrupted(true); }, []);
  const fallback = <StaticGlyph name={name} size={size} state={selected} />;
  const entry = isCatalog(name) ? catalogIcons[name] : null;
  return <span ref={host} className={`animated-icon ${className}`.trim()} data-icon={name} data-icon-source={entry ? "useanimations" : "custom"} data-icon-state={stateful || expanded ? String(Boolean(selected)) : undefined} data-icon-playback={animate ? pending ? "pending" : status ? "result" : "ready" : "static"} style={{ ...style, "--icon-size": `${size}px` } as CSSProperties} aria-hidden="true">
    {animate ? <PlayerBoundary key={name} fallback={fallback} onFailure={() => setFailed(name)}>{entry && name !== "search" ? <UseAnimations animation={entry.animation} size={size} interaction={pending ? "loop" : stateful ? "click-toggle" : "click-replay"} active={stateful ? Boolean(selected) : undefined} autoplay={stateful ? false : autoplay} loop={pending} speed={pending ? 1 : speedFor(entry.animation, status ? tempo.iconResult : tempo.iconReplay)} render={(events, attributes) => <PlayerFrame events={events} attributes={attributes} activation={activation} />} /> : <NativePlayer name={name} size={size} activation={activation} autoplay={autoplay} />}</PlayerBoundary> : fallback}
  </span>;
}

export function AnimatedIcon({ name, size = 18, ...props }: { name: IconName; size?: number; className?: string; style?: CSSProperties; state?: boolean; reveal?: boolean; play?: boolean }) {
  const activation = useRef<Activation | null>(null);
  return <IconPlayback name={name} size={size} activation={activation} status={name === "pending" ? "pending" : undefined} {...props} />;
}

export function ControlAnimation({ name, size = 18, children }: { name?: AnimatedIconName; size?: number; children: (activate: Activation, icon: ReactNode) => ReactElement }) {
  const activation = useRef<Activation | null>(null);
  const activate: Activation = event => {
    const control = event.currentTarget;
    if (!(control instanceof HTMLElement) || event.defaultPrevented || document.hidden || matchMedia("(prefers-reduced-motion: reduce)").matches || control.matches(":disabled, .is-dragging") || control.closest('[inert], [aria-disabled="true"]') || !motionVisible(control)) return;
    activation.current?.(event);
  };
  return children(activate, name ? <IconPlayback name={name} size={size} activation={activation} managed reveal={false} /> : null);
}

export function StatusIcon({ status, size = 18, play = false }: { status: Status; size?: number; play?: boolean }) {
  const activation = useRef<Activation | null>(null);
  return <IconPlayback name={status} status={status} size={size} activation={activation} play={play} />;
}
