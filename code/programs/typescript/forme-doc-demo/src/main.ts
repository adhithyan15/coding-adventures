/**
 * main.ts — CLI entrypoint for the DOC00 v0 demo driver.
 *
 * This is the ONLY file in the program that touches the
 * filesystem.  Everything else (`build.ts`, `plain-text.ts`)
 * is a pure transform.
 *
 *   Usage:
 *     tsx src/main.ts [<corpus-dir>] [<out-dir>]
 *     tsx src/main.ts                          → corpus → dist
 *     tsx src/main.ts ./my-md ./build/site
 *
 * Capabilities: `fs:read`, `fs:list`, `fs:write`, `fs:create`
 * (declared in `required_capabilities.json`).  No network, no
 * shell, no env access.
 *
 * Safety:
 *   - Output directory is validated against absolute paths and
 *     `..` segments BEFORE any directory is created.  An attacker
 *     setting `OUT=/etc` doesn't get to write into /etc.
 *   - Corpus reads are scoped to a single directory walk; the
 *     walker rejects symlinks (no traversal escape via symlink
 *     to /etc/passwd or similar).
 *   - Write paths derived from the bundle's routes have already
 *     been validated by the site-emitter (no `..`, no `\`,
 *     leading `/` required) — we still re-confirm before
 *     opening files, as defence in depth.
 */

import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as url from "node:url";
import { routeToOutputPath } from "@coding-adventures/forme-aot-page-bundle-emitter";

import { build, type MarkdownFile } from "./build.js";
import { bundleSearchClient } from "./search-bundle.js";

// ─────────────────────────────────────────────────────────────────────
// CLI entry
// ─────────────────────────────────────────────────────────────────────

async function cli(): Promise<void> {
  const [, , corpusArg, outArg] = process.argv;
  const corpusDir = path.resolve(process.cwd(), corpusArg ?? "corpus");
  const outDir = path.resolve(process.cwd(), outArg ?? "dist");

  validateOutDir(outDir, process.cwd());
  await warnIfOverwritingFiles(outDir);

  console.log(`[forme-doc-demo] corpus = ${corpusDir}`);
  console.log(`[forme-doc-demo] out    = ${outDir}`);

  const files = await readCorpus(corpusDir);
  console.log(`[forme-doc-demo] read   = ${files.length} markdown files`);

  // Bundle the browser-side search client + UI glue.  esbuild
  // pulls in SearchClient + tokenizer (both pure TS, browser-
  // safe), wraps with the in-page bootstrap, minifies, hands
  // back a string we plug into `emitSite` as `search.clientJs`.
  console.log(`[forme-doc-demo] bundling search client …`);
  const searchClientJs = await bundleSearchClient();
  console.log(`[forme-doc-demo] bundle = ${(searchClientJs.length / 1024).toFixed(1)}KB minified`);

  const bundle = build(files, {
    siteTitle: "Acme Docs",
    githubUrl: "https://github.com/example/acme",
    copyright: `© ${new Date().getFullYear()} Acme`,
    searchClientJs,
  });

  await writeBundle(bundle, outDir);
  console.log(`[forme-doc-demo] wrote  = ${bundle.pages.length} files to ${outDir}`);
  console.log("");
  console.log("Done.  Serve it with any static HTTP server, e.g.:");
  console.log(`    npx serve ${path.relative(process.cwd(), outDir) || "."}`);
  console.log(`    python3 -m http.server --directory ${path.relative(process.cwd(), outDir) || "."}`);
}

// Only invoke the CLI when this module is the entry point, NOT
// when a test imports its helpers.  We compare this module's URL
// against the entry-script URL using `url.pathToFileURL` — that
// handles Windows drive letters and URL-encoding correctly,
// avoiding the basename-suffix fallback's spoofability /
// false-positive risk (a test runner whose entry script happens
// to be named `main.ts` would otherwise auto-execute the CLI).
const isCliEntry =
  process.argv[1] !== undefined &&
  import.meta.url === url.pathToFileURL(process.argv[1]).href;

if (isCliEntry) {
  cli().catch((err: unknown) => {
    console.error("[forme-doc-demo] FAILED:", err instanceof Error ? err.message : err);
    process.exitCode = 1;
  });
}

