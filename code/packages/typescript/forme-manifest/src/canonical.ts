/**
 * canonical.ts — byte-stable TOML serialiser for the manifest hash.
 *
 * The contract:
 *
 *   `canonicalManifestToml(parseManifest(canonicalManifestToml(m)))`
 *     === `canonicalManifestToml(m)` for every valid manifest `m`.
 *
 * In other words: the encoder is a fixed point of the parse/encode
 * loop.  This is what makes `computeManifestHash` meaningful — two
 * manifests with the same fields produce identical bytes regardless
 * of TOML formatting choices in the source file.
 *
 * The serialiser:
 *
 *   - Emits sections in a fixed order: manifestVersion, plugin,
 *     runtime, capabilities, contributes, resources.  The
 *     `[signature]` section is EXCLUDED — that's the document
 *     being signed, not part of it.
 *   - Sorts keys lexicographically inside every table.
 *   - Uses double-quoted strings with minimal JSON-style escapes.
 *   - Emits booleans as `true`/`false`, integers as base-10 with no
 *     sign for non-negative values.
 *   - Emits inline string arrays compactly: `["a", "b", "c"]`.
 *   - Emits each `[[array.of.tables]]` element as a separate header
 *     block, preserving the order in the input array.
 *   - Uses LF line endings exclusively, even on Windows.
 *
 * Output is always UTF-8-ready (the JS string contains only code
 * points that round-trip cleanly through UTF-8).
 *
 * @module canonical
 */

import type {
  Manifest,
  PluginIdentity,
  RuntimeSpec,
  CapabilityBlock,
  CapabilityEntry,
  ContributesBlock,
  StageContribution,
  KindContribution,
  ResourceLimits,
} from "./manifest-types.js";

/**
 * Serialise a manifest to its canonical TOML form.  The output
 * EXCLUDES the `[signature]` section — that's the document being
 * signed.  Callers wanting the full document including signature
 * (for archival, display, etc.) should compose the result with
 * a manually serialised signature block.
 */
