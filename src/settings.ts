import { tauri, el, fail, inTauri } from "./lib/ipc";
import { initSyncIndicator } from "./lib/sync";
import { initSegmented } from "./lib/segmented";
// Side-effect import: paints the saved colour scheme before first paint.
import "./lib/theme";
import "./lib/navigation";
import { initNavGate } from "./lib/navgate";
import { fetchSetupState, renderSetup } from "./lib/setup";
import { noteThemeChoice, PALETTES, setPalette } from "./lib/theme";
import { hydrateIcons } from "./lib/icons";
import { initPublishing } from "./lib/publishing";
import { startOnboarding } from "./lib/onboarding";
import { setBusy, withBusy, confirmSaved } from "./lib/feedback";
import { animatePaneChange, scrollIntoViewMotion } from "./lib/motion";
import type {
  LlmStatus,
  PaletteId,
  ProviderStatus,
  Settings,
  ThemeChoice,
  Thresholds,
} from "./lib/types";

/**
 * macOS-style category switching: one pane visible, the rest hidden. Panes are
 * `role="tabpanel"` and hidden with the `hidden` attribute; `.settings-content`
 * is a flex column, which is a `display` rule on the *parent*, so unlike a
 * `display` rule on the pane itself it cannot defeat `[hidden]`.
 */
function initPanes(): void {
  const nav = el("settings-nav");
  const buttons = Array.from(nav.querySelectorAll<HTMLButtonElement>("button[data-pane]"));
  const compact = window.matchMedia("(max-width: 860px)");
  const orient = () => nav.setAttribute("aria-orientation", compact.matches ? "horizontal" : "vertical");
  orient();
  compact.addEventListener("change", orient);
  let currentPane: string | null = null;

  function show(pane: string, scroll = false): void {
    for (const b of buttons) {
      const active = b.dataset.pane === pane;
      b.classList.toggle("active", active);
      b.setAttribute("aria-selected", String(active));
      b.tabIndex = active ? 0 : -1;
    }
    for (const section of document.querySelectorAll<HTMLElement>(".settings-content section[id^='pane-']")) {
      section.hidden = section.id !== `pane-${pane}`;
    }
    const section = document.getElementById(`pane-${pane}`);
    if (section && currentPane && currentPane !== pane) {
      const from = buttons.findIndex((b) => b.dataset.pane === pane) > buttons.findIndex((b) => b.dataset.pane === currentPane) ? "next" : "back";
      void animatePaneChange(section, from);
    }
    currentPane = pane;
    // The pane is the content the keyboard should land in next.
    if (scroll && section) scrollIntoViewMotion(section, { block: "nearest" });
  }

  buttons.forEach((b, i) => {
    b.id = `tab-${b.dataset.pane}`;
    b.setAttribute("aria-controls", `pane-${b.dataset.pane}`);
    document.getElementById(`pane-${b.dataset.pane}`)?.setAttribute("aria-labelledby", b.id);
    b.addEventListener("click", () => {
      show(b.dataset.pane!, true);
      history.replaceState(null, "", `#${b.dataset.pane}`);
    });
    // Vertical arrow navigation, as a macOS sidebar does.
    b.addEventListener("keydown", (e) => {
      const forward = compact.matches ? "ArrowRight" : "ArrowDown";
      const backward = compact.matches ? "ArrowLeft" : "ArrowUp";
      const dir = e.key === forward ? 1 : e.key === backward ? -1 : 0;
      if (!dir && e.key !== "Home" && e.key !== "End") return;
      e.preventDefault();
      const next = e.key === "Home" ? buttons[0] : e.key === "End" ? buttons[buttons.length - 1] : buttons[(i + dir + buttons.length) % buttons.length];
      show(next.dataset.pane!);
      history.replaceState(null, "", `#${next.dataset.pane}`);
      next.focus();
    });
  });

  // A deep link (#zones) opens that category — the nav is the page's only
  // structure, so losing the hash would make every link land on Setup.
  const initial = location.hash.slice(1);
  show(buttons.some((b) => b.dataset.pane === initial) ? initial : "setup");
}

