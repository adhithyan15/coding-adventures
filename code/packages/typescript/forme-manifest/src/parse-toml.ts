/**
 * parse-toml.ts — hand-rolled strict TOML subset parser for plugin.toml.
 *
 * Supports the surface plugin.toml uses (FM02 §3.2) and rejects
 * everything else with a clear `ManifestError`:
 *
 *   - Top-level scalar assignments (`key = value`)
 *   - Dotted keys (`plugin.name = "..."`)
 *   - `[section]` headers, including `[a.b.c]` nesting
 *   - `[[array.of.tables]]` headers (repeated → array of tables)
 *   - String values: single- and double-quoted, with the basic
 *     JSON-ish escapes (`\n`, `\t`, `\r`, `\\`, `\"`, `\'`)
 *   - Integer values (base 10; sign permitted; underscores rejected
 *     for simplicity)
 *   - Boolean values (`true` / `false`)
 *   - Inline arrays of strings, integers, or booleans (homogeneous;
 *     trailing comma permitted; newlines inside `[ ... ]` permitted)
 *   - `#` comments (rest of line)
 *
 * Explicitly NOT supported (throws `TOML_UNSUPPORTED_FEATURE`):
 *
 *   - Multi-line strings (`"""..."""` / `'''...'''`)
 *   - Float values
 *   - Datetime literals (use a quoted string for `signedAt` instead)
 *   - Inline tables (`{ k = v, k2 = v2 }`)
 *   - Heterogeneous arrays
 *
 * The reason for the strict subset: plugin.toml is a security-
 * sensitive document; the parser is in the trust path of every
 * plugin load.  A bigger parser is a bigger audit surface.  When
 * plugin.toml actually needs a feature we don't support today, we'll
 * extend the parser with focused tests — not before.
 *
 * Output: a `Manifest` value (shape-wise; semantic validation is
 * `validate.ts`'s job).  Type-mismatch failures during parse —
 * e.g. `manifestVersion = "1"` instead of `1` — surface as
 * `FIELD_TYPE_MISMATCH` validator errors, not parse errors, so the
 * full document can be reported even when types are wrong.
 *
 * @module parse-toml
 */

import { ManifestError } from "./errors.js";
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
  SignatureBlock,
} from "./manifest-types.js";

/**
 * Parse a TOML manifest string into a `Manifest` value.
 *
 * Throws `ManifestError` with `code: "TOML_*"` on syntax errors.
 * Semantic errors (missing fields, wrong shape) are NOT thrown
 * here — validate the result with `validateManifest`.
 */
export function parseManifest(text: string): Manifest {
  if (typeof text !== "string") {
    throw new ManifestError({
      code: "TOML_MALFORMED",
      message: "parseManifest: input must be a string",
    });
  }
  const parser = new TomlParser(text);
  const raw = parser.parseDocument();
  return buildManifest(raw);
}

// ─── First pass: TOML → generic object tree ─────────────────────────
//
// We parse into an untyped nested-record structure, then translate
// into the strongly-typed `Manifest` interface.  Two passes keeps
// the parser independent of the manifest schema — easier to test
// each layer in isolation.

type TomlValue =
  | string
  | number
  | boolean
  | readonly TomlValue[]
  | { readonly [key: string]: TomlValue };

/**
 * Keys that, if used as TOML bare keys, would let an attacker walk
 * a `target[seg]` bracket access into `Object.prototype` (or
 * `Function.prototype`, etc.) and write to it.  We reject them
 * outright at the segment-parse layer — every internal table is
 * also a null-prototype object via `Object.create(null)`, but the
 * denylist is the cheap belt to the suspenders.
 *
 * Security: without this, a hostile manifest containing
 *
 *     [__proto__]
 *     polluted = "yes"
 *
 * would resolve `target["__proto__"]` to `Object.prototype` and then
 * write `Object.prototype.polluted = "yes"`, polluting the host
 * process's global prototype.  This is exactly the prototype-
 * pollution attack class — caught in the FM02 §3 security review.
 */
const FORBIDDEN_KEY_SEGMENTS = new Set([
  "__proto__",
  "constructor",
  "prototype",
]);

/** Construct a fresh table with NO prototype chain.  Defence in
 *  depth against prototype pollution if the segment denylist is
 *  ever bypassed by future relaxations of the parser. */
function emptyTable(): Record<string, unknown> {
  return Object.create(null) as Record<string, unknown>;
}

