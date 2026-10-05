/**
 * Pure contracts for the FM-B070 browser-quality gate.
 *
 * Lighthouse is deliberately kept outside this module.  The browser process
 * is an effectful evidence provider; score, resource, fallback, containment,
 * and report-publication policy stay small enough to test without Chrome.
 */
import { randomUUID } from "node:crypto";
import { lstat, mkdir, open, realpath, rename, unlink } from "node:fs/promises";
import { dirname, isAbsolute, relative, resolve, sep } from "node:path";
import { Window } from "happy-dom";

export const MAX_DIAGNOSTICS = 20;
const MAX_HTML_BYTES = 1_048_576;
const MAX_AUDIT_REFS = 256;

export interface QualityTarget {
  readonly id: string;
  readonly route: string;
  readonly document: string;
  readonly performanceMinimum: number;
  readonly accessibilityMinimum: number;
  readonly resources: Readonly<Record<string, number>>;
  readonly fallback: {
    readonly requiredSelectors: readonly string[];
    readonly minimumTextCharacters: number;
    readonly minimumLinks: number;
    readonly minimumListItems: number;
    readonly maximumScripts: number;
  };
}

export interface StaticFallbackReport {
  readonly textCharacters: number;
  readonly links: number;
  readonly listItems: number;
  readonly scripts: number;
  readonly diagnostics: readonly string[];
}

export interface LighthouseReport {
  readonly performance: number;
  readonly accessibility: number;
  readonly resources: Readonly<Record<string, number>>;
  readonly diagnostics: readonly string[];
}

export interface StaticMount {
  readonly routePrefix: string;
  readonly root: string;
}

/** Inspect the initial DOM only. No scripts, resources, or custom elements run. */
export function inspectStaticFallback(html: string, target: QualityTarget): StaticFallbackReport {
  if (Buffer.byteLength(html, "utf8") > MAX_HTML_BYTES) {
    throw new Error(`fallback HTML exceeds ${MAX_HTML_BYTES} bytes`);
  }
  validateTarget(target);

  const window = new Window({ url: `http://127.0.0.1${target.route}` });
  try {
    window.document.write(html);
    window.document.close();
    const document = window.document;
    const textCharacters = normalizeText(document.body.textContent ?? "").length;
    const links = document.querySelectorAll("a[href]").length;
    const listItems = document.querySelectorAll("li").length;
    const scripts = document.querySelectorAll("script").length;
    const diagnostics: string[] = [];

    for (const selector of [...target.fallback.requiredSelectors].sort()) {
      let found = false;
      try {
        found = document.querySelector(selector) !== null;
      } catch {
        throw new Error(`fallback selector is invalid: ${selector.slice(0, 128)}`);
      }
      if (!found) diagnostics.push(`fallback missing selector ${selector}`);
    }
    if (textCharacters < target.fallback.minimumTextCharacters) {
      diagnostics.push(
        `fallback text ${textCharacters} < ${target.fallback.minimumTextCharacters} characters`,
      );
    }
    if (links < target.fallback.minimumLinks) {
      diagnostics.push(`fallback links ${links} < ${target.fallback.minimumLinks}`);
    }
    if (listItems < target.fallback.minimumListItems) {
      diagnostics.push(`fallback list items ${listItems} < ${target.fallback.minimumListItems}`);
    }
    if (scripts > target.fallback.maximumScripts) {
      diagnostics.push(`fallback scripts ${scripts} > ${target.fallback.maximumScripts}`);
    }

    return {
      textCharacters,
      links,
      listItems,
      scripts,
      diagnostics: boundDiagnostics(diagnostics),
    };
  } finally {
    window.close();
  }
}

