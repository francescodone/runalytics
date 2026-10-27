/* Route maps: one basemap, one track drawing, one theme follower.

   The dashboard heatmap and the per-run route map share everything except the
   data they plot, so the shared half lives here and the workout modal can
   mount a route the same way the activity sheet does. Keeping a second copy
   in dashboard.ts is how the two would drift — a token change landing on one
   map and not the other reads as a theming bug. */

import maplibregl from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";
import { Protocol } from "pmtiles";
import { convertFileSrc } from "@tauri-apps/api/core";
import { inTauri } from "./ipc";
import { icons } from "./icons";
import type { Settings, TrackPoint } from "./types";

export function mapColor(token: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(token).trim();
}

export function heatmapColors(): maplibregl.ExpressionSpecification {
  return [
    "interpolate", ["linear"], ["heatmap-density"],
    0, "rgba(0,0,0,0)",
    0.25, mapColor("--map-heat-low"),
    0.55, mapColor("--map-heat-mid"),
    0.8, mapColor("--map-heat-high"),
    1, mapColor("--map-heat-peak"),
  ];
}

/// The basemap tiles (the pmtiles archive, and the online OSM tiles it is cut
/// from) are painted on paper: pale land, bright roads. That is right for the
/// light scheme and wrong for the dark one, where a white sheet glares behind
/// the ink canvas and swallows the palette the rest of the app is wearing.
/// MapLibre has no invert filter, so the dark treatment desaturates the tiles
/// almost to grey, pulls the highlights down toward the canvas, and adds
/// contrast back so roads and coastlines survive the darkening. The light
/// scheme takes the tiles as drawn.
///
/// The heat ramp, the track and the start/finish dots were already palette
/// driven; this is the missing half — the *layers* underneath them.
function basemapPaint(): Record<string, number> {
  if (document.documentElement.dataset.theme === "light") {
    return { "raster-opacity": 1, "raster-saturation": 0, "raster-contrast": 0 };
  }
  return {
    "raster-opacity": 0.94,
    "raster-saturation": -0.72,
    "raster-brightness-max": 0.62,
    "raster-brightness-min": 0.04,
    "raster-contrast": 0.28,
  };
}

/// The id of the raster basemap layer in the live style. The offline archive
/// and the online tiles name their source differently, so the theme follower
/// looks the layer up by type rather than hard-coding one of the two ids.
function basemapLayerId(map: maplibregl.Map): string | null {
  for (const layer of map.getStyle().layers ?? []) {
    if (layer.type === "raster") return layer.id;
  }
  return null;
}

/// Repaint the map's own colours when the scheme or palette changes. The app
/// themes off `[data-theme]`/`[data-palette]` attributes, not the media query,
/// so MapLibre's own style never notices on its own.
export function followMapTheme(map: maplibregl.Map): void {
  const update = () => {
    if (!map.isStyleLoaded()) return;
    map.setPaintProperty("bg", "background-color", mapColor("--bg"));
    const basemap = basemapLayerId(map);
    if (basemap) {
      const paint = basemapPaint();
      for (const [property, value] of Object.entries(paint)) {
        map.setPaintProperty(basemap, property, value);
      }
    }
    if (map.getLayer("heat")) map.setPaintProperty("heat", "heatmap-color", heatmapColors());
    if (map.getLayer("track-glow")) map.setPaintProperty("track-glow", "line-color", mapColor("--map-outline"));
    if (map.getLayer("track-line")) map.setPaintProperty("track-line", "line-color", mapColor("--scale-3"));
    if (map.getLayer("track-ends")) {
      map.setPaintProperty("track-ends", "circle-color", [
        "match", ["get", "kind"], "start", mapColor("--scale-2"), "finish", mapColor("--scale-5"), mapColor("--scale-4"),
      ]);
      map.setPaintProperty("track-ends", "circle-stroke-color", mapColor("--map-outline"));
    }
  };
  const observer = new MutationObserver(update);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "data-palette"] });
  map.on("load", update);
  map.on("remove", () => observer.disconnect());
}

/// Basemap style for the configured pmtiles archive (offline) or OSM raster
/// tiles. Shared by the heatmap and every route map.
export function buildMapStyle(settings: Settings): maplibregl.StyleSpecification {
  const pmtiles = settings.pmtilesPath;
  if (pmtiles && inTauri) {
    // Offline tiles: the pmtiles Protocol serves range requests from the
    // archive exposed through Tauri's asset protocol (supports HTTP Range).
    const protocol = new Protocol();
    maplibregl.addProtocol("pmtiles", protocol.tile);
    const archiveUrl = pmtiles.startsWith("~/")
      ? pmtiles // unlikely; backend resolves absolute paths only
      : convertFileSrc(pmtiles);
    const tileUrl = `pmtiles://${archiveUrl}/{z}/{x}/{y}`;
    return {
      version: 8,
      sources: {
        basemap: {
          type: "raster",
          tiles: [tileUrl],
          tileSize: 256,
          attribution: "PMTiles",
        },
      },
      layers: [
        { id: "bg", type: "background", paint: { "background-color": mapColor("--bg") } },
        { id: "basemap", type: "raster", source: "basemap", paint: basemapPaint() },
      ],
    };
  }
  return {
    version: 8,
    sources: {
      osm: {
        type: "raster",
        tiles: ["https://tile.openstreetmap.org/{z}/{x}/{y}.png"],
        tileSize: 256,
        attribution: "© OpenStreetMap contributors",
      },
    },
    layers: [
      { id: "bg", type: "background", paint: { "background-color": mapColor("--bg") } },
      { id: "osm", type: "raster", source: "osm", paint: basemapPaint() },
    ],
  };
}