class TomlParser {
  private readonly src: string;
  private pos = 0;
  private line = 1;
  private col = 1;
  private root: Record<string, unknown> = emptyTable();
  /** Currently active table for `key = value` assignments. */
  private current: Record<string, unknown> = this.root;
  /** Path leading to `current`, for diagnostics. */
  private currentPath: readonly string[] = [];
  /** Set of paths that have been declared as `[[array]]` tables.
   *  Used to enforce one-array-per-path consistency. */
  private arrayPaths = new Set<string>();
  /** Set of paths already explicitly opened as `[table]`.  Catches
   *  duplicate-section declarations. */
  private declaredSections = new Set<string>();

  constructor(src: string) {
    this.src = src;
  }

  parseDocument(): Record<string, TomlValue> {
    while (!this.eof()) {
      this.skipBlanksAndComments();
      if (this.eof()) break;

      const ch = this.peek();
      if (ch === "[") {
        if (this.peekAt(1) === "[") {
          this.parseArrayOfTablesHeader();
        } else {
          this.parseTableHeader();
        }
      } else {
        this.parseKeyValueLine();
      }
      this.skipInlineWhitespace();
      // After a key = value or [header], we expect EOL or EOF.
      if (!this.eof() && this.peek() !== "\n" && this.peek() !== "#") {
        this.fail("TOML_MALFORMED", `unexpected ${JSON.stringify(this.peek())} after statement`);
      }
      // Consume the rest of the line (newline or comment + newline).
      this.skipInlineWhitespace();
      if (!this.eof() && this.peek() === "#") this.skipToNextLine();
      if (!this.eof() && this.peek() === "\n") this.advance();
    }
    return this.root as Record<string, TomlValue>;
  }

  // ─── Headers ──────────────────────────────────────────────────────

  private parseTableHeader(): void {
    this.expect("[");
    this.skipInlineWhitespace();
    const path = this.parseDottedKey();
    this.skipInlineWhitespace();
    this.expect("]");

    const joined = path.join(".");
    if (this.declaredSections.has(joined)) {
      this.fail("TOML_DUPLICATE_KEY", `section [${joined}] declared more than once`);
    }
    this.declaredSections.add(joined);

    if (this.arrayPaths.has(joined)) {
      this.fail("TOML_DUPLICATE_KEY",
        `cannot redeclare [${joined}] as a table; previously declared as [[${joined}]]`);
    }

    this.current = this.ensureTable(this.root, path, joined);
    this.currentPath = path;
  }

  private parseArrayOfTablesHeader(): void {
    this.expect("[");
    this.expect("[");
    this.skipInlineWhitespace();
    const path = this.parseDottedKey();
    this.skipInlineWhitespace();
    this.expect("]");
    this.expect("]");

    const joined = path.join(".");
    if (this.declaredSections.has(joined)) {
      this.fail("TOML_DUPLICATE_KEY",
        `cannot redeclare [[${joined}]] as an array; previously declared as [${joined}]`);
    }
    this.arrayPaths.add(joined);

    // Walk to the parent and ensure an array exists; push a new empty
    // table; set `current` to that new table.
    const parent = path.length > 1
      ? this.ensureTable(this.root, path.slice(0, -1), path.slice(0, -1).join("."))
      : this.root;
    const last = path[path.length - 1]!;
    let arr = parent[last];
    if (arr === undefined) {
      arr = [];
      parent[last] = arr;
    } else if (!Array.isArray(arr)) {
      this.fail("TOML_DUPLICATE_KEY",
        `cannot redeclare key "${joined}" as array of tables; already has a scalar/table value`);
    }
    const tbl: Record<string, unknown> = emptyTable();
    (arr as unknown[]).push(tbl);
    this.current = tbl;
    this.currentPath = path;
  }

  // ─── Key = value ──────────────────────────────────────────────────

  private parseKeyValueLine(): void {
    const keyPath = this.parseDottedKey();
    this.skipInlineWhitespace();
    this.expect("=");
    this.skipInlineWhitespace();
    const value = this.parseValue();

    // Walk the dotted key into `current`, creating intermediate
    // tables as needed.  The leaf is the actual assignment target.
    let target: Record<string, unknown> = this.current;
    for (let i = 0; i < keyPath.length - 1; i++) {
      const seg = keyPath[i]!;
      const next = target[seg];
      if (next === undefined) {
        const fresh: Record<string, unknown> = emptyTable();
        target[seg] = fresh;
        target = fresh;
      } else if (typeof next === "object" && next !== null && !Array.isArray(next)) {
        target = next as Record<string, unknown>;
      } else {
        this.fail("TOML_DUPLICATE_KEY",
          `cannot descend into "${keyPath.slice(0, i + 1).join(".")}"; ` +
          `it already has a non-table value`);
      }
    }
    const leaf = keyPath[keyPath.length - 1]!;
    if (Object.prototype.hasOwnProperty.call(target, leaf)) {
      this.fail("TOML_DUPLICATE_KEY",
        `key "${[...this.currentPath, ...keyPath].join(".")}" already assigned`);
    }
    target[leaf] = value;
  }

