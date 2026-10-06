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
 *     opening files, reject linked directory components and
 *     multiply-linked targets, and identity-check each open
 *     handle before truncation, as defence in depth.
 */

import * as fs from "node:fs/promises";
import type { FileHandle } from "node:fs/promises";
import { constants as fsConstants, type Stats } from "node:fs";
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
 * The output root must not exist. It is created privately under the canonical
 * working directory and protected by an exclusive writer lock for the whole
 * operation. Directory components are created and inspected one at a time
 * without traversing symlinks or Windows reparse points. Final files are
 * opened with O_NOFOLLOW where the host exposes it, then authority-chain,
 * path, handle, and lock identities are checked before truncation;
 * multiply-linked and unowned files are refused.
 */
export async function writeBundle(
  bundle: { pages: ReadonlyArray<{ route: string; html: string }> },
  outDir: string,
): Promise<void> {
  const context = await createPrivateOutputRoot(outDir);
  try {
    for (const page of bundle.pages) {
      const relPath = routeToOutputPath(page.route);
      const target = safeJoin(context.root, relPath);
      await createPrivateChildDirectories(context, path.dirname(target));
      await writeFileWithoutFollowingLinks(context, target, page.html);
    }
  } finally {
    await releasePrivateOutputRoot(context);
  }
}

type PathIdentity = Readonly<{ dev: number; ino: number }>;
type DirectorySnapshot = Readonly<{ path: string; identity: PathIdentity }>;
type OutputContext = {
  readonly anchor: string;
  readonly root: string;
  readonly authorityChain: ReadonlyArray<DirectorySnapshot>;
  readonly lockPath: string;
  readonly lockHandle: FileHandle;
  readonly lockIdentity: PathIdentity;
  readonly ownedFiles: Map<string, PathIdentity>;
};

function identityOf(stat: Stats): PathIdentity {
  return { dev: stat.dev, ino: stat.ino };
}

function sameIdentity(left: PathIdentity, right: PathIdentity): boolean {
  return left.dev === right.dev && left.ino === right.ino;
}

function isMissing(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    (error as { code?: unknown }).code === "ENOENT"
  );
}

async function inspectDirectory(candidate: string): Promise<PathIdentity> {
  const stat = await fs.lstat(candidate);
  if (stat.isSymbolicLink()) {
    throw new Error(`writeBundle: refusing symbolic link or reparse point ${candidate}`);
  }
  if (!stat.isDirectory()) {
    throw new Error(`writeBundle: expected directory at ${candidate}`);
  }
  return identityOf(stat);
}

async function inspectDirectoryChain(
  root: string,
  descendant: string,
): Promise<ReadonlyArray<DirectorySnapshot>> {
  const resolvedRoot = path.resolve(root);
  const resolvedDescendant = path.resolve(descendant);
  const relative = path.relative(resolvedRoot, resolvedDescendant);
  if (relative === ".." || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    throw new Error(`writeBundle: ${resolvedDescendant} escapes ${resolvedRoot}`);
  }

  const paths = [resolvedRoot];
  let current = resolvedRoot;
  for (const component of relative.split(path.sep).filter((part) => part.length > 0)) {
    current = path.join(current, component);
    paths.push(current);
  }

  const canonicalRoot = await fs.realpath(resolvedRoot);
  const canonicalRootWithSep = canonicalRoot.endsWith(path.sep)
    ? canonicalRoot
    : canonicalRoot + path.sep;
  const snapshots: DirectorySnapshot[] = [];
  for (const candidate of paths) {
    const identity = await inspectDirectory(candidate);
    const canonical = await fs.realpath(candidate);
    if (!(canonical === canonicalRoot || canonical.startsWith(canonicalRootWithSep))) {
      throw new Error(`writeBundle: linked directory ${candidate} escapes ${resolvedRoot}`);
    }
    snapshots.push({ path: candidate, identity });
  }
  return snapshots;
}

function assertSameDirectoryChain(
  before: ReadonlyArray<DirectorySnapshot>,
  after: ReadonlyArray<DirectorySnapshot>,
): void {
  if (
    before.length !== after.length ||
    before.some(
      (entry, index) =>
        after[index]?.path !== entry.path ||
        !sameIdentity(entry.identity, after[index]!.identity),
    )
  ) {
    throw new Error("writeBundle: output directory identity changed during write");
  }
}

function assertPrivateDirectory(candidate: string, stat: Stats): void {
  if (process.platform !== "win32" && (stat.mode & 0o077) !== 0) {
    throw new Error(`writeBundle: private root permissions changed at ${candidate}`);
  }
  if (
    process.platform !== "win32" &&
    typeof process.getuid === "function" &&
    stat.uid !== process.getuid()
  ) {
    throw new Error(`writeBundle: private root ownership changed at ${candidate}`);
  }
}