/**
 * The Appearance pane. The choice is saved to settings.json (the source of
 * truth) and painted immediately through `noteThemeChoice`, so the whole app
 * changes colour the moment the pill is clicked — no reload, no save first.
 */
function initTheme(settings: Settings | null): void {
  const track = el("theme-toggle");
  const buttons = Array.from(track.querySelectorAll<HTMLButtonElement>("[data-theme]"));
  let current = (settings?.theme ?? "system") as ThemeChoice;

  // A radiogroup, not a tablist: this control chooses a stored setting, and
  // arrow keys move the choice itself (tabs would only move focus). The
  // `.active` class still drives the sliding pill in segmented.ts.
  const mark = (choice: ThemeChoice) => {
    buttons.forEach((b) => {
      const active = b.dataset.theme === choice;
      b.classList.toggle("active", active);
      b.setAttribute("aria-checked", String(active));
      // Roving tabindex: only the selected radio is in the tab order.
      b.tabIndex = active ? 0 : -1;
    });
  };
  mark(current);

  const choose = async (b: HTMLButtonElement): Promise<void> => {
    const choice = b.dataset.theme as ThemeChoice;
    if (choice === current) return;
    current = choice;
    mark(choice);
    noteThemeChoice(choice);
    try {
      const latest = await tauri.getSettings();
      await tauri.saveSettings({ ...latest, theme: choice });
    } catch (e) {
      fail(e);
    }
  };

  buttons.forEach((b, i) => {
    b.addEventListener("click", () => void choose(b));
    // Arrow keys select the neighbour and move focus with it, the way a
    // native radio group does; Space/Enter select the focused radio.
    b.addEventListener("keydown", (e) => {
      const forward = e.key === "ArrowRight" || e.key === "ArrowDown";
      const backward = e.key === "ArrowLeft" || e.key === "ArrowUp";
      if (!forward && !backward) return;
      e.preventDefault();
      const next = buttons[(i + (forward ? 1 : -1) + buttons.length) % buttons.length];
      next.focus();
      void choose(next);
    });
  });
}

/** The palette picker: one card per palette, a radiogroup like the scheme pills. */
function initPalette(settings: Settings | null): void {
  const grid = el("palette-picker");
  let current = setPalette(settings?.palette);

  const buttons = PALETTES.map((p) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "palette-option";
    b.dataset.palette = p.id;
    b.setAttribute("role", "radio");
    const swatch = document.createElement("span");
    swatch.className = "palette-swatch";
    swatch.setAttribute("aria-hidden", "true");
    for (const c of p.swatch) {
      const dot = document.createElement("i");
      dot.style.background = c;
      swatch.append(dot);
    }
    const name = document.createElement("strong");
    name.textContent = p.name;
    const blurb = document.createElement("span");
    blurb.className = "muted";
    blurb.textContent = p.blurb;
    b.append(swatch, name, blurb);
    grid.append(b);
    return b;
  });

  const mark = (id: PaletteId) => {
    buttons.forEach((b) => {
      const active = b.dataset.palette === id;
      b.classList.toggle("active", active);
      b.setAttribute("aria-checked", String(active));
      b.tabIndex = active ? 0 : -1;
    });
  };
  mark(current);

  const choose = async (b: HTMLButtonElement): Promise<void> => {
    const id = b.dataset.palette as PaletteId;
    if (id === current) return;
    current = setPalette(id);
    mark(current);
    try {
      const latest = await tauri.getSettings();
      await tauri.saveSettings({ ...latest, palette: id });
    } catch (e) {
      fail(e);
    }
  };

  buttons.forEach((b, i) => {
    b.addEventListener("click", () => void choose(b));
    b.addEventListener("keydown", (e) => {
      const forward = e.key === "ArrowRight" || e.key === "ArrowDown";
      const backward = e.key === "ArrowLeft" || e.key === "ArrowUp";
      if (!forward && !backward) return;
      e.preventDefault();
      const next = buttons[(i + (forward ? 1 : -1) + buttons.length) % buttons.length];
      next.focus();
      void choose(next);
    });
  });
}