  private parseDottedKey(): string[] {
    const parts: string[] = [];
    while (true) {
      this.skipInlineWhitespace();
      parts.push(this.parseBareKey());
      this.skipInlineWhitespace();
      if (this.peek() === ".") {
        this.advance();
        continue;
      }
      break;
    }
    return parts;
  }

  private parseBareKey(): string {
    const start = this.pos;
    while (!this.eof()) {
      const ch = this.peek();
      if ((ch >= "a" && ch <= "z") ||
          (ch >= "A" && ch <= "Z") ||
          (ch >= "0" && ch <= "9") ||
          ch === "-" || ch === "_") {
        this.advance();
      } else {
        break;
      }
    }
    if (this.pos === start) {
      this.fail("TOML_MALFORMED", `expected a bare key, got ${JSON.stringify(this.peek() ?? "EOF")}`);
    }
    const key = this.src.slice(start, this.pos);
    // Security: reject __proto__, constructor, prototype outright.
    // Combined with `Object.create(null)` for every internal table,
    // this defeats prototype-pollution attacks via crafted manifest
    // keys (see FORBIDDEN_KEY_SEGMENTS comment).
    if (FORBIDDEN_KEY_SEGMENTS.has(key)) {
      this.fail("TOML_MALFORMED",
        `key "${key}" is reserved and cannot appear in a manifest`);
    }
    return key;
  }

  // ─── Values ───────────────────────────────────────────────────────

  private parseValue(): TomlValue {
    if (this.eof()) {
      this.fail("TOML_MALFORMED", "unexpected EOF where a value was expected");
    }
    const ch = this.peek();

    if (ch === '"' || ch === "'") {
      // Reject multi-line strings explicitly.
      if (this.peekAt(1) === ch && this.peekAt(2) === ch) {
        this.fail("TOML_UNSUPPORTED_FEATURE",
          "multi-line strings (triple-quoted) are not supported");
      }
      return this.parseString();
    }
    if (ch === "{") {
      this.fail("TOML_UNSUPPORTED_FEATURE", "inline tables `{ ... }` are not supported");
    }
    if (ch === "[") {
      return this.parseArray();
    }
    if (ch === "t" || ch === "f") {
      return this.parseBoolean();
    }
    if (ch === "-" || ch === "+" || (ch >= "0" && ch <= "9")) {
      return this.parseNumber();
    }
    this.fail("TOML_MALFORMED",
      `expected a value (string, integer, boolean, or array); got ${JSON.stringify(ch)}`);
  }

  private parseString(): string {
    const quote = this.peek();
    this.advance(); // opening quote
    let out = "";
    while (true) {
      if (this.eof() || this.peek() === "\n") {
        this.fail("TOML_UNTERMINATED_STRING", "string is not closed before end of line");
      }
      const ch = this.peek();
      if (ch === quote) {
        this.advance();
        return out;
      }
      if (ch === "\\" && quote === '"') {
        // Double-quoted: process escapes.
        this.advance();
        if (this.eof()) {
          this.fail("TOML_UNTERMINATED_STRING", "trailing backslash with no escape");
        }
        const esc = this.peek();
        this.advance();
        switch (esc) {
          case "n":  out += "\n"; break;
          case "t":  out += "\t"; break;
          case "r":  out += "\r"; break;
          case "\\": out += "\\"; break;
          case '"':  out += '"';  break;
          case "0":  out += "\0"; break;
          case "u": {
            // \uXXXX — exactly 4 hex chars.
            let hex = "";
            for (let i = 0; i < 4; i++) {
              if (this.eof() || !/[0-9a-fA-F]/.test(this.peek())) {
                this.fail("TOML_INVALID_ESCAPE",
                  "\\u escape requires 4 hexadecimal digits");
              }
              hex += this.peek();
              this.advance();
            }
            out += String.fromCharCode(parseInt(hex, 16));
            break;
          }
          default:
            this.fail("TOML_INVALID_ESCAPE",
              `unsupported escape \\${esc}; recognised: \\n \\t \\r \\\\ \\" \\u`);
        }
      } else {
        // Single-quoted strings (literal) don't process escapes at all.
        out += ch;
        this.advance();
      }
    }
  }