// ─────────────────────────────────────────────────────────────────────
// Filesystem helpers — these are the entire I/O surface.
// ─────────────────────────────────────────────────────────────────────

/**
 * Validate the user-supplied output directory.
 *
 * The most important guarantee is downstream: every per-file
 * write goes through `safeJoin(outDir, relPath)` which
 * re-validates containment.  This function's job is to catch
 * obviously-dangerous `outDir` *values* before any directory
 * is created.
 *
 * Rules:
 *   - Reject empty / non-string.
 *   - Reject explicit Unix system roots (`/`, `/etc`, ...) and
 *     Windows system roots (`C:\Windows`, `C:\Program Files`, ...).
 *   - Require the output dir to live *inside* the current
 *     working directory.  This is the single most effective
 *     guard — a `npm start corpus ~/Documents` typo otherwise
 *     happily overwrites files in the user's Documents folder.
 *
 *     Callers running the CLI from a project root (the typical
 *     case) get sensible behaviour: `./dist`, `./build`,
 *     `./out`, `./public` all work.  A path *outside* CWD is
 *     refused with an explicit error pointing at the override.
 */
export function validateOutDir(outDir: string, cwd: string): void {
  if (typeof outDir !== "string" || outDir.length === 0) {
    throw new Error("validateOutDir: outDir must be a non-empty string");
  }
  // System-directory blocklist — defence in depth even though
  // the cwd-containment check below would catch most of these
  // (a CLI run from `/etc` is unusual but not impossible).
  //
  // Unix system roots.
  const bannedUnix = ["/", "/bin", "/boot", "/dev", "/etc", "/home",
                      "/lib", "/opt", "/proc", "/root", "/sbin",
                      "/sys", "/usr", "/var"];
  // Windows system paths — match case-insensitively so the user
  // can't sidestep by passing "c:\\WINDOWS" or similar.
  const bannedWin = ["C:\\", "C:\\Windows", "C:\\Program Files",
                     "C:\\Program Files (x86)", "C:\\Users",
                     "C:\\ProgramData"];
  // Strip trailing separators from BOTH sides so `c:\` matches
  // `C:\` matches `C:` — avoids accidentally allowing through a
  // trailing-separator variant of a banned path.
  //
  // Explicit charCodeAt loop (no regex) to satisfy CodeQL's
  // `js/polynomial-redos` rule, which flags `+`-quantified
  // regexes on user input regardless of actual polynomial
  // behaviour.  Matches the project-wide convention established
  // by sidebar-builder/page-shell after the same rule fired
  // there.
  const stripTrailing = (s: string): string => {
    let end = s.length;
    while (end > 0) {
      const c = s.charCodeAt(end - 1);
      if (c === 0x2f /* "/" */ || c === 0x5c /* "\" */) {
        end--;
      } else {
        break;
      }
    }
    return end === s.length ? s : s.slice(0, end);
  };
  const norm = stripTrailing(outDir);
  for (const b of bannedUnix) {
    if (norm === stripTrailing(b) || outDir === b) {
      throw new Error(`validateOutDir: refusing to write to system directory ${outDir}`);
    }
  }
  for (const b of bannedWin) {
    if (norm.toLowerCase() === stripTrailing(b).toLowerCase()) {
      throw new Error(`validateOutDir: refusing to write to system directory ${outDir}`);
    }
  }
  // Containment check: outDir must be cwd or a descendant of it.
  // `path.resolve` normalises both sides; we append `path.sep`
  // to defeat prefix-string false matches (the same trick as
  // `safeJoin`).
  const cwdResolved = path.resolve(cwd);
  const outResolved = path.resolve(outDir);
  const cwdWithSep = cwdResolved.endsWith(path.sep)
    ? cwdResolved
    : cwdResolved + path.sep;
  if (!(outResolved === cwdResolved || outResolved.startsWith(cwdWithSep))) {
    throw new Error(
      `validateOutDir: outDir must live inside the working directory (got ${outDir}, cwd ${cwd})`,
    );
  }
}

