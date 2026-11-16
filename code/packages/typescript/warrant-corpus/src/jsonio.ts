/**
 * # Bounded, strict JSON reading and canonical writing
 *
 * The corpus is *untrusted input* to the runner (VER04i section 6.2): a stale
 * or hand-edited corpus must fail closed, not be interpreted charitably. So:
 *
 * - files are size-capped **before** they are parsed, and only regular files
 *   are opened (no symlink to somewhere surprising);
 * - duplicate object keys are an error. `JSON.parse` silently keeps the last
 *   one, which is how two readers end up disagreeing about a document;
 * - a number must be a **safe integer**. `JSON.parse` turns `9007199254740993`
 *   into `9007199254740992` without a word; here that is an error, so no count
 *   or counter in the corpus can be a rounded lie. (`U64` *values* never travel
 *   as JSON numbers: they are tagged decimal text parsed with `BigInt`.)
 * - strings must be well-formed Unicode: an unpaired surrogate (`"\\ud800"`) is an
 *   error, as invalid UTF-8 already is;
 * - objects are built with **no prototype**, so a `"__proto__"` key is just
 *   data and cannot reach `Object.prototype`;
 * - output is canonical: keys sorted by code point, two-space indent, LF,
 *   ASCII-only (the same escapes Python's `ensure_ascii` writes, DEL included),
 *   trailing newline, so a regenerated file is byte-identical to the committed
 *   one in every lane.
 *
 * Because the parser is written here, it is also the only place that decides
 * what "JSON" means for the whole runner; nothing else calls `JSON.parse`.
 */

import { createHash } from "node:crypto";
import { closeSync, fstatSync, lstatSync, openSync, readSync } from "node:fs";
import { basename } from "node:path";
import { CorpusError, need } from "./errors.ts";

export const MAX_CORPUS_BYTES = 32 * 1024 * 1024;
export const MAX_TEXT_BYTES = 8 * 1024 * 1024;
/**
 * JSON nesting limit: at most this many nested containers (the next one is
 * refused); scalars do not count. A 64-deep model IR is about 130 levels.
 */
export const MAX_JSON_DEPTH = 256;

/** A parsed JSON value (objects have a null prototype). */
export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

/** Is `value` a JSON object (not an array, not null)? */
export function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Read a regular file of at most `limit` bytes, or throw.
 *
 * The size is checked from `lstat` first (a huge file is refused without
 * being read) and again on what was actually read (the file may have grown).
 */
export function readBytes(path: string, limit: number): Uint8Array {
  const name = basename(path);
  let descriptor: number | undefined;
  try {
    need(lstatSync(path).isFile(), "io", `${name} is not a regular file`);
    descriptor = openSync(path, "r");
    const size = fstatSync(descriptor).size;
    need(size <= limit, "input-limit", `${name} exceeds ${limit} bytes`);
    const buffer = new Uint8Array(Math.min(size, limit) + 1);
    let filled = 0;
    while (filled < buffer.length) {
      const count = readSync(descriptor, buffer, filled, buffer.length - filled, null);
      if (count === 0) break;
      filled += count;
    }
    need(filled <= limit, "input-limit", `${name} exceeds ${limit} bytes`);
    return buffer.slice(0, filled);
  } catch (error) {
    if (error instanceof CorpusError) throw error;
    throw new CorpusError("io", `cannot read ${name}`);
  } finally {
    if (descriptor !== undefined) closeSync(descriptor);
  }
}

/** Lowercase hex SHA-256 of `data`. */
export function sha256Hex(data: Uint8Array): string {
  return createHash("sha256").update(data).digest("hex");
}

/** Decode UTF-8 strictly (no replacement characters, a BOM is kept as data). */
export function decodeUtf8(data: Uint8Array, code = "malformed-json"): string {
  try {
    return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(data);
  } catch {
    throw new CorpusError(code, "input is not valid UTF-8");
  }
}

