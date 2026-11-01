import { useEffect, useRef, useState } from "react";
import maplibregl from "maplibre-gl";
import { Protocol } from "pmtiles";
import { convertFileSrc } from "@tauri-apps/api/core";
import { IconFocus2, IconPlus, IconMinus } from "@tabler/icons-react";
import type { HeatPoint, Settings, TrackPoint } from "../lib/types";
import { IconButton } from "./ui";
import "maplibre-gl/dist/maplibre-gl.css";

const protocol = new Protocol();
maplibregl.addProtocol("pmtiles", protocol.tile);

const FLAG_SVG = '<svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" focusable="false"><path class="flag-check" d="M0 0h6v6H0zM12 0h6v6h-6zM6 6h6v6H6zM18 6h6v6h-6zM0 12h6v6H0zM12 12h6v6h-6zM6 18h6v6H6zM18 18h6v6h-6z"/></svg>';
function themeColor(name: string, fallback: string) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}
function mixHex(base: string, other: string, weight: number) {
  const read = (hex: string) => {
    const full = hex.length === 4 ? `#${[...hex.slice(1)].map(channel => channel + channel).join("")}` : hex;
    return [1, 3, 5].map(offset => parseInt(full.slice(offset, offset + 2), 16));
  };
  const [a, b] = [read(base), read(other)];
  return `#${a.map((channel, i) => Math.round(channel * (1 - weight) + b[i] * weight).toString(16).padStart(2, "0")).join("")}`;
}