/**
 * If `outDir` already exists and contains files that don't look
 * like a previous demo build's output, log a prominent warning.
 *
 * We don't BLOCK overwrite — the typical run case is re-running
 * the demo against the previous `dist/` — but we do want a user
 * who accidentally points the demo at their actual website to
 * see the warning before everything is silently overwritten.
 *
 * Heuristic: a "previous demo build" has either no files at all,
 * or `index.html` + `sidebar.json` + `search/manifest.json` at
 * the top level.  Anything else triggers the warning.
 */
async function warnIfOverwritingFiles(outDir: string): Promise<void> {
  let entries: import("node:fs").Dirent[];
  try {
    entries = await fs.readdir(outDir, { withFileTypes: true });
  } catch {
    return; // Doesn't exist yet → nothing to warn about.
  }
  if (entries.length === 0) return;
  // Look for the demo-build fingerprint.
  const names = new Set(entries.map((e) => e.name));
  const looksLikeDemoBuild =
    names.has("index.html") && names.has("sidebar.json") && names.has("search");
  if (looksLikeDemoBuild) return;
  console.warn(
    `[forme-doc-demo] WARNING: ${outDir} contains files that don't look ` +
    `like a previous demo build (${entries.length} entries: ` +
    `${entries.slice(0, 5).map((e) => e.name).join(", ")}${entries.length > 5 ? ", ..." : ""}). ` +
    `Existing files at conflicting paths WILL be overwritten.`,
  );
}

/**
 * Walk a corpus directory and return every `.md` file under it.
 * Refuses symlinks (defence against escape-via-symlink).  Result
 * paths are relative to `root` and use forward slashes (so the
 * downstream `routeFor` works identically on Windows).
 */
export async function readCorpus(root: string): Promise<MarkdownFile[]> {
  const files: MarkdownFile[] = [];
  await walk(root, "", files);
  // Stable order: sort by path so build output is deterministic
  // regardless of `readdir`'s OS-specific iteration order.
  files.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
  return files;
}

async function walk(root: string, rel: string, out: MarkdownFile[]): Promise<void> {
  const absDir = path.join(root, rel);
  const entries = await fs.readdir(absDir, { withFileTypes: true });
  for (const e of entries) {
    if (e.isSymbolicLink()) continue;        // skip symlinks unconditionally
    const childRel = rel === "" ? e.name : `${rel}/${e.name}`;
    if (e.isDirectory()) {
      await walk(root, childRel, out);
    } else if (e.isFile() && e.name.endsWith(".md")) {
      const source = await fs.readFile(path.join(root, childRel), "utf8");
      out.push({ path: childRel, source });
    }
  }
}

/**
 * Write a `PageBundleConfig` to disk.  Each PageEntry's body is
 * written to `outDir/<routeToOutputPath(route)>`; intermediate
 * directories are created on demand.
 *
 * Containment check: every resolved write target must start with
 * `outDir + path.sep`.  If a malicious route ever slipped past
 * the site-emitter's validation, this catches it before opening
 * any handle.
 */
export async function writeBundle(
  bundle: { pages: ReadonlyArray<{ route: string; html: string }> },
  outDir: string,
): Promise<void> {
  await fs.mkdir(outDir, { recursive: true });
  for (const page of bundle.pages) {
    const relPath = routeToOutputPath(page.route);
    const target = safeJoin(outDir, relPath);
    await fs.mkdir(path.dirname(target), { recursive: true });
    await fs.writeFile(target, page.html, "utf8");
  }
}

/**
 * Join `base` and `rel`, then assert the result stays within
 * `base`.  Defends against any rel-path that — after normalisation
 * — escapes upward.  Throws on escape, otherwise returns the
 * joined absolute path.
 */
export function safeJoin(base: string, rel: string): string {
  const target = path.resolve(base, rel);
  // Containment check using the resolved-prefix comparison.
  // Append `path.sep` to `base` so that `outDir/foo` doesn't
  // accept an outDir of `outD` (prefix-string false match).
  const baseWithSep = base.endsWith(path.sep) ? base : base + path.sep;
  if (!(target === base || target.startsWith(baseWithSep))) {
    throw new Error(`safeJoin: ${rel} escapes ${base} (resolved to ${target})`);
  }
  return target;
}
