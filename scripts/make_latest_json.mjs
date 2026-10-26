#!/usr/bin/env node
// Build the static `latest.json` the Tauri updater reads from
// .../releases/latest/download/latest.json (see tauri.conf.json's
// plugins.updater.endpoints).
//
//   node scripts/make_latest_json.mjs <macos-bundle-dir> <owner/repo> <version>
//
// Reads Runalytics.app.tar.gz(.sig) from the bundle dir — the .sig is written
// by `tauri build` itself when TAURI_SIGNING_PRIVATE_KEY is set — and writes
// latest.json next to them. The manifest's `signature` field is the .sig file
// content verbatim: tauri-cli already base64-encodes the minisign signature
// box when it writes the file, and the updater base64-decodes the manifest
// field once. Encoding it again here would fail verification at install time.
//
// Only the host architecture gets an entry: scripts/release.sh builds one
// machine's bundle. On the other Mac architecture the updater reports
// TargetNotFound (check() errors, it does not silently see "no update"), so
// cross-arch updates come from CI releases only — tauri-action generates its
// own manifest covering both Darwin targets.
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { arch } from "node:os";

const [dir, ownerRepo, version] = process.argv.slice(2);
if (!dir || !ownerRepo || !version) {
  console.error(
    "usage: node scripts/make_latest_json.mjs <macos-bundle-dir> <owner/repo> <version>",
  );
  process.exit(1);
}

// The updater target string, e.g. `darwin-aarch64` / `darwin-x86_64`.
const target = `darwin-${arch() === "arm64" ? "aarch64" : "x86_64"}`;

const asset = "Runalytics.app.tar.gz";
const signature = readFileSync(join(dir, `${asset}.sig`), "utf8").trim();

const manifest = {
  version,
  notes: "",
  pub_date: new Date().toISOString(),
  platforms: {
    [target]: {
      url: `https://github.com/${ownerRepo}/releases/download/v${version}/${asset}`,
      signature,
    },
  },
};

const out = join(dir, "latest.json");
writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`  wrote ${out} (${target})`);
