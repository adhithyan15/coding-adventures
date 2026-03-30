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
 * Recursively copy a directory.
 *
 * We implement this ourselves rather than using `fs.cpSync` because
 * `cpSync` with `recursive: true` was added in Node 16.7 but the
 * `filter` option is newer. A simple recursive implementation is
 * clearer and more portable.
 */
function copyDir(src: string, dest: string, exclude: string[] = []): void {
  fs.mkdirSync(dest, { recursive: true });

  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    // Skip browser-specific output directories to prevent recursion.
    // When this script runs a second time, dist/chrome/, dist/firefox/,
    // dist/safari/ already exist inside dist/.
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
 *
 * Prerequisites:
 * - `npm run build` (Vite) must have run first, producing `dist/`
 * - `manifest.json` must exist in the project root
 *
 * This function:
 * 1. Reads the base manifest from the project root
 * 2. For each browser, creates a `dist/<browser>/` directory
 * 3. Copies all compiled files into it
 * 4. Writes a browser-specific manifest (using transformManifest)
 */
function buildAllBrowsers(): void {
  // Verify dist/ exists (Vite must have run first)
  if (!fs.existsSync(DIST_DIR)) {
    console.error("Error: dist/ directory not found. Run `npm run build` first.");
    process.exit(1);
  }

  // Read the base manifest from dist/ (already has paths rewritten by
  // our Vite plugin) rather than the project root
  const distManifestPath = path.join(DIST_DIR, "manifest.json");
  if (!fs.existsSync(distManifestPath)) {
    console.error("Error: dist/manifest.json not found. Run `npm run build` first.");
    process.exit(1);
  }

  const baseManifest = JSON.parse(fs.readFileSync(distManifestPath, "utf-8"));

  for (const browser of BROWSERS) {
    const browserDir = path.join(DIST_DIR, browser);

    console.log(`Building for ${browser}...`);

    // Copy all compiled files into the browser-specific directory.
    // Exclude the browser output directories themselves to prevent
    // infinite recursion on repeated runs.
    copyDir(DIST_DIR, browserDir, BROWSERS);

    // Transform the manifest for this browser and write it
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
