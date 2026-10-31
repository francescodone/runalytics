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
    const revealRoute = (progress: number) => {
      for (const [layer, color] of [["route-outline", "#ffffff"], ["route", "#16634b"]]) {
        instance.setPaintProperty(layer, "line-gradient", progress >= 1 ? ["to-color", color] : ["step", ["line-progress"], color, progress, "rgba(0,0,0,0)"]);
      }
    };
    const finishForReducedMotion = () => {
      if (!reducedMotion.matches) return;
      cancelAnimationFrame(animationFrame);
      if (instance.getLayer("route")) revealRoute(1);
    };
    reducedMotion.addEventListener("change", finishForReducedMotion);
    instance.on("load", () => {
      if (heat) {
        instance.addSource("history", {type:"geojson",data:{type:"FeatureCollection",features:points.map(point => ({type:"Feature",properties:{weight:"weight" in point ? point.weight : 1},geometry:{type:"Point",coordinates:[point.lon,point.lat]}}))}});
        instance.addLayer({id:"history",type:"heatmap",source:"history",paint:{"heatmap-weight":["get","weight"],"heatmap-intensity":1,"heatmap-radius":18,"heatmap-opacity":.75,"heatmap-color":["interpolate",["linear"],["heatmap-density"],0,"rgba(22,99,75,0)",.25,"#87c6a5",.5,"#36926b",.75,"#16634b",1,"#092c24"]}});
        fit.current(); return;
      }
      instance.addSource("route", { type: "geojson", lineMetrics: true, data: { type: "Feature", properties: {}, geometry: { type: "LineString", coordinates } } });
      instance.addLayer({ id: "route-outline", source: "route", type: "line", paint: { "line-color": "#ffffff", "line-width": 7, "line-gradient": ["to-color", "rgba(0,0,0,0)"] } });
      instance.addLayer({ id: "route", source: "route", type: "line", paint: { "line-color": "#16634b", "line-width": 4, "line-gradient": ["to-color", "rgba(0,0,0,0)"] } });
      instance.addSource("ends", { type: "geojson", data: { type: "FeatureCollection", features: [coordinates[0], coordinates[coordinates.length - 1]].map((coordinate, index) => ({ type: "Feature", properties: { end: index }, geometry: { type: "Point", coordinates: coordinate } })) } });
      instance.addLayer({ id: "ends", source: "ends", type: "circle", paint: { "circle-radius": 6, "circle-color": ["match", ["get", "end"], 0, "#16634b", "#2968ba"], "circle-stroke-color": "#fff", "circle-stroke-width": 2 } });
      fit.current();
      if (reducedMotion.matches) { revealRoute(1); return; }
      const started = performance.now();
      const draw = (now: number) => {
        const progress = Math.min((now - started) / 3000, 1);
        revealRoute(progress * progress * (3 - 2 * progress));
        if (progress < 1) animationFrame = requestAnimationFrame(draw);
      };
      revealRoute(0);
      animationFrame = requestAnimationFrame(draw);
    });
    instance.on("error", () => setFailed(true));
    const resize = new ResizeObserver(() => instance.resize());
    resize.observe(host.current);
    return () => { cancelAnimationFrame(animationFrame); reducedMotion.removeEventListener("change", finishForReducedMotion); resize.disconnect(); instance.remove(); map.current = null; };
  }, [track, heat, settings.pmtilesPath]);
  return <figure className="route-figure"><div ref={host} className="route-map" role="region" aria-label={heat ? "Training route heatmap" : "Recorded GPS route"} /><div className="route-controls"><IconButton label="Zoom in" onClick={() => map.current?.zoomIn({ duration: 0 })}><IconPlus size={18} /></IconButton><IconButton label="Zoom out" onClick={() => map.current?.zoomOut({ duration: 0 })}><IconMinus size={18} /></IconButton><IconButton label={heat ? "Fit training area" : "Fit route"} onClick={() => fit.current()}><IconFocus2 size={18} /></IconButton></div>{failed && <figcaption>Map tiles unavailable. The recorded route remains available when the map renderer is supported.</figcaption>}</figure>;
}