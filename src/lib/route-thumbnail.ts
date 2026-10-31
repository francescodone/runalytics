import type { TrackPoint } from "./types";

export function routeThumbnail(track: TrackPoint[]): string | null {
  const points = track.filter((point) => Number.isFinite(point.lat) && Number.isFinite(point.lon));
  if (points.length < 2) return null;
  const latitude = points.reduce((sum, point) => sum + point.lat, 0) / points.length;
  const longitudeScale = Math.max(0.01, Math.cos(latitude * Math.PI / 180));
  const west = Math.min(...points.map((point) => point.lon));
  const east = Math.max(...points.map((point) => point.lon));
  const south = Math.min(...points.map((point) => point.lat));
  const north = Math.max(...points.map((point) => point.lat));
  const width = (east - west) * longitudeScale;
  const height = north - south;
  const scale = 52 / Math.max(width, height, 0.000001);
  const insetX = (64 - width * scale) / 2;
  const insetY = (64 - height * scale) / 2;
  const stride = Math.max(1, Math.floor(points.length / 250));
  const simplified = points.filter((_, index) => index % stride === 0 || index === points.length - 1);
  const path = simplified.map((point, index) => `${index ? "L" : "M"}${(insetX + (point.lon - west) * longitudeScale * scale).toFixed(2)},${(insetY + (north - point.lat) * scale).toFixed(2)}`).join(" ");
  return `<svg viewBox="0 0 64 64" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="${path}"/></svg>`;
}