  private parseBoolean(): boolean {
    if (this.matchLiteral("true")) return true;
    if (this.matchLiteral("false")) return false;
    this.fail("TOML_MALFORMED",
      `expected boolean (true/false); got ${JSON.stringify(this.peekIdent())}`);
  }

  /**
   * Numeric literal.  We accept signed base-10 integers only.  Floats,
   * hex/oct/bin literals, underscores, and dates are deliberately
   * unsupported — TOML's number flexibility is overkill for plugin
   * manifests.
   */
  private parseNumber(): number {
    const start = this.pos;
    if (this.peek() === "+" || this.peek() === "-") this.advance();
    if (this.eof() || this.peek() < "0" || this.peek() > "9") {
      this.fail("TOML_INVALID_INTEGER",
        "expected a decimal digit after sign");
    }
    while (!this.eof() && this.peek() >= "0" && this.peek() <= "9") {
      this.advance();
    }
    // Reject floats and exponents explicitly so callers see a helpful
    // error instead of a generic "unexpected character".
    if (!this.eof() && (this.peek() === "." || this.peek() === "e" || this.peek() === "E")) {
      this.fail("TOML_UNSUPPORTED_FEATURE",
        "floating-point and exponent literals are not supported in manifests");
    }
    if (!this.eof() && this.peek() === "_") {
      this.fail("TOML_INVALID_INTEGER",
        "underscore digit separators are not supported");
    }
    const text = this.src.slice(start, this.pos);
    const n = parseInt(text, 10);
    if (!Number.isFinite(n)) {
      this.fail("TOML_INVALID_INTEGER", `could not parse ${JSON.stringify(text)} as an integer`);
    }
    return n;
  }

  private parseArray(): readonly TomlValue[] {
    this.expect("[");
    const out: TomlValue[] = [];
    let elemType: string | null = null;
    while (true) {
      this.skipBlanksAndComments();
      if (this.eof()) {
        this.fail("TOML_INVALID_ARRAY", "unterminated array");
      }
      if (this.peek() === "]") {
        this.advance();
        return out;
      }
      const v = this.parseValue();
      const t = typeofToml(v);
      if (elemType === null) elemType = t;
      else if (elemType !== t) {
        this.fail("TOML_INVALID_ARRAY",
          `heterogeneous arrays are not supported; first element was ${elemType}, got ${t}`);
      }
      out.push(v);
      this.skipBlanksAndComments();
      if (this.eof()) {
        this.fail("TOML_INVALID_ARRAY", "unterminated array");
      }
      if (this.peek() === ",") {
        this.advance();
        continue;
      }
      if (this.peek() === "]") {
        this.advance();
        return out;
      }
      this.fail("TOML_INVALID_ARRAY",
        `expected ',' or ']' in array; got ${JSON.stringify(this.peek())}`);
    }
  }

  // ─── Whitespace, comments, eof ────────────────────────────────────

  private skipInlineWhitespace(): void {
    while (!this.eof() && (this.peek() === " " || this.peek() === "\t")) {
      this.advance();
    }
  }

  private skipBlanksAndComments(): void {
    while (!this.eof()) {
      const ch = this.peek();
      if (ch === " " || ch === "\t" || ch === "\n") {
        this.advance();
      } else if (ch === "#") {
        this.skipToNextLine();
      } else {
        return;
      }
    }
  }

  private skipToNextLine(): void {
    while (!this.eof() && this.peek() !== "\n") this.advance();
  }

  // ─── Primitive ops ────────────────────────────────────────────────

  private peek(): string {
    return this.src[this.pos] ?? "";
  }

  private peekAt(offset: number): string {
    return this.src[this.pos + offset] ?? "";
  }

  private peekIdent(): string {
    let i = this.pos;
    while (i < this.src.length && /[A-Za-z0-9_]/.test(this.src[i]!)) i++;
    return this.src.slice(this.pos, i);
  }

  private advance(): void {
    const ch = this.src[this.pos];
    this.pos++;
    if (ch === "\n") {
      this.line++;
      this.col = 1;
    } else {
      this.col++;
    }
  }

  private eof(): boolean {
    return this.pos >= this.src.length;
  }