/** Reduce a raw Lighthouse result to the exact bounded release evidence. */
export function evaluateLighthouseResult(target: QualityTarget, raw: unknown): LighthouseReport {
  validateTarget(target);
  const root = record(raw, "Lighthouse result");
  const categories = record(root.categories, "Lighthouse categories");
  const performanceCategory = nullableRecord(categories.performance);
  const accessibilityCategory = nullableRecord(categories.accessibility);
  const performance = score(performanceCategory?.score, "performance");
  const accessibility = score(accessibilityCategory?.score, "accessibility");
  const audits = record(root.audits, "Lighthouse audits");
  const resources = resourceSummary(audits);
  const diagnostics: string[] = [];

  if (accessibility < target.accessibilityMinimum) {
    diagnostics.push(
      `accessibility score ${accessibility} < ${target.accessibilityMinimum}`,
    );
  }
  const auditRefs = Array.isArray(accessibilityCategory?.auditRefs)
    ? accessibilityCategory.auditRefs.slice(0, MAX_AUDIT_REFS)
    : [];
  const failedAuditIds = new Set<string>();
  for (const candidate of auditRefs) {
    const ref = nullableRecord(candidate);
    if (ref === null || typeof ref.id !== "string" || ref.id.length > 128) continue;
    if (typeof ref.weight !== "number" || ref.weight <= 0) continue;
    const audit = nullableRecord(audits[ref.id]);
    if (audit === null || audit.score === 1) continue;
    if (audit.scoreDisplayMode === "manual" || audit.scoreDisplayMode === "notApplicable") continue;
    failedAuditIds.add(ref.id);
  }
  for (const id of [...failedAuditIds].sort()) {
    diagnostics.push(`accessibility audit failed: ${id}`);
  }
  if (performance < target.performanceMinimum) {
    diagnostics.push(`performance score ${performance} < ${target.performanceMinimum}`);
  }
  for (const [resourceType, maximum] of Object.entries(target.resources).sort(([a], [b]) =>
    a.localeCompare(b))) {
    const actual = resources[resourceType];
    if (actual === undefined) {
      diagnostics.push(`resource ${resourceType} is missing from Lighthouse evidence`);
    } else if (actual > maximum) {
      diagnostics.push(`resource ${resourceType} ${actual} > ${maximum} bytes`);
    }
  }

  return {
    performance,
    accessibility,
    resources,
    diagnostics: boundDiagnostics(diagnostics),
  };
}

