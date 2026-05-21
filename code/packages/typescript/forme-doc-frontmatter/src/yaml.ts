/**
 * yaml.ts — tiny YAML subset parser for docs frontmatter.
 *
 * This is NOT a general-purpose YAML parser.  It covers
 * exactly the subset documentation frontmatter actually uses:
 *
 *   - flat key/value maps (no nested objects)
 *   - scalar values: strings, integers, floats, booleans,
 *     null, dates-as-strings (we don't parse Date objects;
 *     callers can interpret ISO 8601 strings themselves)
 *   - flat arrays of scalars on a single line: `[a, b, c]`
 *   - flat arrays of scalars on multiple lines using `- item`
 *     syntax
 *   - quoted strings (single or double, no escape sequences
 *     beyond `\\` and matching quote)
 *
 * Anything beyond this throws `TypeError`.  Full YAML is a
 * security hazard (the spec includes anchors, custom tags,
 * tag-resolution that some implementations turn into code
 * execution).  This subset is enough for `title: "..."`,
 * `date: 2026-05-20`, `tags: [a, b]`, etc.
 *
 * Defense-in-depth:
 *   - Output object is built via `Object.create(null)` so
 *     `__proto__: bad` as a key cannot pollute
 *     `Object.prototype`.
 *   - Keys `__proto__`, `constructor`, `prototype` rejected.
 *   - No `eval`, no `new Function`, no JSON.parse-via-reviver.
 *
 * @module yaml
 */

const KEY_RE = /^([A-Za-z_][A-Za-z0-9_\-]{0,127})\s*:\s*(.*)$/;
// Reject every Object.prototype method name plus the three obvious
// pollution sinks.  Defense-in-depth: parser output is already a
// null-prototype object so we can't pollute *our* output, but
// callers commonly `Object.assign({}, fm)` into a normal object —
// after which `obj.toString` becomes a string instead of a function
// and code that does `String(obj)` blows up.  Rejecting at parse
// time gives a clear error instead.
const RESERVED_KEYS = new Set([
  "__proto__", "constructor", "prototype",
  "toString", "valueOf", "hasOwnProperty", "isPrototypeOf",
  "propertyIsEnumerable", "toLocaleString",
  "__defineGetter__", "__defineSetter__",
  "__lookupGetter__", "__lookupSetter__",
]);

// Caps to prevent malicious / pathological inputs from exhausting
// memory.  These are generous for real-world docs frontmatter
// (typical: < 1 KB, < 20 keys, < 200-char values) but bounded
// enough that a 100 MB block of dashes can't lock up the process.
const MAX_SOURCE_BYTES = 1024 * 1024;    // 1 MB
const MAX_KEYS = 1000;
const MAX_VALUE_LEN = 64 * 1024;          // 64 KB
const MAX_ERROR_VALUE_LEN = 200;          // truncate quoted strings in error messages

function shorten(s: string): string {
  return s.length > MAX_ERROR_VALUE_LEN ? `${s.slice(0, MAX_ERROR_VALUE_LEN)}…` : s;
}

/**
 * Parse a YAML frontmatter block (the content between `---`
 * delimiters, NOT including the delimiters themselves) into an
 * object.
 *
 * Throws `TypeError` on any input outside the supported
 * subset.
 */