/// The map's own zoom/compass/fit controls, drawn in the app's material.
/// MapLibre's built-ins are unusable here: their icons are baked into CSS
/// background images with a hardcoded colour, so they cannot follow the
/// light/dark scheme (the app themes off `[data-theme]`, not the media query),
/// and the global glass-pill `button` rule would restyle their boxes. This
/// control draws the same three actions with the app's Tabler icons and glass
/// tokens. `fit` is the one the library has no equivalent for: it returns the
/// view to the data the map was built around — the heatmap points, or the
/// run's track — which is the question a runner actually asks after panning
/// away. The object is structural (maplibre's `IControl` is not exported as a
/// type); `addControl` accepts anything with this shape.
export function mapControls(fit: () => void) {
  const specs: { icon: string; title: string; run: (map: maplibregl.Map) => void }[] = [
    { icon: icons.plus, title: "Zoom in", run: (m) => m.zoomIn() },
    { icon: icons.minus, title: "Zoom out", run: (m) => m.zoomOut() },
    { icon: icons.fit, title: "Fit to data", run: () => fit() },
    { icon: icons.locate, title: "Reset north", run: (m) => m.resetNorth() },
  ];
  let container: HTMLElement | null = null;
  return {
    onAdd(map: maplibregl.Map): HTMLElement {
      const group = document.createElement("div");
      // `.maplibregl-ctrl` earns pointer-events and the corner margin from the
      // library; `.map-ctrl` is the app's own glass box.
      group.className = "maplibregl-ctrl map-ctrl";
      for (const s of specs) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.title = s.title;
        btn.setAttribute("aria-label", s.title);
        btn.innerHTML = s.icon;
        btn.addEventListener("click", () => s.run(map));
        group.appendChild(btn);
      }
      container = group;
      return group;
    },
    onRemove(): void {
      container?.remove();
      container = null;
    },
    getDefaultPosition: () => "top-right" as const,
  };
}

/// Mount a run's route into `el`: the track as a glowing line with start and
/// finish dots at opposite ends of the data ramp, fitted to its own extent.
/// Returns the map so the caller can tear it down when its surface closes.
export function mountRouteMap(
  el: HTMLElement,
  track: TrackPoint[],
  settings: Settings,
): maplibregl.Map | null {
  if (track.length === 0) return null;
  el.innerHTML = "";
  const first = track[0];
  const coords = track.map((p) => [p.lon, p.lat]) as [number, number][];
  // Built before the map so the fit control can restore the run's own extent.
  const bounds = new maplibregl.LngLatBounds(coords[0], coords[0]);
  for (const c of coords) bounds.extend(c);
  const map = new maplibregl.Map({
    container: el,
    style: buildMapStyle(settings),
    center: [first.lon, first.lat],
    zoom: 12,
  });
  map.addControl(mapControls(() => map.fitBounds(bounds, { padding: 36, maxZoom: 16 })));
  followMapTheme(map);
  map.on("load", () => {
    // The basemap arrives asynchronously and a modal can be closed while it
    // is in flight; drawing into a detached container would leave the layer
    // on a map nobody can see or remove.
    if (!document.contains(el)) return;
    // Start / finish markers carry a `kind` property so the circle layer can
    // colour them differently from the line.
    map.addSource("track", {
      type: "geojson",
      data: {
        type: "FeatureCollection",
        features: [
          {
            type: "Feature",
            properties: { kind: "line" },
            geometry: { type: "LineString", coordinates: coords },
          },
          {
            type: "Feature",
            properties: { kind: "start" },
            geometry: { type: "Point", coordinates: coords[0] },
          },
          {
            type: "Feature",
            properties: { kind: "finish" },
            geometry: { type: "Point", coordinates: coords[coords.length - 1] },
          },
        ],
      },
    });
    map.addLayer({
      id: "track-glow",
      type: "line",
      source: "track",
      filter: ["==", ["get", "kind"], "line"],
      paint: {
        "line-color": mapColor("--map-outline"),
        "line-width": 6,
        "line-opacity": 0.55,
      },
    });
    map.addLayer({
      id: "track-line",
      type: "line",
      source: "track",
      filter: ["==", ["get", "kind"], "line"],
      paint: {
        "line-color": mapColor("--scale-3"),
        "line-width": 2.5,
        "line-opacity": 0.95,
      },
    });
    map.addLayer({
      id: "track-ends",
      type: "circle",
      source: "track",
      filter: ["!=", ["get", "kind"], "line"],
      paint: {
        "circle-radius": 5,
        // Start and finish use opposite ends of the shared data ramp, keeping
        // the route legible without introducing another meaning by hue.
        "circle-color": [
          "match",
          ["get", "kind"],
          "start",
          mapColor("--scale-2"),
          "finish",
          mapColor("--scale-5"),
          mapColor("--scale-4"),
        ],
        "circle-stroke-color": mapColor("--map-outline"),
        "circle-stroke-width": 1.5,
      },
    });
    map.fitBounds(bounds, { padding: 36, maxZoom: 16 });
  });
  return map;
}