function assertAuthorityPrefix(
  authority: ReadonlyArray<DirectorySnapshot>,
  observed: ReadonlyArray<DirectorySnapshot>,
): void {
  if (observed.length < authority.length) {
    throw new Error("writeBundle: output authority chain was shortened");
  }
  assertSameDirectoryChain(authority, observed.slice(0, authority.length));
}

/**
 * Establish a single-writer output root beneath the canonical current working
 * directory. The root must not exist: exclusive creation plus mode 0700 and an
 * O_EXCL lock file form the enforced quiescent-root contract used on portable
 * Node runtimes that do not expose openat-style descriptor-relative writes.
 * Concurrent mutation by another process running as the same OS identity is
 * outside that contract; every observable identity change still fails closed.
 */
async function createPrivateOutputRoot(requested: string): Promise<OutputContext> {
  const lexicalAnchor = path.resolve(process.cwd());
  const requestedResolved = path.resolve(requested);
  const relative = path.relative(lexicalAnchor, requestedResolved);
  if (
    relative.length === 0 ||
    relative === ".." ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  ) {
    throw new Error("writeBundle: output directory must be a new child of the working directory");
  }

  const anchor = await fs.realpath(lexicalAnchor);
  const root = path.resolve(anchor, relative);
  const components = relative.split(path.sep).filter((component) => component.length > 0);
  let current = anchor;
  for (const component of components.slice(0, -1)) {
    current = path.join(current, component);
    try {
      await fs.mkdir(current, { mode: 0o700 });
    } catch (error) {
      if (
        typeof error !== "object" ||
        error === null ||
        !("code" in error) ||
        (error as { code?: unknown }).code !== "EEXIST"
      ) {
        throw error;
      }
    }
    await inspectDirectory(current);
  }

  try {
    const existing = await fs.lstat(root);
    if (existing.isSymbolicLink()) {
      throw new Error(`writeBundle: refusing symbolic link or reparse point ${root}`);
    }
    throw new Error(`writeBundle: private output root already exists at ${root}`);
  } catch (error) {
    if (!isMissing(error)) throw error;
  }

  await fs.mkdir(root, { mode: 0o700 });
  if (process.platform !== "win32") await fs.chmod(root, 0o700);
  const rootStat = await fs.lstat(root);
  if (rootStat.isSymbolicLink() || !rootStat.isDirectory()) {
    throw new Error(`writeBundle: private output root is linked or invalid at ${root}`);
  }
  assertPrivateDirectory(root, rootStat);

  const authorityChain = await inspectDirectoryChain(anchor, root);
  const lockPath = path.join(root, ".forme-write-lock");
  const noFollow = typeof fsConstants.O_NOFOLLOW === "number" ? fsConstants.O_NOFOLLOW : 0;
  const lockHandle = await fs.open(
    lockPath,
    fsConstants.O_WRONLY | fsConstants.O_CREAT | fsConstants.O_EXCL | noFollow,
    0o600,
  );
  const lockStat = await lockHandle.stat();
  if (!lockStat.isFile() || lockStat.nlink !== 1) {
    await lockHandle.close();
    throw new Error("writeBundle: could not establish the private writer lock");
  }

  return {
    anchor,
    root,
    authorityChain,
    lockPath,
    lockHandle,
    lockIdentity: identityOf(lockStat),
    ownedFiles: new Map<string, PathIdentity>(),
  };
}

async function verifyPrivateOutputRoot(context: OutputContext): Promise<void> {
  const observed = await inspectDirectoryChain(context.anchor, context.root);
  assertSameDirectoryChain(context.authorityChain, observed);
  const rootStat = await fs.lstat(context.root);
  assertPrivateDirectory(context.root, rootStat);

  const openedLock = await context.lockHandle.stat();
  const namedLock = await fs.lstat(context.lockPath);
  if (
    !openedLock.isFile() ||
    openedLock.nlink !== 1 ||
    namedLock.isSymbolicLink() ||
    namedLock.nlink !== 1 ||
    !sameIdentity(context.lockIdentity, identityOf(openedLock)) ||
    !sameIdentity(context.lockIdentity, identityOf(namedLock))
  ) {
    throw new Error("writeBundle: private writer lock identity changed");
  }
}

async function releasePrivateOutputRoot(context: OutputContext): Promise<void> {
  try {
    await verifyPrivateOutputRoot(context);
    await fs.unlink(context.lockPath);
  } finally {
    await context.lockHandle.close();
  }
}