// The zone editor's field map, and the ranges the backend clamps to. Kept here
// only to build the live preview and to know which input feeds which field —
// the authoritative clamping is Thresholds::normalized() in Rust, and saving
// re-reads the normalised values the backend returns.
const RATIO_FIELDS = [
  "ratioDecreasingHi",
  "ratioResumingHi",
  "ratioMaintainingHi",
  "ratioOptimizedHi",
] as const;
const RECOVERY_FIELDS = ["recoveryExhausted", "recoveryFatigued", "recoveryNormal"] as const;

function fieldId(key: keyof Thresholds): string {
  // camelCase -> the kebab-case id used in settings.html.
  return `th-${key.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)}`;
}

function readNumber(key: keyof Thresholds): number {
  const v = Number(el<HTMLInputElement>(fieldId(key)).value);
  return Number.isFinite(v) ? v : 0;
}

/** The form as a Thresholds object, unclamped — the backend normalises it. */
function formThresholds(): Thresholds {
  return {
    ratioDecreasingHi: readNumber("ratioDecreasingHi"),
    ratioResumingHi: readNumber("ratioResumingHi"),
    ratioMaintainingHi: readNumber("ratioMaintainingHi"),
    ratioOptimizedHi: readNumber("ratioOptimizedHi"),
    recoveryExhausted: readNumber("recoveryExhausted"),
    recoveryFatigued: readNumber("recoveryFatigued"),
    recoveryNormal: readNumber("recoveryNormal"),
    buildWeeks: readNumber("buildWeeks"),
    downWeekFactor: readNumber("downWeekFactor") / 100,
    paceToleranceSecKm: readNumber("paceToleranceSecKm"),
    paceStreak: readNumber("paceStreak"),
  };
}

function paintForm(th: Thresholds): void {
  for (const key of [
    ...RATIO_FIELDS,
    ...RECOVERY_FIELDS,
    "buildWeeks",
    "paceToleranceSecKm",
    "paceStreak",
  ] as (keyof Thresholds)[]) {
    el<HTMLInputElement>(fieldId(key)).value = String(th[key]);
  }
  // Stored as a fraction, shown as a percentage — a percent field is far
  // easier to reason about than "0.7".
  el<HTMLInputElement>("th-down-week-factor").value = String(Math.round(th.downWeekFactor * 100));
}

/**
 * Draw one band strip. Widths are proportional, so the strip shows at a glance
 * how wide a zone actually is — the reason the editor has one rather than only
 * number fields.
 *
 * A segment only gets its numeric range when it is wide enough to hold it:
 * narrowing a band to a tenth of the strip would otherwise clip "Performance
 * 1.30–1.50" to "rmance 1.30-", which is worse than showing the name alone.
 */
function paintStrip(
  host: HTMLElement,
  segments: { name: string; range: string; w: number; cls: string }[],
): void {
  const total = segments.reduce((s, x) => s + Math.max(x.w, 0), 0) || 1;
  host.innerHTML = segments
    .map((s) => {
      const frac = Math.max(s.w, 0) / total;
      const text = frac >= 0.17 ? `${s.name} ${s.range}` : s.name;
      return `<span class="zp-seg ${s.cls}" style="--w:${Math.max(s.w, 0.001)}" title="${s.name} ${s.range}">${text}</span>`;
    })
    .join("");
}

/**
 * Ratio bands, drawn on a 0–2 axis so the zones read proportionally. COROS's
 * six status names live in five segments: Resuming and Performance share one
 * range (COROS splits them by trend, not by ratio), so the strip shows them
 * as one segment with both names.
 */
function paintRatioPreview(th: Thresholds): void {
  const top = Math.max(2, th.ratioOptimizedHi * 1.15);
  const dec = th.ratioDecreasingHi;
  const res = th.ratioResumingHi;
  const main = th.ratioMaintainingHi;
  const opt = th.ratioOptimizedHi;
  const f = (v: number) => v.toFixed(2);
  paintStrip(el("ratio-preview"), [
    { name: "Decreasing", range: `< ${f(dec)}`, w: dec, cls: "zone-decreasing" },
    {
      name: "Resuming / Performance",
      range: `${f(dec)}–${f(res)}`,
      w: res - dec,
      cls: "zone-resuming",
    },
    { name: "Maintaining", range: `${f(res)}–${f(main)}`, w: main - res, cls: "zone-maintaining" },
    { name: "Optimized", range: `${f(main)}–${f(opt)}`, w: opt - main, cls: "zone-optimized" },
    { name: "Excessive", range: `> ${f(opt)}`, w: top - opt, cls: "zone-excessive" },
  ]);
}

