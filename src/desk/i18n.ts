import { createInstance } from "i18next";
import type { TrainingPlan } from "../lib/types";
import en from "./locales/en.json";
import es from "./locales/es.json";
import fr from "./locales/fr.json";
import de from "./locales/de.json";
import it from "./locales/it.json";
import pt from "./locales/pt.json";
import ja from "./locales/ja.json";
import zh from "./locales/zh.json";

const catalogs = { en, es, fr, de, it, pt, ja, zh };
export type Locale = keyof typeof catalogs;

export function resolveLocale(language: string | undefined): Locale {
  const base = language?.split(/[-_]/)[0].toLowerCase();
  return base && Object.prototype.hasOwnProperty.call(catalogs, base) ? base as Locale : "en";
}

export function languagePreference(): Locale | "system" {
  try {
    const saved = localStorage.getItem("runalytics.language");
    return saved && Object.prototype.hasOwnProperty.call(catalogs, saved) ? saved as Locale : "system";
  } catch {
    return "system";
  }
}

const preference = languagePreference();
export const locale = preference === "system" ? resolveLocale(typeof navigator === "undefined" ? undefined : navigator.language) : preference;
export const formatLocale = locale === "en" ? "en-GB" : locale === "zh" ? "zh-CN" : locale;
const i18n = createInstance();
void i18n.init({
  lng: locale,
  fallbackLng: "en",
  resources: Object.fromEntries(Object.entries(catalogs).map(([language, translation]) => [language, { translation }])),
  initAsync: false,
  keySeparator: false,
  nsSeparator: false,
  interpolation: { escapeValue: false },
  returnEmptyString: false,
});

export function t(key: string, values: Record<string, unknown> = {}): string {
  return i18n.t(key, { defaultValue: key, ...values });
}

const patterns = Object.keys(en).filter(key => key.includes("{{")).map(key => {
  const names: string[] = [];
  const escaped = key.split(/(\{\{\w+\}\})/).map(part => {
    if (/^\{\{\w+\}\}$/.test(part)) { names.push(part.slice(2, -2)); return "([\\s\\S]+?)"; }
    return part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }).join("");
  return { key, names, expression: new RegExp(`^${escaped}$`) };
}).sort((first, second) => second.key.length - first.key.length);

export function text(value: string): string;
export function text(value: string | null): string | null;
export function text(value: string | undefined): string | undefined;
export function text(value: string | null | undefined): string | null | undefined {
  if (locale === "en" || !value) return value;
  if (Object.prototype.hasOwnProperty.call(en, value)) return t(value);
  for (const pattern of patterns) {
    const match = pattern.expression.exec(value);
    if (match) return t(pattern.key, Object.fromEntries(pattern.names.map((name, index) => [name, match[index + 1]])));
  }
  return value.split("\n").map(line => {
    if (line !== value) return text(line);
    const prefix = /^(Error: |[\p{Extended_Pictographic}\uFE0F]+\s*)/u.exec(line)?.[0];
    return prefix ? prefix + text(line.slice(prefix.length)) : line;
  }).join("\n");
}

export function number(value: number, digits?: number): string;
export function number(value: number | null | undefined, digits?: number): string | undefined;
export function number(value: number | null | undefined, digits = 0): string | undefined {
  return value == null ? undefined : new Intl.NumberFormat(formatLocale, { minimumFractionDigits: digits, maximumFractionDigits: digits }).format(value);
}
const dateValue = (value: string | number | Date) => typeof value === "string" && /^\d{4}-\d{2}-\d{2}$/.test(value) ? new Date(`${value}T12:00:00`) : new Date(value);
export const date = (value: string | number | Date) => dateValue(value).toLocaleDateString(formatLocale, { day: "2-digit", month: "2-digit", year: "numeric" });
export const dateTime = (value: string | number | Date) => `${dateValue(value).toLocaleTimeString(formatLocale, { hour: "2-digit", minute: "2-digit", hour12: false })} ${date(value)}`;
export const distance = (meters: number) => `${number(meters / 1000, 1)} km`;
export function duration(seconds: number, includeSeconds = false): string {
  const rounded = includeSeconds ? Math.round(seconds) : seconds;
  const hours = Math.floor(rounded / 3600);
  const minutes = Math.floor(rounded % 3600 / 60);
  const result = hours > 0 ? t("{{hours}}h {{minutes}}m", { hours, minutes: String(minutes).padStart(2, "0") }) : t("{{minutes}}m", { minutes });
  return includeSeconds ? t("{{duration}} {{seconds}}s", { duration: result, seconds: String(rounded % 60).padStart(2, "0") }) : result;
}
export const timeRange = (range: [number, number] | null) => range ? range[0] === range[1] ? t("{{value0}} min", { value0: number(range[0]) }) : t("{{value0}}-{{value1}} min", { value0: number(range[0]), value1: number(range[1]) }) : null;

export function planName(plan: TrainingPlan): string {
  if (plan.name?.trim()) return plan.name.trim();
  const kind = plan.kind === "improvement" ? "improvement" : plan.kind === "maintenance" ? "maintenance" : "race";
  const distances: Record<string, string> = { "5K": "5K", "10K": "10K", "21K": "Half marathon", "42K": "Marathon" };
  return t("{{distance}} {{kind}} plan · ends {{date}}", { distance: text(distances[plan.distance] ?? plan.distance), kind: text(kind), date: dateValue(plan.targetDate).toLocaleDateString(formatLocale, { day: "numeric", month: "short", year: "numeric" }) });
}

if (typeof document !== "undefined") document.documentElement.lang = locale;