async function createPrivateChildDirectories(
  context: OutputContext,
  requested: string,
): Promise<void> {
  const resolved = path.resolve(requested);
  const relative = path.relative(context.root, resolved);
  if (relative === ".." || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    throw new Error(`writeBundle: ${resolved} escapes ${context.root}`);
  }

  await verifyPrivateOutputRoot(context);
  let current = context.root;
  for (const component of relative.split(path.sep).filter((part) => part.length > 0)) {
    current = path.join(current, component);
    try {
      await fs.mkdir(current, { mode: 0o700 });
    } catch (error) {
      if (
        typeof error !== "object" ||
        error === null ||
        !("code" in error) ||
        (error as { code?: unknown }).code !== "EEXIST"
      ) {
        throw error;
      }
    }
    await inspectDirectory(current);
  }
  const observed = await inspectDirectoryChain(context.anchor, resolved);
  assertAuthorityPrefix(context.authorityChain, observed);
}

async function writeFileWithoutFollowingLinks(
  context: OutputContext,
  target: string,
  contents: string,
): Promise<void> {
  const parent = path.dirname(target);
  await verifyPrivateOutputRoot(context);
  const beforeDirectories = await inspectDirectoryChain(context.anchor, parent);
  assertAuthorityPrefix(context.authorityChain, beforeDirectories);

  try {
    const existing = await fs.lstat(target);
    if (existing.isSymbolicLink()) {
      throw new Error(`writeBundle: refusing symbolic link or reparse point ${target}`);
    }
    if (!existing.isFile()) {
      throw new Error(`writeBundle: expected regular file at ${target}`);
    }
    if (existing.nlink !== 1) {
      throw new Error(`writeBundle: refusing multiply-linked output file ${target}`);
    }
    const owned = context.ownedFiles.get(target);
    if (owned === undefined || !sameIdentity(owned, identityOf(existing))) {
      throw new Error(`writeBundle: refusing unowned existing output file ${target}`);
    }
  } catch (error) {
    if (!isMissing(error)) throw error;
  }

  const noFollow = typeof fsConstants.O_NOFOLLOW === "number" ? fsConstants.O_NOFOLLOW : 0;
  const handle = await fs.open(
    target,
    fsConstants.O_WRONLY | fsConstants.O_CREAT | noFollow,
    0o644,
  );
  try {
    const opened = await handle.stat();
    if (!opened.isFile() || opened.nlink !== 1) {
      throw new Error(`writeBundle: refusing linked or non-regular output file ${target}`);
    }
    const named = await fs.lstat(target);
    if (
      named.isSymbolicLink() ||
      named.nlink !== 1 ||
      !sameIdentity(identityOf(opened), identityOf(named))
    ) {
      throw new Error(`writeBundle: output file identity mismatch at ${target}`);
    }

    const afterOpenDirectories = await inspectDirectoryChain(context.anchor, parent);
    assertSameDirectoryChain(beforeDirectories, afterOpenDirectories);
    await verifyPrivateOutputRoot(context);

    const immediatelyBeforeTruncate = await handle.stat();
    if (
      immediatelyBeforeTruncate.nlink !== 1 ||
      !sameIdentity(identityOf(opened), identityOf(immediatelyBeforeTruncate))
    ) {
      throw new Error(`writeBundle: output file link identity changed at ${target}`);
    }

    await handle.truncate(0);
    await handle.writeFile(contents, "utf8");
    await handle.sync();

    const afterWrite = await fs.lstat(target);
    if (
      afterWrite.isSymbolicLink() ||
      afterWrite.nlink !== 1 ||
      !sameIdentity(identityOf(opened), identityOf(afterWrite))
    ) {
      throw new Error(`writeBundle: output file identity changed at ${target}`);
    }
    const afterWriteDirectories = await inspectDirectoryChain(context.anchor, parent);
    assertSameDirectoryChain(beforeDirectories, afterWriteDirectories);
    await verifyPrivateOutputRoot(context);
    context.ownedFiles.set(target, identityOf(afterWrite));
  } finally {
    await handle.close();
  }
}

/**
 * Join `base` and `rel`, then assert the result stays within
 * `base`.  Defends against any rel-path that — after normalisation
 * — escapes upward.  Throws on escape, otherwise returns the
 * joined absolute path.
 */
export function safeJoin(base: string, rel: string): string {
  const resolvedBase = path.resolve(base);
  const target = path.resolve(resolvedBase, rel);
  // Containment check using the resolved-prefix comparison.
  // Append `path.sep` to `base` so that `outDir/foo` doesn't
  // accept an outDir of `outD` (prefix-string false match).
  const baseWithSep = resolvedBase.endsWith(path.sep)
    ? resolvedBase
    : resolvedBase + path.sep;
  if (!(target === resolvedBase || target.startsWith(baseWithSep))) {
    throw new Error(`safeJoin: ${rel} escapes ${base} (resolved to ${target})`);
  }
  return target;
}
