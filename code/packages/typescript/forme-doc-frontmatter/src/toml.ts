/**
 * toml.ts — tiny TOML subset parser for docs frontmatter.
 *
 * Same scope as the YAML parser: just enough to cover what
 * doc-page frontmatter uses.  No nested tables, no array of
 * tables, no inline tables, no multi-line strings.
 *
 * Supports:
 *   - bare-key + value lines: `key = value`
 *   - scalar values: integer, float, boolean, basic string
 *     (double-quoted with `\"` `\\` escapes), literal string
 *     (single-quoted)
 *   - dates as strings (we DON'T parse RFC 3339 into Date —
 *     callers do that if they want, but the parser returns
 *     the raw RFC 3339 string)
 *   - inline arrays of scalars: `[a, b, c]`
 *
 * Same `Object.create(null)` + reserved-key rejection as the
 * YAML parser.
 *
 * @module toml
 */

const KEY_RE = /^([A-Za-z_][A-Za-z0-9_\-]{0,127})\s*=\s*(.*)$/;
const DATE_RE = /^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?)?$/;
// See yaml.ts for the rationale on the widened reject list.
const RESERVED_KEYS = new Set([
  "__proto__", "constructor", "prototype",
  "toString", "valueOf", "hasOwnProperty", "isPrototypeOf",
  "propertyIsEnumerable", "toLocaleString",
  "__defineGetter__", "__defineSetter__",
  "__lookupGetter__", "__lookupSetter__",
]);

const MAX_SOURCE_BYTES = 1024 * 1024;
const MAX_KEYS = 1000;
const MAX_VALUE_LEN = 64 * 1024;
const MAX_ERROR_VALUE_LEN = 200;

function shorten(s: string): string {
  return s.length > MAX_ERROR_VALUE_LEN ? `${s.slice(0, MAX_ERROR_VALUE_LEN)}…` : s;
}

export function parseToml(source: string): Record<string, unknown> {
  if (source.length > MAX_SOURCE_BYTES) {
    throw new TypeError(
      `forme-doc-frontmatter: TOML source exceeds ${MAX_SOURCE_BYTES}-byte cap (got ${source.length})`,
    );
  }
  const out: Record<string, unknown> = Object.create(null);
  const lines = source.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const rawLine = lines[i]!;
    const line = rawLine.replace(/^\s+|\s+$/g, "");
    if (line.length === 0 || line.startsWith("#")) continue;
    if (line.startsWith("[")) {
      throw new TypeError(
        `forme-doc-frontmatter: TOML tables / arrays of tables not supported (line ${i + 1})`,
      );
    }
    const m = KEY_RE.exec(line);
    if (!m) {
      throw new TypeError(
        `forme-doc-frontmatter: TOML line ${i + 1} is not a "key = value" pair: ${JSON.stringify(shorten(line))}`,
      );
    }
    const key = m[1]!;
    if (RESERVED_KEYS.has(key)) {
      throw new TypeError(
        `forme-doc-frontmatter: TOML key ${JSON.stringify(key)} is a JS prototype-pollution sink name`,
      );
    }
    if (key in out) {
      throw new TypeError(
        `forme-doc-frontmatter: TOML key ${JSON.stringify(key)} duplicated`,
      );
    }
    if (Object.keys(out).length >= MAX_KEYS) {
      throw new TypeError(
        `forme-doc-frontmatter: TOML key count exceeds ${MAX_KEYS}-key cap`,
      );
    }
    const rhs = m[2]!;
    if (rhs.length > MAX_VALUE_LEN) {
      throw new TypeError(
        `forme-doc-frontmatter: TOML value for ${JSON.stringify(key)} exceeds ${MAX_VALUE_LEN}-byte cap`,
      );
    }
    // Strip inline comment (TOML `# ...` after value, but only
    // when outside a quoted string).
    const trimmedRhs = stripInlineComment(rhs, i + 1);
    out[key] = parseValue(trimmedRhs, i + 1);
  }
  return out;
}