/** Recovery bands on the 0–100 score axis. */
function paintRecoveryPreview(th: Thresholds): void {
  const e = th.recoveryExhausted;
  const f = th.recoveryFatigued;
  const n = th.recoveryNormal;
  paintStrip(el("recovery-preview"), [
    { name: "Exhausted", range: `≤ ${e}`, w: e + 1, cls: "tone-bad" },
    { name: "Fatigued", range: `${e + 1}–${f}`, w: f - e, cls: "tone-warn" },
    { name: "Normal", range: `${f + 1}–${n}`, w: n - f, cls: "tone-good" },
    { name: "Fresh", range: `${n + 1}–100`, w: 100 - n, cls: "tone-fresh" },
  ]);
}

function paintCycleHint(th: Thresholds): void {
  const cycle = th.buildWeeks + 1;
  el("cycle-hint").textContent =
    `A ${cycle}-week cycle: ${th.buildWeeks} week${th.buildWeeks === 1 ? "" : "s"} of ` +
    `increasing load, then an easier week at ${Math.round(th.downWeekFactor * 100)}% of your ` +
    `chronic load. COROS's own guidance is a 3:1 cycle at 70%.`;
}

function paintPreviews(): void {
  const th = formThresholds();
  paintRatioPreview(th);
  paintRecoveryPreview(th);
  paintCycleHint(th);
}

/**
 * The Training zones pane. Edits repaint the band strips live; Save writes and
 * then re-reads, so the form ends up showing exactly the values the backend
 * kept after clamping — if a number was out of range, the field corrects itself
 * rather than silently disagreeing with the dashboard.
 */
function initZones(settings: Settings | null): void {
  // Outside Tauri there are no settings to show; the pane stays blank rather
  // than filling the fields with "undefined".
  if (settings?.thresholds) paintForm(settings.thresholds);
  paintPreviews();

  document
    .querySelectorAll<HTMLElement>("#pane-zones input")
    .forEach((i) => i.addEventListener("input", paintPreviews));

  el("zones-save").addEventListener("click", async (ev) => {
    const btn = ev.target as HTMLButtonElement;
    try {
      await withBusy(btn, "Save", async () => {
        const latest = await tauri.getSettings();
        await tauri.saveSettings({ ...latest, thresholds: formThresholds() });
        const saved = await tauri.getSettings();
        if (saved?.thresholds) {
          paintForm(saved.thresholds);
          paintPreviews();
        }
      });
      // Only confirm once the write has actually landed.
      confirmSaved(btn, "Save");
    } catch (e) {
      fail(e);
    }
  });

  el("zones-reset").addEventListener("click", async (ev) => {
    const busy = setBusy(ev.currentTarget as HTMLButtonElement, "Restoring defaults…");
    try {
      // Sending the defaults through the same save path means the backend stays
      // the only place the COROS numbers are defined for a write.
      const latest = await tauri.getSettings().catch(() => null);
      if (!latest) return;
      const th: Thresholds = {
        ratioDecreasingHi: 0.5,
        ratioResumingHi: 0.8,
        ratioMaintainingHi: 1.0,
        ratioOptimizedHi: 1.5,
        recoveryExhausted: 19,
        recoveryFatigued: 69,
        recoveryNormal: 89,
        buildWeeks: 3,
        downWeekFactor: 0.7,
        paceToleranceSecKm: 4,
        paceStreak: 3,
      };
      await tauri.saveSettings({ ...latest, thresholds: th });
      paintForm(th);
      paintPreviews();
    } catch (e) {
      fail(e);
    } finally {
      busy.restore();
    }
  });
}


function setBadge(badge: HTMLElement, connected: boolean): void {
  badge.textContent = connected ? "connected" : "disconnected";
  badge.classList.toggle("ok", connected);
}

