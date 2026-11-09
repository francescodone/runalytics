import { createNativeChromeBridge, createTauriChromeTransport } from "./native-chrome";
import { navigate, readRoute } from "./data";
import { t } from "./i18n";

export async function startNativeChromeProbe() {
  const root = document.documentElement;
  const navigation = document.querySelector<HTMLElement>(".desk-navigation");
  const workspace = document.getElementById("workspace");
  if (!navigation || !workspace) return;
  const originalDisplay = navigation.style.display;
  const originalPadding = workspace.style.paddingBottom;
  let stopped = false;
  let frame = 0;
  let previous = "";
  const bridge = createNativeChromeBridge(await createTauriChromeTransport(), {
    onNavigate: selected => { previous = ""; navigate(selected); schedule(); },
    onState: state => {
      navigation.style.display = state.ready ? "none" : originalDisplay;
      workspace.style.paddingBottom = state.ready ? `calc(${state.bottomInset}px + var(--space-12))` : originalPadding;
      root.dataset.nativeChromeProbe = state.ready ? "ready" : "fallback";
      root.dataset.nativeChromeInset = String(state.bottomInset);
    },
  });
  const publish = () => {
    frame = 0;
    if (stopped) return;
    const blocked = [...document.querySelectorAll<HTMLElement>("dialog[open], .desk-inspector")].some(element => element.getClientRects().length > 0);
    const route = readRoute();
    const selected = route.workspace === "training" || route.workspace === "journal" || route.workspace === "health" ? route.workspace : null;
    const theme = root.dataset.deskTheme === "dark" ? "dark" : "light";
    const identity = JSON.stringify({ blocked, selected, theme, width: innerWidth, height: innerHeight });
    if (identity === previous) return;
    previous = identity;
    void bridge.update(blocked ? null : {
      tabs: [{ id: "training", title: t("training") }, { id: "journal", title: t("journal") }, { id: "health", title: t("health") }],
      selected, theme,
    });
  };
  function schedule() {
    if (!stopped && !frame) frame = requestAnimationFrame(publish);
  }
  const observer = new MutationObserver(schedule);
  observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["open", "data-state"] });
  observer.observe(root, { attributes: true, attributeFilter: ["data-desk-theme"] });
  window.addEventListener("hashchange", schedule);
  window.addEventListener("resize", schedule);
  const stop = () => {
    stopped = true;
    cancelAnimationFrame(frame);
    observer.disconnect();
    window.removeEventListener("hashchange", schedule);
    window.removeEventListener("resize", schedule);
    window.removeEventListener("pagehide", stop);
    void bridge.close();
  };
  window.addEventListener("pagehide", stop);
  schedule();
}