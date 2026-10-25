// Appearance: light / dark / system.
//
// The stylesheet keys every light-scheme token off `html[data-theme="light"]`
// (see app.css), so all this module has to do is keep that attribute correct.
//
// Two constraints shape the design:
//   1. The saved choice lives in settings.json, reachable only through an async
//      IPC call — too late for first paint.
//   2. The CSP has no 'unsafe-inline', so the attribute cannot be stamped by an
//      inline script in each page.
// So the choice is mirrored into localStorage, which is synchronous:
// applyFromCache() runs on import (module scripts run after the document is
// parsed but before first paint) and paints the right scheme immediately, and
// syncTheme() reconciles against settings.json once it has loaded. The cache is
// only ever a mirror — settings.json stays the source of truth.

import { tauri } from "./ipc";
import type { PaletteId, ThemeChoice } from "./types";

/** The resolved scheme actually painted — which token set is active. */
export type ResolvedTheme = "light" | "dark";

const CACHE_KEY = "runalytics.theme";

const CHOICES: ThemeChoice[] = ["system", "light", "dark"];

function readCache(): ThemeChoice | null {
  try {
    const v = localStorage.getItem(CACHE_KEY);
    return CHOICES.includes(v as ThemeChoice) ? (v as ThemeChoice) : null;
  } catch {
    // Private mode or a disabled storage layer: fall through to the default.
    return null;
  }
}

/** Every palette the Settings picker offers, default first. `swatch` is the
 * preview strip (canvas, primary, accent) in dark-scheme colours; `bar` is the
 * top of the nav gradient (`--brand-start`), which is what sits behind the iOS
 * status bar once the toolbar extends under it. */
export const PALETTES: {
  id: PaletteId;
  name: string;
  blurb: string;
  swatch: [string, string, string];
  bar: string;
}[] = [
  { id: "amethyst", name: "Amethyst Dusk", blurb: "The original violet twilight", swatch: ["#151321", "#7650dd", "#b6a7ff"], bar: "#5544e9" },
  { id: "glacier", name: "Glacier Lake", blurb: "Cold, clear alpine blue", swatch: ["#0a1721", "#0077bc", "#6ebdff"], bar: "#00719b" },
  { id: "evergreen", name: "Evergreen Trail", blurb: "Deep forest and fresh moss", swatch: ["#0b1911", "#008557", "#6fcc9e"], bar: "#007d31" },
  { id: "sakura", name: "Sakura Bloom", blurb: "Cherry blossom at first light", swatch: ["#1f1016", "#c12c77", "#f195b9"], bar: "#b40087" },
  { id: "ember", name: "Desert Ember", blurb: "Warm copper of a sunset run", swatch: ["#1f110b", "#b84b00", "#f49d74"], bar: "#bb2600" },
];

const PALETTE_KEY = "runalytics.palette";

function isPalette(v: unknown): v is PaletteId {
  return PALETTES.some((p) => p.id === v);
}

/** Tell the OS to colour the status-bar area like the top of the toolbar.
 * Each page's static meta carries the default; this keeps it honest once a
 * palette is applied — on iOS a stale value reads as a wrong-coloured bar. */
function paintStatusBar(palette: PaletteId): void {
  const meta = document.querySelector<HTMLMetaElement>("meta[name='theme-color']");
  const bar = PALETTES.find((p) => p.id === palette)?.bar;
  if (meta && bar) meta.content = bar;
}

/** Paint a palette and mirror it to the cache. The default carries no
 * attribute, so the base token set in app.css applies untouched. */
export function setPalette(id: PaletteId | null | undefined): PaletteId {
  const palette = isPalette(id) ? id : "amethyst";
  if (palette === "amethyst") delete document.documentElement.dataset.palette;
  else document.documentElement.dataset.palette = palette;
  paintStatusBar(palette);
  try {
    localStorage.setItem(PALETTE_KEY, palette);
  } catch {
    // Cache only; settings.json is reconciled on every load.
  }
  return palette;
}

function readPaletteCache(): PaletteId | null {
  try {
    const v = localStorage.getItem(PALETTE_KEY);
    return isPalette(v) ? v : null;
  } catch {
    return null;
  }
}

/** What the OS wants right now — the answer for "system". */
export function systemTheme(): ResolvedTheme {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/** Resolve a stored choice to the scheme to paint. Anything unrecognised means
 * "system" rather than a guessed colour, matching the pre-setting behaviour. */
export function resolveTheme(choice: ThemeChoice | null | undefined): ResolvedTheme {
  return choice === "light" || choice === "dark" ? choice : systemTheme();
}

/** The scheme currently painted, for the picker's initial state. */
export function currentTheme(): ResolvedTheme {
  return document.documentElement.dataset.theme === "light" ? "light" : "dark";
}

// The choice the OS listener is currently following, and whether it is installed.
let tracked: ThemeChoice | null | undefined = null;
let listening = false;

/**
 * Paint a choice and mirror it to the cache. Does not write settings.json —
 * saving is the caller's job — so this stays usable from a plain page load with
 * no form behind it. Returns the scheme now painted.
 *
 * The cache holds the *choice*, not the resolved scheme: a runner on "system"
 * must still follow the OS on the next launch, which a cached scheme would
 * freeze out.
 */
export function setTheme(choice: ThemeChoice | null | undefined): ResolvedTheme {
  const theme = resolveTheme(choice);
  document.documentElement.dataset.theme = theme;
  try {
    localStorage.setItem(CACHE_KEY, choice ?? "system");
  } catch {
    // Cache is an optimisation only; the next load re-reads settings anyway.
  }
  return theme;
}

// Applying on import is the whole point of the module: every page entry imports
// it, and module scripts run before first paint, so the attribute is set in
// time for the stylesheet's first match. With no cache yet (first launch) the
// choice is "system" — exactly what the app did before this setting existed.
document.documentElement.dataset.theme = resolveTheme(readCache());
setPalette(readPaletteCache());

/**
 * Reconcile with settings.json once it has loaded. Every page calls this after
 * its first data fetch; the cache may be stale if the setting changed on
 * another page, or the OS is the thing that moved.
 */
export async function syncTheme(): Promise<ResolvedTheme> {
  const settings = await tauri.getSettings().catch(() => null);
  if (settings) setPalette(settings.palette);
  return trackTheme(settings?.theme);
}

/**
 * For callers that already hold the settings (the Settings page): paint the
 * stored choice and install the OS listener. Call once per page load — later
 * picks go through `noteThemeChoice`, so the listener never stacks.
 */
export function trackTheme(choice: ThemeChoice | null | undefined): ResolvedTheme {
  tracked = choice;
  const painted = setTheme(choice);
  if (!listening) {
    listening = true;
    window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
      // Only "system" follows the OS; an explicit light/dark stays put.
      if (tracked !== "light" && tracked !== "dark") setTheme(tracked);
    });
  }
  return painted;
}

/** The runner changed their pick: repaint and retarget the OS listener. */
export function noteThemeChoice(choice: ThemeChoice | null | undefined): void {
  tracked = choice;
  setTheme(choice);
}

// Kick off the reconciliation here rather than in each page: this module is
// already imported by every entry for the synchronous paint above, so one call
// at import time covers all six pages without any per-page wiring. The cache
// paints instantly and this corrects it a moment later.
void syncTheme();