const NUMBER = /-?(?:0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?/y;
/** A high surrogate not followed by a low one, or a low one not preceded by a high one. */
const UNPAIRED_SURROGATE = /[\ud800-\udbff](?![\udc00-\udfff])|(?<![\ud800-\udbff])[\udc00-\udfff]/;
const ESCAPES: Record<string, string> = {
  '"': '"',
  "\\": "\\",
  "/": "/",
  b: "\b",
  f: "\f",
  n: "\n",
  r: "\r",
  t: "\t",
};

/** A one-pass recursive-descent JSON reader (see the module comment). */
class Reader {
  private at = 0;
  private readonly text: string;

  constructor(text: string) {
    this.text = text;
  }

  parseDocument(): Json {
    const value = this.value(0);
    this.space();
    need(this.at === this.text.length, "malformed-json", "trailing data after the document");
    return value;
  }

  private fail(message: string): never {
    throw new CorpusError("malformed-json", message);
  }

  private space(): void {
    while (this.at < this.text.length && " \t\n\r".includes(this.text.charAt(this.at))) {
      this.at += 1;
    }
  }

  private expect(literal: string): void {
    need(this.text.startsWith(literal, this.at), "malformed-json", "invalid or truncated JSON");
    this.at += literal.length;
  }

  private value(depth: number): Json {
    this.space();
    const char = this.text.charAt(this.at);
    if (char === "{") return this.object(depth);
    if (char === "[") return this.array(depth);
    if (char === '"') return this.string();
    if (char === "t") {
      this.expect("true");
      return true;
    }
    if (char === "f") {
      this.expect("false");
      return false;
    }
    if (char === "n") {
      this.expect("null");
      return null;
    }
    return this.number();
  }

  private number(): number {
    NUMBER.lastIndex = this.at;
    const match = NUMBER.exec(this.text);
    if (match === null) this.fail("invalid or truncated JSON");
    const [literal, fraction, exponent] = match;
    this.at += literal.length;
    need(
      fraction === undefined && exponent === undefined,
      "malformed-json",
      "only integer numbers are allowed",
    );
    const number = Number(literal);
    need(
      Number.isSafeInteger(number) && !Object.is(number, -0),
      "malformed-json",
      "number is not a safe integer",
    );
    return number;
  }

  private string(): string {
    this.at += 1; // the opening quote
    let out = "";
    for (;;) {
      need(this.at < this.text.length, "malformed-json", "unterminated string");
      const char = this.text.charAt(this.at);
      this.at += 1;
      if (char === '"') {
        need(!UNPAIRED_SURROGATE.test(out), "malformed-json", "string has an unpaired surrogate");
        return out;
      }
      need(char >= " ", "malformed-json", "control character in string");
      if (char !== "\\") {
        out += char;
        continue;
      }
      const escape = this.text.charAt(this.at);
      this.at += 1;
      if (escape === "u") {
        const hex = this.text.slice(this.at, this.at + 4);
        need(/^[0-9a-fA-F]{4}$/.test(hex), "malformed-json", "bad \\u escape");
        out += String.fromCharCode(parseInt(hex, 16));
        this.at += 4;
      } else {
        const mapped = ESCAPES[escape];
        need(mapped !== undefined, "malformed-json", "bad escape");
        out += mapped;
      }
    }
  }

  private array(depth: number): Json[] {
    need(depth < MAX_JSON_DEPTH, "malformed-json", "JSON nests too deeply");
    this.at += 1;
    const items: Json[] = [];
    this.space();
    if (this.text.charAt(this.at) === "]") {
      this.at += 1;
      return items;
    }
    for (;;) {
      items.push(this.value(depth + 1));
      this.space();
      const char = this.text.charAt(this.at);
      this.at += 1;
      if (char === "]") return items;
      need(char === ",", "malformed-json", "invalid or truncated JSON");
    }
  }

  private object(depth: number): Json {
    need(depth < MAX_JSON_DEPTH, "malformed-json", "JSON nests too deeply");
    this.at += 1;
    const result: { [key: string]: Json } = Object.create(null) as { [key: string]: Json };
    this.space();
    if (this.text.charAt(this.at) === "}") {
      this.at += 1;
      return result;
    }
    for (;;) {
      this.space();
      need(this.text.charAt(this.at) === '"', "malformed-json", "object key must be a string");
      const key = this.string();
      need(!(key in result), "malformed-json", "duplicate object key");
      this.space();
      need(this.text.charAt(this.at) === ":", "malformed-json", "invalid or truncated JSON");
      this.at += 1;
      result[key] = this.value(depth + 1);
      this.space();
      const char = this.text.charAt(this.at);
      this.at += 1;
      if (char === "}") return result;
      need(char === ",", "malformed-json", "invalid or truncated JSON");
    }
  }
}

/** Parse UTF-8 JSON strictly (see the module comment for what that means). */
export function parseJson(data: Uint8Array): Json {
  return new Reader(decodeUtf8(data)).parseDocument();
}

/** Code point order: UTF-16 code unit order differs for U+E000..U+FFFF versus astral keys. */
function compareCodePoints(left: string, right: string): number {
  const a = Array.from(left, (char) => char.codePointAt(0) as number);
  const b = Array.from(right, (char) => char.codePointAt(0) as number);
  for (let at = 0; at < a.length && at < b.length; at += 1) {
    if (a[at] !== b[at]) return (a[at] as number) < (b[at] as number) ? -1 : 1;
  }
  return a.length - b.length;
}

const SHORT_ESCAPES: Record<string, string> = {
  '"': '\\"',
  "\\": "\\\\",
  "\b": "\\b",
  "\f": "\\f",
  "\n": "\\n",
  "\r": "\\r",
  "\t": "\\t",
};

/** A JSON string literal, ASCII only: everything outside U+0020..U+007E is a lowercase \uXXXX. */
function renderString(text: string): string {
  let out = '"';
  // Iterating UTF-16 code units emits an astral character as its surrogate pair.
  for (let at = 0; at < text.length; at += 1) {
    const char = text.charAt(at);
    const unit = text.charCodeAt(at);
    const short = SHORT_ESCAPES[char];
    if (short !== undefined) out += short;
    else if (unit >= 0x20 && unit <= 0x7e) out += char;
    else out += `\\u${unit.toString(16).padStart(4, "0")}`;
  }
  return `${out}"`;
}

/**
 * Render by hand, never through `JSON.stringify`: a runtime object enumerates
 * integer-like keys ("2" before "10") before every other key whatever order
 * they were inserted in, so the *sorted* order has to be applied to a list.
 */
function renderValue(value: unknown, level: number): string {
  if (value === null) return "null";
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "number") return String(value);
  if (typeof value === "bigint") return value.toString();
  if (typeof value === "string") return renderString(value);
  const pad = "  ".repeat(level);
  const inner = "  ".repeat(level + 1);
  if (Array.isArray(value)) {
    if (value.length === 0) return "[]";
    return `[\n${value.map((item) => inner + renderValue(item, level + 1)).join(",\n")}\n${pad}]`;
  }
  const record = value as Record<string, unknown>;
  const keys = Object.keys(record).sort(compareCodePoints);
  if (keys.length === 0) return "{}";
  const members = keys.map((key) => `${inner}${renderString(key)}: ${renderValue(record[key], level + 1)}`);
  return `{\n${members.join(",\n")}\n${pad}}`;
}

/** Canonical rendering: code point key order, 2-space indent, LF, ASCII, final newline. */
export function renderJson(document: unknown): string {
  return `${renderValue(document, 0)}\n`;
}