function parseValue(rhs: string, lineNo: number): unknown {
  const s = rhs.trim();
  if (s.length === 0) {
    throw new TypeError(
      `forme-doc-frontmatter: TOML empty value on line ${lineNo}`,
    );
  }
  if (s.startsWith("[") && s.endsWith("]")) {
    const inner = s.slice(1, -1).trim();
    if (inner.length === 0) return [];
    return splitOutsideQuotes(inner, ",", lineNo).map((p) => parseScalar(p.trim(), lineNo));
  }
  return parseScalar(s, lineNo);
}

function parseScalar(s: string, lineNo: number): unknown {
  if (s.startsWith('"') && s.endsWith('"') && s.length >= 2) {
    return parseBasicString(s, lineNo);
  }
  if (s.startsWith("'") && s.endsWith("'") && s.length >= 2) {
    // Literal string — no escapes.
    const inner = s.slice(1, -1);
    if (inner.indexOf("'") !== -1) {
      throw new TypeError(
        `forme-doc-frontmatter: TOML unescaped ' in literal string on line ${lineNo}`,
      );
    }
    return inner;
  }
  if (s === "true") return true;
  if (s === "false") return false;
  // RFC 3339 date — return as string (we don't construct Date).
  if (DATE_RE.test(s)) return s;
  // Integer.
  if (/^[+-]?\d+$/.test(s)) {
    const n = parseInt(s, 10);
    if (!Number.isSafeInteger(n)) {
      throw new TypeError(
        `forme-doc-frontmatter: TOML integer ${JSON.stringify(s)} on line ${lineNo} exceeds safe integer range`,
      );
    }
    return n;
  }
  // Float.
  if (/^[+-]?\d+\.\d+$/.test(s)) return parseFloat(s);
  throw new TypeError(
    `forme-doc-frontmatter: TOML scalar ${JSON.stringify(shorten(s))} on line ${lineNo} not recognised (must be quoted string, integer, float, boolean, or RFC 3339 date)`,
  );
}

function parseBasicString(s: string, lineNo: number): string {
  const inner = s.slice(1, -1);
  let out = "";
  for (let i = 0; i < inner.length; i++) {
    const ch = inner[i]!;
    if (ch === '"') {
      throw new TypeError(
        `forme-doc-frontmatter: TOML unescaped " inside basic string on line ${lineNo}`,
      );
    }
    if (ch === "\\") {
      const next = inner[i + 1];
      if (next === "\\") { out += "\\"; i++; continue; }
      if (next === '"')  { out += '"';  i++; continue; }
      if (next === "n")  { out += "\n"; i++; continue; }
      if (next === "t")  { out += "\t"; i++; continue; }
      if (next === "r")  { out += "\r"; i++; continue; }
      throw new TypeError(
        `forme-doc-frontmatter: TOML unsupported escape \\${next ?? ""} on line ${lineNo}`,
      );
    }
    out += ch;
  }
  return out;
}

function stripInlineComment(s: string, lineNo: number): string {
  let quote: string | null = null;
  for (let i = 0; i < s.length; i++) {
    const ch = s[i]!;
    if (quote !== null) {
      if (ch === "\\" && i + 1 < s.length) { i++; continue; }
      if (ch === quote) quote = null;
      continue;
    }
    if (ch === '"' || ch === "'") { quote = ch; continue; }
    if (ch === "#") return s.slice(0, i);
  }
  if (quote !== null) {
    throw new TypeError(
      `forme-doc-frontmatter: TOML unterminated quoted string on line ${lineNo}`,
    );
  }
  return s;
}

function splitOutsideQuotes(s: string, sep: string, lineNo: number): string[] {
  const out: string[] = [];
  let cur = "";
  let quote: string | null = null;
  for (let i = 0; i < s.length; i++) {
    const ch = s[i]!;
    if (quote !== null) {
      cur += ch;
      if (ch === "\\" && i + 1 < s.length) {
        cur += s[i + 1];
        i++;
        continue;
      }
      if (ch === quote) quote = null;
      continue;
    }
    if (ch === '"' || ch === "'") { quote = ch; cur += ch; continue; }
    if (ch === sep) { out.push(cur); cur = ""; continue; }
    cur += ch;
  }
  if (quote !== null) {
    throw new TypeError(
      `forme-doc-frontmatter: TOML unterminated quoted string in inline list on line ${lineNo}`,
    );
  }
  out.push(cur);
  return out;
}