function applyStatus(statuses: ProviderStatus[]): void {
  const coros = statuses.find((s) => s.provider === "coros");
  const connected = coros?.connected ?? false;
  setBadge(el("coros-status"), connected);
  el<HTMLButtonElement>("coros-disconnect").disabled = !connected;
  el("coros-detail").textContent = coros?.lastSync
    ? `Last sync: ${new Date(coros.lastSync).toLocaleString()}`
    : (coros?.detail ?? "");
}

function showDesktopNotice(): void {
  el("coros-status").textContent = "desktop only";
  el("coros-detail").textContent =
    "This page is open in a plain browser. Runalytics gets its data from the desktop app — start it with `npm run tauri dev` and use Settings there.";
  el<HTMLButtonElement>("coros-check").disabled = true;
}

async function refresh(): Promise<void> {
  try {
    applyStatus(await tauri.providerStatus());
  } catch (e) {
    if (!inTauri) showDesktopNotice();
    else fail(e);
  }
}

/// The Setup pane: the three steps and the one action available now. Steps
/// live here because setup is configuration, not dashboard content — and
/// finishing the last step is what unlocks the rest of the nav, so every
/// advance re-checks the gate.
async function renderSetupPane(): Promise<void> {
  const host = el("setup-steps");
  const panel = el("setup-wizard");
  const advance = () => {
    void initNavGate().catch(() => undefined);
    void refresh().catch(() => undefined);
    void renderSetupPane();
  };  const state = await fetchSetupState();
  if (state === null) {
    host.innerHTML = "";
    panel.innerHTML =
      '<p class="setup-status">Runalytics needs the desktop app to set anything up — run ' +
      "`npm run tauri dev` and open this page there.</p>";
    return;
  }
  renderSetup(state, host, panel, advance);
}

