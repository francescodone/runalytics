import { defineConfig } from "vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
// @ts-expect-error type error without @types/node package
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;

// Multi-page build: dashboard, plan, calendar, achievements and settings are
// separate pages navigated with plain links inside the single webview window.
//
// https://vite.dev/config/
export default defineConfig(() => ({
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
        main: fileURLToPath(new URL("./index.html", import.meta.url)),
        plan: fileURLToPath(new URL("./plan.html", import.meta.url)),
        calendar: fileURLToPath(new URL("./calendar.html", import.meta.url)),
        injury: fileURLToPath(new URL("./injury.html", import.meta.url)),
        achievements: fileURLToPath(new URL("./achievements.html", import.meta.url)),
        settings: fileURLToPath(new URL("./settings.html", import.meta.url)),
      },
    },
  },
}));
