#!/usr/bin/env npx tsx

/**
 * Multi-Browser Build Script
 * ==========================
 *
 * After Vite builds the extension into `dist/`, this script creates
 * per-browser directories with browser-specific manifests:
 *
 * ```
 * dist/              ← Vite's output (shared JS/HTML/CSS)
 * dist/chrome/       ← Chrome-ready extension
 * dist/firefox/      ← Firefox-ready extension
 * dist/safari/       ← Safari-ready extension (input to converter)
 * ```
 *
 * Each browser directory contains the same compiled code but a different
 * `manifest.json` — Chrome's has `browser_specific_settings` removed,
 * Firefox's keeps it, and Safari's removes it.
 *
 * Usage:
 *   npm run build          # Vite build first
 *   npx tsx scripts/build-all-browsers.ts
 *
 * Or use the combined command:
 *   npm run build:release  # Vite build + this script
 */

import * as fs from "node:fs";
import * as path from "node:path";
import { transformManifest, type Browser } from "@coding-adventures/browser-extension-toolkit/src/manifest-transformer.js";

const BROWSERS: Browser[] = ["chrome", "firefox", "safari"];
const DIST_DIR = "dist";

/**
 * Recursively copy a directory, excluding specified subdirectories.
 */
function copyDir(src: string, dest: string, exclude: string[] = []): void {
  fs.mkdirSync(dest, { recursive: true });

  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    if (exclude.includes(entry.name)) continue;

    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);

    if (entry.isDirectory()) {
      copyDir(srcPath, destPath, exclude);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

/**
 * Build extension output for all target browsers.
 */
function buildAllBrowsers(): void {
  if (!fs.existsSync(DIST_DIR)) {
    console.error("Error: dist/ directory not found. Run `npm run build` first.");
    process.exit(1);
  }

  const distManifestPath = path.join(DIST_DIR, "manifest.json");
  if (!fs.existsSync(distManifestPath)) {
    console.error("Error: dist/manifest.json not found. Run `npm run build` first.");
    process.exit(1);
  }

  const baseManifest = JSON.parse(fs.readFileSync(distManifestPath, "utf-8"));

  for (const browser of BROWSERS) {
    const browserDir = path.join(DIST_DIR, browser);

    console.log(`Building for ${browser}...`);

    copyDir(DIST_DIR, browserDir, BROWSERS);

    const manifest = transformManifest(baseManifest, browser);
    fs.writeFileSync(
      path.join(browserDir, "manifest.json"),
      JSON.stringify(manifest, null, 2),
    );

    console.log(`  → ${browserDir}/`);
  }

  console.log("\nDone! Browser-specific builds:");
  for (const browser of BROWSERS) {
    const browserDir = path.join(DIST_DIR, browser);
    const files = fs.readdirSync(browserDir);
    console.log(`  ${browser}: ${files.length} files`);
  }
}

buildAllBrowsers();