export function parseYaml(source: string): Record<string, unknown> {
  if (source.length > MAX_SOURCE_BYTES) {
    throw new TypeError(
      `forme-doc-frontmatter: YAML source exceeds ${MAX_SOURCE_BYTES}-byte cap (got ${source.length})`,
    );
  }
  // Prototype-free accumulator (defense-in-depth).
  const out: Record<string, unknown> = Object.create(null);
  const lines = source.split("\n");

  let i = 0;
  while (i < lines.length) {
    const rawLine = lines[i]!;
    const line = rawLine.replace(/\s+$/, ""); // rstrip
    // Skip blank lines + comments.
    if (line.length === 0 || /^\s*#/.test(line)) { i++; continue; }
    // Indented lines belong to the previous key (only "- list-item"
    // continuation is supported; anything else is rejected).
    if (/^\s/.test(rawLine) && !/^\s*-\s/.test(rawLine)) {
      throw new TypeError(
        `forme-doc-frontmatter: YAML indented continuation lines not supported (line ${i + 1})`,
      );
    }
    const m = KEY_RE.exec(line);
    if (!m) {
      throw new TypeError(
        `forme-doc-frontmatter: YAML line ${i + 1} is not a "key: value" pair: ${JSON.stringify(shorten(line))}`,
      );
    }
    const key = m[1]!;
    if (RESERVED_KEYS.has(key)) {
      throw new TypeError(
        `forme-doc-frontmatter: YAML key ${JSON.stringify(key)} is a JS prototype-pollution sink name`,
      );
    }
    if (key in out) {
      throw new TypeError(
        `forme-doc-frontmatter: YAML key ${JSON.stringify(key)} duplicated`,
      );
    }
    if (Object.keys(out).length >= MAX_KEYS) {
      throw new TypeError(
        `forme-doc-frontmatter: YAML key count exceeds ${MAX_KEYS}-key cap`,
      );
    }
    const rhs = m[2]!;
    if (rhs.length > MAX_VALUE_LEN) {
      throw new TypeError(
        `forme-doc-frontmatter: YAML value for ${JSON.stringify(key)} exceeds ${MAX_VALUE_LEN}-byte cap`,
      );
    }
    let value: unknown;
    if (rhs === "") {
      // Multi-line list — consume following `  - item` lines.
      const items: unknown[] = [];
      let j = i + 1;
      while (j < lines.length && /^\s*-\s/.test(lines[j]!)) {
        const itemStr = lines[j]!.replace(/^\s*-\s+/, "").trim();
        items.push(parseScalar(itemStr, j + 1));
        j++;
      }
      if (items.length === 0) {
        throw new TypeError(
          `forme-doc-frontmatter: YAML key ${JSON.stringify(key)} has empty value on line ${i + 1} and no list items follow`,
        );
      }
      value = items;
      i = j;
    } else {
      value = parseValue(rhs, i + 1);
      i++;
    }
    out[key] = value;
  }
  return out;
}

function parseValue(rhs: string, lineNo: number): unknown {
  // Inline array `[a, b, c]`?
  if (rhs.startsWith("[") && rhs.endsWith("]")) {
    const inner = rhs.slice(1, -1).trim();
    if (inner.length === 0) return [];
    // Split on commas but not inside quoted strings.
    return splitOutsideQuotes(inner, ",", lineNo).map((s) => parseScalar(s.trim(), lineNo));
  }
  return parseScalar(rhs, lineNo);
}

function parseScalar(s: string, lineNo: number): unknown {
  if (s.length === 0) {
    throw new TypeError(
      `forme-doc-frontmatter: YAML empty scalar on line ${lineNo}`,
    );
  }
  // Quoted strings.
  if ((s.startsWith('"') && s.endsWith('"')) || (s.startsWith("'") && s.endsWith("'"))) {
    const quote = s[0]!;
    const inner = s.slice(1, -1);
    // Reject unescaped same-quote inside.
    let out = "";
    for (let i = 0; i < inner.length; i++) {
      const ch = inner[i]!;
      if (ch === "\\") {
        const next = inner[i + 1];
        if (next === "\\") { out += "\\"; i++; continue; }
        if (next === quote) { out += quote; i++; continue; }
        throw new TypeError(
          `forme-doc-frontmatter: YAML unsupported escape \\${next ?? ""} on line ${lineNo}`,
        );
      }
      if (ch === quote) {
        throw new TypeError(
          `forme-doc-frontmatter: YAML unescaped ${quote} inside ${quote}…${quote} on line ${lineNo}`,
        );
      }
      out += ch;
    }
    return out;
  }
  // Booleans (YAML 1.2: only true / false).
  if (s === "true") return true;
  if (s === "false") return false;
  // null.
  if (s === "null" || s === "~") return null;
  // Integer.
  if (/^-?\d+$/.test(s)) {
    const n = parseInt(s, 10);
    if (!Number.isSafeInteger(n)) {
      throw new TypeError(
        `forme-doc-frontmatter: YAML integer ${JSON.stringify(s)} on line ${lineNo} exceeds safe integer range`,
      );
    }
    return n;
  }
  // Float.
  if (/^-?\d+\.\d+$/.test(s)) {
    return parseFloat(s);
  }
  // Bare scalar — treated as string.  Reject if it looks like
  // a structural character (`{}[]`) that would indicate the
  // caller wanted a nested structure we don't support.
  if (/[{}[\]]/.test(s)) {
    throw new TypeError(
      `forme-doc-frontmatter: YAML bare scalar ${JSON.stringify(shorten(s))} on line ${lineNo} contains structural characters; quote it if it's meant to be a string`,
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
    if (ch === '"' || ch === "'") {
      quote = ch;
      cur += ch;
      continue;
    }
    if (ch === sep) {
      out.push(cur);
      cur = "";
      continue;
    }
    cur += ch;
  }
  if (quote !== null) {
    throw new TypeError(
      `forme-doc-frontmatter: YAML unterminated quoted string in inline list on line ${lineNo}`,
    );
  }
  out.push(cur);
  return out;
}