async function main(): Promise<void> {
  hydrateIcons();
  initSyncIndicator();
  initSegmented();
  initPanes();
  initPublishing();
  void initNavGate().catch(() => undefined);
  // A sync from anywhere can complete step 2 — keep the pane honest about it.
  // The gate re-checks too: a plan created elsewhere (or a first sync landing
  // while this page is open) is what makes the other tabs appear, and the pane
  // must not claim "You're all set" over a sidebar that still hides them.
  window.addEventListener("runalytics:sync-completed", () => {
    void initNavGate().catch(() => undefined);
    void refresh().catch(() => undefined);
    void renderSetupPane();
  });
  await refresh();
  await renderSetupPane();
  const settings = await tauri.getSettings().catch(() => null);
  if (settings?.pmtilesPath) el<HTMLInputElement>("pmtiles-path").value = settings.pmtilesPath;
  initTheme(settings);
  initPalette(settings);
  initZones(settings);
  if (!inTauri) {
    for (const id of ["zones-save", "zones-reset", "tiles-save", "llm-save", "assistant-remove"]) {
      el<HTMLButtonElement>(id).disabled = true;
    }
  }

  el("replay-tutorial").addEventListener("click", () => startOnboarding());

  el("coros-check").addEventListener("click", async (ev) => {
    const btn = ev.target as HTMLButtonElement;
    el("coros-detail").textContent = "Checking the COROS API…";
    el("api-tools").innerHTML = "";
    try {
      const tools = await withBusy(btn, "Check API", () => tauri.checkProviderApi("coros"));
      el("coros-detail").textContent = `API reachable — ${tools.length} tools exposed.`;
      el("api-tools").innerHTML = tools
        .map((t) => `<span class="chip mono">${t}</span>`)
        .join("");
    } catch (e) {
      el("coros-detail").textContent =
        `API check failed: ${typeof e === "string" ? e : String(e)}. ` +
        "Connect first (the endpoint exists but needs an authorized token).";
    }
  });

  el("coros-disconnect").addEventListener("click", async (ev) => {
    try {
      await withBusy(ev.currentTarget as HTMLButtonElement, "Disconnecting…", () => tauri.disconnectProvider("coros"));
      await refresh();
      // Disconnecting undoes step 1 — the pane has to fall back with it.
      await renderSetupPane();
    } catch (e) {
      fail(e);
    }
  });

  el("tiles-save").addEventListener("click", async (ev) => {
    const btn = ev.target as HTMLButtonElement;
    try {
      await withBusy(btn, "Save", async () => {
        const current = await tauri.getSettings();
        await tauri.saveSettings({
          ...current,
          pmtilesPath: el<HTMLInputElement>("pmtiles-path").value.trim() || null,
        });
      });
      confirmSaved(btn, "Save");
    } catch (e) {
      fail(e);
    }
  });

  // ---- injury assistant ----
  // One assistant, not a list: the pane shows either the configured assistant
  // (read-only, key masked) or the add form. There is deliberately no edit
  // path — the key can't be read back out of the keychain, so changing one
  // means removing and re-adding, which keeps the card honest about what the
  // app actually holds.
  const renderAssistant = async (): Promise<void> => {
    const card = el("assistant-card");
    const form = el("assistant-form");
    let status: LlmStatus | null = null;
    try {
      status = await tauri.llmStatus();
    } catch {
      /* outside Tauri — the pane stays on the form at defaults */
    }
    const endpoint = status?.endpoint?.trim() ?? "";
    if (!endpoint) {
      card.hidden = true;
      form.hidden = false;
      return;
    }
    // The enable checkbox is gone: having an endpoint *is* being enabled. An
    // older settings.json could hold an endpoint with the flag off, which made
    // the assistant look configured everywhere else yet answer with the
    // offline note. Fold that state forward rather than showing a dead card.
    if (!status!.configured) {
      try {
        await tauri.saveSettings({ ...(await tauri.getSettings()), llmEnabled: true });
      } catch {
        /* outside Tauri — the card still describes the stored endpoint */
      }
    }
    el("assistant-endpoint").textContent = endpoint;
    el("assistant-model").textContent = status!.model?.trim() || "default";
    el("assistant-key").textContent = status!.hasKey
      ? "••••••••••••  stored in the keychain"
      : "none — this endpoint needs no key";
    form.hidden = true;
    card.hidden = false;
  };
  await renderAssistant();

  el("llm-save").addEventListener("click", async (ev) => {
    const btn = ev.target as HTMLButtonElement;
    const endpoint = el<HTMLInputElement>("llm-endpoint").value.trim();
    // Catch the shape here rather than at ask time: a typo saved as the
    // assistant's endpoint surfaces as "could not reach the endpoint", which
    // reads as a network problem, not as the field being wrong.
    const url = URL.canParse(endpoint) ? new URL(endpoint) : null;
    if (!url || (url.protocol !== "https:" && url.protocol !== "http:")) {
      fail("Enter the endpoint as a full URL, e.g. https://api.openai.com/v1.");
      return;
    }
    if (
      url.protocol === "http:" &&
      !["localhost", "127.0.0.1", "::1", "[::1]"].includes(url.hostname)
    ) {
      fail("Plain http is only allowed for a local endpoint (localhost).");
      return;
    }
    try {
      await withBusy(btn, "Add assistant", async () => {
        const key = el<HTMLInputElement>("llm-key").value.trim();
        if (key) await tauri.setLlmApiKey(key);
        await tauri.saveSettings({
          ...(await tauri.getSettings()),
          llmEnabled: true,
          llmEndpoint: endpoint,
          llmModel: el<HTMLInputElement>("llm-model").value.trim() || null,
        });
        el<HTMLInputElement>("llm-key").value = "";
      });
      confirmSaved(btn, "Add assistant");
      await renderAssistant();
    } catch (e) {
      fail(e);
    }
  });

  el("assistant-remove").addEventListener("click", async (ev) => {
    const btn = ev.target as HTMLButtonElement;
    try {
      await withBusy(btn, "Remove assistant", async () => {
        await tauri.saveSettings({
          ...(await tauri.getSettings()),
          llmEnabled: false,
          llmEndpoint: null,
          llmModel: null,
        });
        // The key belongs to this assistant, so it goes with it. No entry is
        // not an error — an endpoint may never have needed one.
        await tauri.clearLlmApiKey().catch(() => undefined);
      });
      await renderAssistant();
    } catch (e) {
      fail(e);
    }
  });
}

main();