  private expect(s: string): void {
    if (this.src.startsWith(s, this.pos)) {
      for (let i = 0; i < s.length; i++) this.advance();
      return;
    }
    this.fail("TOML_MALFORMED",
      `expected ${JSON.stringify(s)}; got ${JSON.stringify(this.peek() ?? "EOF")}`);
  }

  private matchLiteral(s: string): boolean {
    if (this.src.startsWith(s, this.pos)) {
      // Ensure we're not in the middle of a bigger identifier.
      const after = this.src[this.pos + s.length] ?? "";
      if (after && /[A-Za-z0-9_]/.test(after)) return false;
      for (let i = 0; i < s.length; i++) this.advance();
      return true;
    }
    return false;
  }

  private ensureTable(
    root: Record<string, unknown>,
    path: readonly string[],
    joinedForDiagnostic: string,
  ): Record<string, unknown> {
    let cur: Record<string, unknown> = root;
    for (let i = 0; i < path.length; i++) {
      const seg = path[i]!;
      const next = cur[seg];
      if (next === undefined) {
        const fresh: Record<string, unknown> = {};
        cur[seg] = fresh;
        cur = fresh;
      } else if (Array.isArray(next)) {
        // Walk into the LAST element of the array-of-tables (so
        // `[[foo]]` then `[[foo.bar]]` lands in the most recent foo).
        const last = next[next.length - 1];
        if (last === undefined || typeof last !== "object") {
          this.fail("TOML_MALFORMED",
            `cannot descend into array of tables at ${joinedForDiagnostic}`);
        }
        cur = last as Record<string, unknown>;
      } else if (typeof next === "object" && next !== null) {
        cur = next as Record<string, unknown>;
      } else {
        this.fail("TOML_DUPLICATE_KEY",
          `cannot redeclare "${path.slice(0, i + 1).join(".")}" as a table; ` +
          `already has a scalar value`);
      }
    }
    return cur;
  }

  private fail(code: import("./errors.js").ManifestErrorCode, message: string): never {
    throw new ManifestError({
      code,
      message: `${message} (at line ${this.line}, column ${this.col})`,
      path: this.currentPath.join("."),
    });
  }
}

function typeofToml(v: TomlValue): string {
  if (typeof v === "string") return "string";
  if (typeof v === "boolean") return "boolean";
  if (typeof v === "number") return "integer";
  if (Array.isArray(v)) return "array";
  return "table";
}

// ─── Second pass: generic tree → Manifest ───────────────────────────

function buildManifest(raw: Record<string, TomlValue>): Manifest {
  const manifestVersion = (raw["manifestVersion"] as number | undefined) ?? 0;
  const plugin = buildPluginIdentity(asTable(raw["plugin"]));
  const runtime = buildRuntimeSpec(asTable(raw["runtime"]));
  const capabilities = buildCapabilities(asTable(raw["capabilities"]));
  const contributes = buildContributes(asTable(raw["contributes"]));
  const resources = raw["resources"]
    ? buildResourceLimits(asTable(raw["resources"]))
    : undefined;
  const signature = raw["signature"]
    ? buildSignatureBlock(asTable(raw["signature"]))
    : undefined;

  return {
    manifestVersion,
    plugin,
    runtime,
    capabilities,
    contributes,
    ...(resources !== undefined ? { resources } : {}),
    ...(signature !== undefined ? { signature } : {}),
  };
}

function asTable(v: unknown): Record<string, unknown> {
  if (v === undefined || v === null) return {};
  if (typeof v !== "object" || Array.isArray(v)) return {};
  return v as Record<string, unknown>;
}

function asString(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
}

function asNumber(v: unknown): number | undefined {
  return typeof v === "number" ? v : undefined;
}

function asStringArray(v: unknown): readonly string[] | undefined {
  if (!Array.isArray(v)) return undefined;
  if (!v.every((item) => typeof item === "string")) return undefined;
  return v as readonly string[];
}

function asTableArray(v: unknown): readonly Record<string, unknown>[] {
  if (!Array.isArray(v)) return [];
  const out: Record<string, unknown>[] = [];
  for (const item of v) {
    if (typeof item === "object" && item !== null && !Array.isArray(item)) {
      out.push(item as Record<string, unknown>);
    }
  }
  return out;
}