export default function RouteMap({ track, heat, settings }: { track: TrackPoint[]; heat?: HeatPoint[]; settings: Settings }) {
  const host = useRef<HTMLDivElement>(null);
  const map = useRef<maplibregl.Map | null>(null);
  const [failed, setFailed] = useState(false);
  const fit = useRef(() => {});
  useEffect(() => {
    const points = (heat ?? track).filter(point => Number.isFinite(point.lon) && Number.isFinite(point.lat) && Math.abs(point.lat) <= 90 && Math.abs(point.lon) <= 180);
    const coordinates = points.map(point => [point.lon, point.lat] as [number, number]);
    if (!host.current || !coordinates.length) return;
    setFailed(false);
    const bounds = coordinates.reduce((result, coordinate) => result.extend(coordinate), new maplibregl.LngLatBounds(coordinates[0], coordinates[0]));
    const offline = settings.pmtilesPath && "__TAURI_INTERNALS__" in window;
    const tiles = offline ? [`pmtiles://${convertFileSrc(settings.pmtilesPath!)}/{z}/{x}/{y}`] : ["https://tile.openstreetmap.org/{z}/{x}/{y}.png"];
    let instance: maplibregl.Map;
    try {
      instance = new maplibregl.Map({ container: host.current, attributionControl: { compact: true }, style: { version: 8, sources: { base: { type: "raster", tiles, tileSize: 256, attribution: offline ? "PMTiles" : '<a href="https://www.openstreetmap.org/copyright">OpenStreetMap contributors</a>' } }, layers: [{ id: "base", type: "raster", source: "base" }] }, center: coordinates[0], zoom: 12, canvasContextAttributes: { preserveDrawingBuffer: true } });
    } catch { setFailed(true); return; }
    map.current = instance;
    fit.current = () => instance.fitBounds(bounds, { padding: 40, maxZoom: 16, duration: 0 });
    let animationFrame = 0;
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    let action = themeColor("--desk-action", "#16634b");
    let outline = themeColor("--desk-surface", "#ffffff");
    let routeProgress = 0;
    const revealRoute = (progress: number) => {
      routeProgress = progress;
      for (const [layer, color] of [["route-outline", outline], ["route", action]]) {
        instance.setPaintProperty(layer, "line-gradient", progress >= 1 ? ["to-color", color] : ["step", ["line-progress"], color, progress, "rgba(0,0,0,0)"]);
      }
    };
    const heatColors = (): maplibregl.ExpressionSpecification => ["interpolate", ["linear"], ["heatmap-density"], 0, "rgba(0,0,0,0)", .25, mixHex(action, "#ffffff", .62), .5, mixHex(action, "#ffffff", .3), .75, action, 1, mixHex(action, "#000000", .55)];
    const paintTheme = () => {
      action = themeColor("--desk-action", "#16634b");
      outline = themeColor("--desk-surface", "#ffffff");
      const dark = document.documentElement.dataset.deskTheme === "dark";
      instance.setPaintProperty("base", "raster-saturation", -.85);
      instance.setPaintProperty("base", "raster-brightness-min", dark ? .06 : .3);
      instance.setPaintProperty("base", "raster-brightness-max", dark ? .32 : 1);
      instance.setPaintProperty("base", "raster-opacity", .88);
      instance.setPaintProperty("map-background", "background-color", mixHex(outline, action, .12));
      if (instance.getLayer("route")) {
        instance.setPaintProperty("route", "line-color", action);
        instance.setPaintProperty("route-outline", "line-color", outline);
        revealRoute(routeProgress);
      }
      if (instance.getLayer("history")) instance.setPaintProperty("history", "heatmap-color", heatColors());
    };
    const themeObserver = new MutationObserver(() => { if (instance.getLayer("map-background")) paintTheme(); });
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["data-desk-theme", "data-desk-palette"] });
    const finishForReducedMotion = () => {
      if (!reducedMotion.matches) return;
      cancelAnimationFrame(animationFrame);
      if (instance.getLayer("route")) revealRoute(1);
    };
    reducedMotion.addEventListener("change", finishForReducedMotion);
    instance.on("load", () => {
      instance.addLayer({ id: "map-background", type: "background" }, "base");
      paintTheme();
      if (heat) {
        instance.addSource("history", {type:"geojson",data:{type:"FeatureCollection",features:points.map(point => ({type:"Feature",properties:{weight:"weight" in point ? point.weight : 1},geometry:{type:"Point",coordinates:[point.lon,point.lat]}}))}});
        instance.addLayer({id:"history",type:"heatmap",source:"history",paint:{"heatmap-weight":["get","weight"],"heatmap-intensity":1,"heatmap-radius":18,"heatmap-opacity":.75,"heatmap-color":heatColors()}});
        fit.current(); return;
      }
      instance.addSource("route", { type: "geojson", lineMetrics: true, data: { type: "Feature", properties: {}, geometry: { type: "LineString", coordinates } } });
      instance.addLayer({ id: "route-outline", source: "route", type: "line", layout: { "line-cap": "round", "line-join": "round" }, paint: { "line-color": outline, "line-width": 8, "line-gradient": ["to-color", "rgba(0,0,0,0)"] } });
      instance.addLayer({ id: "route", source: "route", type: "line", layout: { "line-cap": "round", "line-join": "round" }, paint: { "line-color": action, "line-width": 4, "line-gradient": ["to-color", "rgba(0,0,0,0)"] } });
      const start = document.createElement("div");
      start.className = "route-marker route-marker-start";
      start.setAttribute("role", "img");
      start.setAttribute("aria-label", "Route start");
      const finish = document.createElement("div");
      finish.className = "route-marker route-marker-end";
      finish.setAttribute("role", "img");
      finish.setAttribute("aria-label", "Route finish");
      finish.innerHTML = FLAG_SVG;
      new maplibregl.Marker({ element: start, anchor: "center" }).setLngLat(coordinates[0]).addTo(instance);
      new maplibregl.Marker({ element: finish, anchor: "center" }).setLngLat(coordinates[coordinates.length - 1]).addTo(instance);
      fit.current();
      if (reducedMotion.matches) { revealRoute(1); return; }
      revealRoute(0);
      const started = performance.now();
      const draw = (now: number) => {
        const progress = Math.min((now - started) / 3000, 1);
        revealRoute(progress * progress * (3 - 2 * progress));
        if (progress < 1) animationFrame = requestAnimationFrame(draw);
      };
      animationFrame = requestAnimationFrame(draw);
    });
    instance.on("error", () => setFailed(true));
    const resize = new ResizeObserver(() => instance.resize());
    resize.observe(host.current);
    return () => { cancelAnimationFrame(animationFrame); themeObserver.disconnect(); reducedMotion.removeEventListener("change", finishForReducedMotion); resize.disconnect(); instance.remove(); map.current = null; };
  }, [track, heat, settings.pmtilesPath]);
  return <figure className="route-figure"><div ref={host} className="route-map" role="region" aria-label={heat ? "Training route heatmap" : "Recorded GPS route"} /><div className="route-controls"><IconButton label="Zoom in" onClick={() => map.current?.zoomIn({ duration: 0 })}><IconPlus size={18} /></IconButton><IconButton label="Zoom out" onClick={() => map.current?.zoomOut({ duration: 0 })}><IconMinus size={18} /></IconButton><IconButton label={heat ? "Fit training area" : "Fit route"} onClick={() => fit.current()}><IconFocus2 size={18} /></IconButton></div>{failed && <figcaption>Map tiles unavailable. The recorded route remains available when the map renderer is supported.</figcaption>}</figure>;
}