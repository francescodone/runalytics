import { createNativeChromeBridge, createTauriChromeTransport } from "./native-chrome";
import type { NativeChromeColors } from "./native-chrome";
import { navigate, readRoute } from "./data";
import { t } from "./i18n";

export function startNativeNavigation() {
  const root = document.documentElement;
  const navigation = document.querySelector<HTMLElement>(".desk-navigation");
  const workspace = document.getElementById("workspace");
  if (!navigation || !workspace) return () => {};
  const originalDisplay = navigation.style.display;
  const originalPadding = workspace.style.paddingBottom;
  const originalScrollPadding = workspace.style.scrollPaddingBottom;
  let stopped = false;
  let frame = 0;
  let previous = "";
  let acknowledgedInset = 0;
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = 1;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  const colors = (): NativeChromeColors | undefined => {
    if (!context) return;
    const style = getComputedStyle(root);
    const read = (token: string): [number, number, number, number] => {
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = style.getPropertyValue(token).trim();
      context.fillRect(0, 0, 1, 1);
      const [red, green, blue, alpha] = context.getImageData(0, 0, 1, 1).data;
      return [red / 255, green / 255, blue / 255, alpha / 255];
    };
    return { surface: read("--desk-surface"), selected: read("--desk-action"), unselected: read("--desk-muted") };
  };
  const blocked = () => document.visibilityState === "hidden" || !matchMedia("(max-width: 800px)").matches ||
    document.activeElement?.matches('input:not([type="button"]):not([type="submit"]), textarea, select, [contenteditable="true"]') ||
    [...document.querySelectorAll<HTMLElement>("dialog[open], .desk-inspector")].some(element => element.getClientRects().length > 0);
  const paint = (ready: boolean, bottomInset = 0) => {
    if (ready) acknowledgedInset = bottomInset;
    const retained = ready || acknowledgedInset > 0 && matchMedia("(max-width: 800px)").matches && blocked();
    navigation.style.display = retained ? "none" : originalDisplay;
    workspace.style.paddingBottom = retained ? `calc(${acknowledgedInset}px + var(--space-12))` : originalPadding;
    workspace.style.scrollPaddingBottom = retained ? `calc(${acknowledgedInset}px + var(--space-4))` : originalScrollPadding;
    root.dataset.nativeNavigation = ready ? "ready" : "fallback";
  };
  const bridge = createNativeChromeBridge({
    listen: async receive => (await createTauriChromeTransport()).listen(receive),
    attach: async snapshot => (await createTauriChromeTransport()).attach(snapshot),
    detach: async () => (await createTauriChromeTransport()).detach(),
  }, {
    onNavigate: selected => {
      previous = "";
      if (!blocked()) navigate(selected);
      schedule();
    },
    onState: state => { if (!stopped) paint(state.ready && !blocked(), state.bottomInset); },
  });
  const publish = () => {
    frame = 0;
    if (stopped) return;
    const suspended = Boolean(blocked());
    const route = readRoute();
    const selected = route.workspace === "training" || route.workspace === "journal" || route.workspace === "health" ? route.workspace : null;
    const theme = root.dataset.deskTheme === "dark" ? "dark" : "light";
    const appearance = colors();
    const identity = JSON.stringify({ suspended, selected, theme, colors: appearance, width: innerWidth, height: innerHeight });
    if (identity === previous) return;
    previous = identity;
    void bridge.update(suspended ? null : {
      tabs: [{ id: "training", title: t("training") }, { id: "journal", title: t("journal") }, { id: "health", title: t("health") }],
      selected, theme, colors: appearance,
    });
  };
  function schedule() {
    if (!stopped && !frame) frame = requestAnimationFrame(publish);
  }
  const observer = new MutationObserver(schedule);
  observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["open", "data-state", "inert"] });
  observer.observe(root, { attributes: true, attributeFilter: ["data-desk-theme", "data-desk-palette"] });
  const resume = () => { previous = ""; schedule(); };
  const suspend = () => {
    previous = "";
    void bridge.update(null);
  };
  window.addEventListener("hashchange", schedule);
  window.addEventListener("resize", schedule);
  window.addEventListener("pagehide", suspend);
  window.addEventListener("pageshow", resume);
  document.addEventListener("visibilitychange", resume);
  document.addEventListener("focusin", schedule);
  document.addEventListener("focusout", schedule);
  window.visualViewport?.addEventListener("resize", resume);
  schedule();
  return () => {
    stopped = true;
    cancelAnimationFrame(frame);
    observer.disconnect();
    window.removeEventListener("hashchange", schedule);
    window.removeEventListener("resize", schedule);
    window.removeEventListener("pagehide", suspend);
    window.removeEventListener("pageshow", resume);
    document.removeEventListener("visibilitychange", resume);
    document.removeEventListener("focusin", schedule);
    document.removeEventListener("focusout", schedule);
    window.visualViewport?.removeEventListener("resize", resume);
    acknowledgedInset = 0;
    paint(false);
    delete root.dataset.nativeNavigation;
    void bridge.close();
  };
}