function buildPluginIdentity(raw: Record<string, unknown>): PluginIdentity {
  // We intentionally pass `undefined` through on missing/wrong-type fields
  // so the validator can catch them with a precise path.
  const name        = asString(raw["name"]);
  const version     = asString(raw["version"]);
  const apiVersion  = asNumber(raw["apiVersion"]);
  return {
    name:        (name        ?? "") as string,
    version:     (version     ?? "") as string,
    apiVersion:  (apiVersion  ?? 0) as number,
    ...(asString(raw["description"]) !== undefined ? { description: asString(raw["description"])! } : {}),
    ...(asString(raw["license"])     !== undefined ? { license:     asString(raw["license"])!     } : {}),
    ...(asStringArray(raw["authors"]) !== undefined ? { authors:    asStringArray(raw["authors"])! } : {}),
    ...(asString(raw["homepage"])    !== undefined ? { homepage:    asString(raw["homepage"])!    } : {}),
    ...(asString(raw["repository"])  !== undefined ? { repository:  asString(raw["repository"])!  } : {}),
  };
}

function buildRuntimeSpec(raw: Record<string, unknown>): RuntimeSpec {
  const kind = asString(raw["kind"]) ?? "";
  const entry = asString(raw["entry"]) ?? "";
  const platformsRaw = raw["platforms"];
  let platforms: Record<string, string> | undefined;
  if (typeof platformsRaw === "object" && platformsRaw !== null && !Array.isArray(platformsRaw)) {
    const p: Record<string, string> = {};
    for (const [k, v] of Object.entries(platformsRaw)) {
      if (typeof v === "string") p[k] = v;
    }
    platforms = p;
  }
  return {
    kind: kind as RuntimeSpec["kind"],
    entry,
    ...(platforms !== undefined ? { platforms } : {}),
  };
}

function buildCapabilities(raw: Record<string, unknown>): CapabilityBlock {
  const required = asTableArray(raw["required"]).map(buildCapabilityEntry);
  const optional = asTableArray(raw["optional"]).map(buildCapabilityEntry);
  return { required, optional };
}

function buildCapabilityEntry(raw: Record<string, unknown>): CapabilityEntry {
  return {
    realm:  asString(raw["realm"]) ?? "",
    scope:  asString(raw["scope"]) ?? "",
    ...(asString(raw["detail"]) !== undefined ? { detail: asString(raw["detail"])! } : {}),
    reason: asString(raw["reason"]) ?? "",
  };
}

function buildContributes(raw: Record<string, unknown>): ContributesBlock {
  const stages = asTableArray(raw["stages"]).map(buildStageContribution);
  const kinds  = asTableArray(raw["kinds"]).map(buildKindContribution);
  return { stages, kinds };
}

function buildStageContribution(raw: Record<string, unknown>): StageContribution {
  return {
    id:       asString(raw["id"])       ?? "",
    consumes: asString(raw["consumes"]) ?? "",
    produces: asString(raw["produces"]) ?? "",
    ...(asString(raw["configSchema"]) !== undefined ? { configSchema: asString(raw["configSchema"])! } : {}),
  };
}

function buildKindContribution(raw: Record<string, unknown>): KindContribution {
  return {
    name:    asString(raw["name"])    ?? "",
    version: asString(raw["version"]) ?? "",
    ...(asString(raw["schema"])    !== undefined ? { schema:    asString(raw["schema"])!    } : {}),
    ...(asString(raw["subtypeOf"]) !== undefined ? { subtypeOf: asString(raw["subtypeOf"])! } : {}),
  };
}

function buildResourceLimits(raw: Record<string, unknown>): ResourceLimits {
  const out: { -readonly [K in keyof ResourceLimits]?: number } = {};
  const mm = asNumber(raw["maxMemoryMb"]);
  const mw = asNumber(raw["maxWallClockMs"]);
  const mf = asNumber(raw["maxFileDescriptors"]);
  const mc = asNumber(raw["maxConcurrentRpcs"]);
  if (mm !== undefined) out.maxMemoryMb = mm;
  if (mw !== undefined) out.maxWallClockMs = mw;
  if (mf !== undefined) out.maxFileDescriptors = mf;
  if (mc !== undefined) out.maxConcurrentRpcs = mc;
  return out;
}

function buildSignatureBlock(raw: Record<string, unknown>): SignatureBlock {
  return {
    algorithm: asString(raw["algorithm"]) ?? "",
    publicKey: asString(raw["publicKey"]) ?? "",
    signature: asString(raw["signature"]) ?? "",
    signedAt:  asString(raw["signedAt"])  ?? "",
  };
}
