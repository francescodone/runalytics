import { createNativeChromeBridge, createTauriChromeTransport } from "./native-chrome";
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
  const blocked = () => document.visibilityState === "hidden" || !matchMedia("(max-width: 800px)").matches ||
    document.activeElement?.matches('input:not([type="button"]):not([type="submit"]), textarea, select, [contenteditable="true"]') ||
    [...document.querySelectorAll<HTMLElement>("dialog[open], .desk-inspector")].some(element => element.getClientRects().length > 0);
  const paint = (ready: boolean, bottomInset = 0) => {
    navigation.style.display = ready ? "none" : originalDisplay;
    workspace.style.paddingBottom = ready ? `calc(${bottomInset}px + var(--space-12))` : originalPadding;
    workspace.style.scrollPaddingBottom = ready ? `calc(${bottomInset}px + var(--space-4))` : originalScrollPadding;
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
    const identity = JSON.stringify({ suspended, selected, theme, width: innerWidth, height: innerHeight });
    if (identity === previous) return;
    previous = identity;
    void bridge.update(suspended ? null : {
      tabs: [{ id: "training", title: t("training") }, { id: "journal", title: t("journal") }, { id: "health", title: t("health") }],
      selected, theme,
    });
  };
  function schedule() {
    if (!stopped && !frame) frame = requestAnimationFrame(publish);
  }
  const observer = new MutationObserver(schedule);
  observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["open", "data-state", "inert"] });
  observer.observe(root, { attributes: true, attributeFilter: ["data-desk-theme"] });
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
    paint(false);
    delete root.dataset.nativeNavigation;
    void bridge.close();
  };
}