import { defineConfig } from "vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
// @ts-expect-error type error without @types/node package
import { fileURLToPath } from "node:url";
// @ts-expect-error type error without @types/node package
import { readFileSync } from "node:fs";

const host = process.env.TAURI_DEV_HOST;
const entryPages = new Set(["index.html", "plan.html", "calendar.html", "achievements.html", "injury.html", "settings.html", "support.html"]);
const deskTemplate = () => readFileSync(fileURLToPath(new URL("./desk.html", import.meta.url)), "utf8");

// package.json is the version source of truth (scripts/bump_version.sh keeps
// tauri.conf.json and Cargo in step), so the UI reads its version from here.
const pkg = JSON.parse(
  readFileSync(fileURLToPath(new URL("./package.json", import.meta.url)), "utf8"),
) as { version: string };

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [{
    name: "training-desk-entries",
    transformIndexHtml: {
      order: "pre",
      handler(html, context) {
        const entry = context.path === "/" ? "index.html" : context.path.slice(1);
        return entryPages.has(entry) ? deskTemplate() : html;
      },
    },
    async configureServer(server) {
      const { deskFixture } = await import("./tests/desk.fixture.mjs");
      server.middlewares.use(async (request, response, next) => {
        const pathname = request.url?.split("?")[0];
        const entry = pathname === "/" ? "index.html" : pathname?.slice(1);
        const legacy = request.headers["x-runalytics-legacy"] === "1" && entryPages.has(entry ?? "");
        if (pathname !== "/__desk_preview.html" && !legacy) return next();
        try {
          const template = legacy ? readFileSync(fileURLToPath(new URL(`./${entry}`, import.meta.url)), "utf8") : deskTemplate();
          const html = legacy ? template : template.replace('<script type="module"', `<script>${deskFixture}</script><script type="module"`);
          response.setHeader("Content-Type", "text/html");
          response.end(await server.transformIndexHtml(legacy ? `/__legacy/${entry}` : "/desk.html", html));
        } catch (error) { next(error); }
      });
    },
  }],
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  clearScreen: false,
  server: {
    port: 1499,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1499,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    rollupOptions: {
      input: {
        desk: fileURLToPath(new URL("./desk.html", import.meta.url)),
        main: fileURLToPath(new URL("./index.html", import.meta.url)),
        plan: fileURLToPath(new URL("./plan.html", import.meta.url)),
        calendar: fileURLToPath(new URL("./calendar.html", import.meta.url)),
        injury: fileURLToPath(new URL("./injury.html", import.meta.url)),
        achievements: fileURLToPath(new URL("./achievements.html", import.meta.url)),
        settings: fileURLToPath(new URL("./settings.html", import.meta.url)),
        support: fileURLToPath(new URL("./support.html", import.meta.url)),
      },
    },
  },
}));