/** Map one HTTP request into the most-specific generated artifact mount. */
export function resolveStaticRequest(
  method: string,
  rawUrl: string,
  mounts: readonly StaticMount[],
): string {
  if (method !== "GET" && method !== "HEAD") {
    throw new Error("quality server accepts only GET and HEAD");
  }
  const rawPath = rawUrl.split(/[?#]/, 1)[0] ?? "";
  let requestPath: string;
  try {
    requestPath = decodeURIComponent(rawPath);
  } catch {
    throw new Error("quality server request path is not valid UTF-8 percent encoding");
  }
  if (!requestPath.startsWith("/") || requestPath.includes("\0")) {
    throw new Error("quality server request path is invalid");
  }

  const ordered = [...mounts].sort((a, b) => b.routePrefix.length - a.routePrefix.length);
  for (const mount of ordered) {
    if (!mount.routePrefix.startsWith("/") || !mount.routePrefix.endsWith("/")) {
      throw new Error("quality server mount prefixes must be absolute directories");
    }
    if (!requestPath.startsWith(mount.routePrefix)) continue;
    const suffix = requestPath.slice(mount.routePrefix.length) || "index.html";
    const candidate = resolve(mount.root, suffix);
    requireContainedPath(resolve(mount.root), candidate, "quality server request escaped its contained mount");
    return candidate;
  }
  throw new Error("quality server request did not match a configured mount");
}

/** Atomically replace the evidence file without following a target symlink. */
export async function atomicWriteQualitySummary(
  targetPath: string,
  allowedRoot: string,
  text: string,
): Promise<void> {
  const outputDir = dirname(targetPath);
  await mkdir(outputDir, { recursive: true });
  const directory = await lstat(outputDir);
  if (!directory.isDirectory() || directory.isSymbolicLink()) {
    throw new Error("web-quality output directory must be a real directory");
  }
  const canonicalRoot = await realpath(allowedRoot);
  const canonicalOutput = await realpath(outputDir);
  requireContainedPath(canonicalRoot, canonicalOutput, "web-quality output escaped the blog directory");

  const temporaryPath = resolve(outputDir, `.forme-web-quality-${randomUUID()}.tmp`);
  let handle: Awaited<ReturnType<typeof open>> | null = await open(temporaryPath, "wx", 0o600);
  try {
    await handle.writeFile(text, "utf8");
    await handle.sync();
    await handle.close();
    handle = null;
    await rename(temporaryPath, targetPath);
  } finally {
    if (handle !== null) await handle.close().catch(() => {});
    await unlink(temporaryPath).catch(() => {});
  }
}

function validateTarget(target: QualityTarget): void {
  if (!/^[a-z0-9-]{1,64}$/.test(target.id)) throw new Error("quality target id is invalid");
  if (!target.route.startsWith("/") || target.route.length > 512) {
    throw new Error("quality target route is invalid");
  }
  if (target.document.length === 0 || target.document.length > 512) {
    throw new Error("quality target document is invalid");
  }
  for (const [label, value] of [
    ["performance minimum", target.performanceMinimum],
    ["accessibility minimum", target.accessibilityMinimum],
  ] as const) {
    if (!Number.isFinite(value) || value < 0 || value > 1) {
      throw new Error(`quality ${label} must be between zero and one`);
    }
  }
  const resources = Object.entries(target.resources);
  if (resources.length === 0 || resources.length > 16) {
    throw new Error("quality target must define one to sixteen resource budgets");
  }
  for (const [kind, maximum] of resources) {
    if (!/^[a-z-]{1,32}$/.test(kind) || !Number.isSafeInteger(maximum) || maximum < 0) {
      throw new Error("quality resource budget is invalid");
    }
  }
  if (target.fallback.requiredSelectors.length > 16) {
    throw new Error("quality target has too many fallback selectors");
  }
  for (const selector of target.fallback.requiredSelectors) {
    if (selector.length === 0 || selector.length > 128) {
      throw new Error("quality fallback selector is invalid");
    }
  }
  for (const value of [
    target.fallback.minimumTextCharacters,
    target.fallback.minimumLinks,
    target.fallback.minimumListItems,
    target.fallback.maximumScripts,
  ]) {
    if (!Number.isSafeInteger(value) || value < 0) {
      throw new Error("quality fallback count is invalid");
    }
  }
}

function resourceSummary(audits: Record<string, unknown>): Readonly<Record<string, number>> {
  const audit = record(audits["resource-summary"], "Lighthouse resource-summary audit");
  const details = record(audit.details, "Lighthouse resource-summary details");
  if (!Array.isArray(details.items) || details.items.length > 64) {
    throw new Error("Lighthouse resource-summary is missing bounded items");
  }
  const resources: Record<string, number> = {};
  for (const candidate of details.items) {
    const item = record(candidate, "Lighthouse resource-summary item");
    if (typeof item.resourceType !== "string" || !/^[a-z-]{1,32}$/.test(item.resourceType)) {
      throw new Error("Lighthouse resource-summary has an invalid resource type");
    }
    if (!Number.isFinite(item.transferSize) || (item.transferSize as number) < 0) {
      throw new Error(`Lighthouse resource ${item.resourceType} has an invalid transfer size`);
    }
    resources[item.resourceType] = Math.round(item.transferSize as number);
  }
  if (resources.total === undefined) {
    throw new Error("Lighthouse resource-summary is missing total transfer bytes");
  }
  return Object.fromEntries(Object.entries(resources).sort(([a], [b]) => a.localeCompare(b)));
}

function score(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0 || value > 1) {
    throw new Error(`Lighthouse is missing a finite ${label} score between zero and one`);
  }
  return Math.round(value * 100) / 100;
}

function normalizeText(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}

function boundDiagnostics(diagnostics: readonly string[]): readonly string[] {
  if (diagnostics.length <= MAX_DIAGNOSTICS) return diagnostics;
  return [
    ...diagnostics.slice(0, MAX_DIAGNOSTICS - 1),
    `diagnostics truncated after ${MAX_DIAGNOSTICS - 1} entries`,
  ];
}

function record(value: unknown, label: string): Record<string, unknown> {
  const result = nullableRecord(value);
  if (result === null) throw new Error(`${label} must be an object`);
  return result;
}

function nullableRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function requireContainedPath(parent: string, candidate: string, message: string): void {
  const suffix = relative(parent, candidate);
  if (suffix === "" || (!isAbsolute(suffix) && suffix !== ".." && !suffix.startsWith(`..${sep}`))) {
    return;
  }
  throw new Error(message);
}