export function canonicalManifestToml(manifest: Manifest): string {
  const lines: string[] = [];

  // 1. Top-level scalar(s).
  lines.push(`manifestVersion = ${formatInt(manifest.manifestVersion)}`);
  lines.push("");

  // 2. [plugin]
  appendTable(lines, "plugin", pluginEntries(manifest.plugin));

  // 3. [runtime]
  appendTable(lines, "runtime", runtimeEntries(manifest.runtime));

  // 4. [[capabilities.required]] / [[capabilities.optional]]
  appendCapabilities(lines, manifest.capabilities);

  // 5. [[contributes.stages]] / [[contributes.kinds]]
  appendContributes(lines, manifest.contributes);

  // 6. [resources] (optional)
  if (manifest.resources && Object.keys(manifest.resources).length > 0) {
    appendTable(lines, "resources", resourceEntries(manifest.resources));
  }

  // Drop any trailing blank line so the output is exactly one
  // newline at EOF, matching most editor conventions.
  while (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
  return lines.join("\n") + "\n";
}

// ─── Section emitters ───────────────────────────────────────────────

function appendTable(
  lines: string[],
  header: string,
  entries: ReadonlyArray<readonly [string, string]>,
): void {
  if (entries.length === 0) return;
  lines.push(`[${header}]`);
  // Caller passes entries in pre-sorted order; the helper is
  // intentionally NOT idempotent on key order, to avoid double
  // sorting (which would mask bugs).
  for (const [k, v] of entries) {
    lines.push(`${k} = ${v}`);
  }
  lines.push("");
}

function appendCapabilities(lines: string[], block: CapabilityBlock): void {
  for (const [bucket, entries] of [
    ["required" as const, block.required],
    ["optional" as const, block.optional],
  ]) {
    for (const cap of entries) {
      lines.push(`[[capabilities.${bucket}]]`);
      for (const [k, v] of capabilityEntries(cap)) {
        lines.push(`${k} = ${v}`);
      }
      lines.push("");
    }
  }
}

function appendContributes(lines: string[], block: ContributesBlock): void {
  for (const stage of block.stages) {
    lines.push("[[contributes.stages]]");
    for (const [k, v] of stageEntries(stage)) {
      lines.push(`${k} = ${v}`);
    }
    lines.push("");
  }
  for (const kind of block.kinds) {
    lines.push("[[contributes.kinds]]");
    for (const [k, v] of kindEntries(kind)) {
      lines.push(`${k} = ${v}`);
    }
    lines.push("");
  }
}

// ─── Per-table entry builders ───────────────────────────────────────
//
// Each builder returns a sorted-key entry list ready for the
// emitter.  We sort here (not at emit time) so the per-table
// schema is explicit and reviewable.

function pluginEntries(p: PluginIdentity): ReadonlyArray<readonly [string, string]> {
  const e: Array<[string, string]> = [];
  if (p.authors)     e.push(["authors",     formatStringArray(p.authors)]);
  e.push(["apiVersion",                     formatInt(p.apiVersion)]);
  if (p.description) e.push(["description", formatString(p.description)]);
  if (p.homepage)    e.push(["homepage",    formatString(p.homepage)]);
  if (p.license)     e.push(["license",     formatString(p.license)]);
  e.push(["name",                           formatString(p.name)]);
  if (p.repository)  e.push(["repository",  formatString(p.repository)]);
  e.push(["version",                        formatString(p.version)]);
  return sortByKey(e);
}

function runtimeEntries(r: RuntimeSpec): ReadonlyArray<readonly [string, string]> {
  const e: Array<[string, string]> = [];
  if (r.entry) e.push(["entry", formatString(r.entry)]);
  e.push(["kind", formatString(r.kind)]);
  // platforms is a map; render as an inline table on a single line so
  // the surrounding structure stays a flat [section] (per the v0
  // strict-subset parser, we don't accept TOML inline tables, but we
  // DO accept dotted-key assignments under a parent header — emit
  // those instead).
  if (r.platforms) {
    const keys = Object.keys(r.platforms).sort();
    for (const k of keys) {
      e.push([`platforms.${k}`, formatString(r.platforms[k]!)]);
    }
  }
  return sortByKey(e);
}

function capabilityEntries(c: CapabilityEntry): ReadonlyArray<readonly [string, string]> {
  const e: Array<[string, string]> = [];
  if (c.detail) e.push(["detail", formatString(c.detail)]);
  e.push(["realm",  formatString(c.realm)]);
  e.push(["reason", formatString(c.reason)]);
  e.push(["scope",  formatString(c.scope)]);
  return sortByKey(e);
}

function stageEntries(s: StageContribution): ReadonlyArray<readonly [string, string]> {
  const e: Array<[string, string]> = [];
  if (s.configSchema) e.push(["configSchema", formatString(s.configSchema)]);
  e.push(["consumes", formatString(s.consumes)]);
  e.push(["id",       formatString(s.id)]);
  e.push(["produces", formatString(s.produces)]);
  return sortByKey(e);
}

function kindEntries(k: KindContribution): ReadonlyArray<readonly [string, string]> {
  const e: Array<[string, string]> = [];
  e.push(["name", formatString(k.name)]);
  if (k.schema)    e.push(["schema",    formatString(k.schema)]);
  if (k.subtypeOf) e.push(["subtypeOf", formatString(k.subtypeOf)]);
  e.push(["version", formatString(k.version)]);
  return sortByKey(e);
}

function resourceEntries(r: ResourceLimits): ReadonlyArray<readonly [string, string]> {
  const e: Array<[string, string]> = [];
  const fields: Array<keyof ResourceLimits> = [
    "maxConcurrentRpcs", "maxFileDescriptors", "maxMemoryMb", "maxWallClockMs",
  ];
  for (const f of fields) {
    const v = r[f];
    if (typeof v === "number") e.push([f, formatInt(v)]);
  }
  return sortByKey(e);
}

// ─── Value formatters ───────────────────────────────────────────────

function formatString(s: string): string {
  return '"' + escapeString(s) + '"';
}

function escapeString(s: string): string {
  let out = "";
  for (let i = 0; i < s.length; i++) {
    const ch = s.charCodeAt(i);
    switch (ch) {
      case 0x08: out += "\\b"; break;
      case 0x09: out += "\\t"; break;
      case 0x0a: out += "\\n"; break;
      case 0x0c: out += "\\f"; break;
      case 0x0d: out += "\\r"; break;
      case 0x22: out += '\\"'; break;
      case 0x5c: out += "\\\\"; break;
      default:
        if (ch < 0x20) {
          out += "\\u" + ch.toString(16).padStart(4, "0");
        } else {
          out += s[i];
        }
    }
  }
  return out;
}

function formatInt(n: number): string {
  if (!Number.isFinite(n) || !Number.isInteger(n)) {
    throw new Error(`canonicalManifestToml: value ${n} is not a finite integer`);
  }
  return String(n);
}

function formatStringArray(items: readonly string[]): string {
  return "[" + items.map(formatString).join(", ") + "]";
}

// ─── Helpers ────────────────────────────────────────────────────────

function sortByKey(
  e: ReadonlyArray<readonly [string, string]>,
): ReadonlyArray<readonly [string, string]> {
  return [...e].sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
}
