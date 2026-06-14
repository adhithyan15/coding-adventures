/**
 * Canonical JSON serialisation per RFC 8785 (JSON Canonicalization Scheme).
 *
 * The goal is "two equal logical values produce byte-identical output."
 * That is what makes content-addressed identifiers (RevisionId) actually
 * stable across runs, machines, and source-format roundtrips.
 *
 * RFC 8785 sums to four rules:
 *
 *   1. **Object keys are sorted.**  Sort in UTF-16 code-unit order — which
 *      is what JavaScript's default String comparison already does.  No
 *      collation, no locale.
 *   2. **No insignificant whitespace.**  No spaces, no newlines, no tabs.
 *   3. **Numbers use the I-JSON / IEEE-754 shortest-form spelling.**
 *      JavaScript's `String(n)` for finite numbers is *almost* this; it
 *      diverges in one place we have to fix (see below).
 *   4. **Strings are UTF-8 encoded.**  We return a JS string and let the
 *      caller hand it to a UTF-8 encoder; the *content* of the string
 *      uses minimal escaping per RFC 8259 §7.
 *
 * === Strings — what we escape ===
 *
 * JSON requires escaping `"`, `\`, and U+0000–U+001F.  Among those, the
 * spec defines short forms (`\b \f \n \r \t \" \\`) and a `\uXXXX`
 * fallback.  We use the short form when one exists; everything else in
 * the control range gets `\u` lower-case-hex (RFC 8785 §3.2.2).  Code
 * points >= U+0020 are emitted as-is — including non-ASCII, including
 * astral surrogate pairs.  This matches every reasonable JSON encoder;
 * the difference between encoders is only in the escape choice.
 *
 * === Numbers — what's tricky ===
 *
 * JavaScript's `String(n)` already produces the IEEE-754 shortest-form
 * representation in most cases (e.g. `0.1` → `"0.1"`, not the full
 * double precision expansion).  But it produces `"1e+21"` instead of
 * the RFC 8785 form `"1e+21"` — they happen to match.  However, for
 * very small numbers it produces `"1e-7"` which RFC 8785 also accepts.
 *
 * The one disagreement: `-0` should serialise to `"0"`, not `"-0"`,
 * per ES2017 JSON.stringify and RFC 7493 / RFC 8785.  We special-case
 * it here.  Likewise NaN and ±Infinity are not valid JSON values —
 * we throw `RangeError` so they don't silently become "null".
 *
 * Integers: any finite number that round-trips through `Number.isInteger`
 * uses its decimal integer form ("42", not "42.0").  Non-integers use
 * the shortest decimal that round-trips, which is what `String(n)` does.
 *
 * === Throws on cycles ===
 *
 * Cyclic structures cannot be canonicalised.  We detect cycles with a
 * WeakSet and throw `TypeError` with a helpful message.  Without this
 * the function would infinite-loop on a hostile input.
 */

import type { JsonValue } from "@coding-adventures/forme-types";

/**
 * Serialise a JsonValue into its RFC 8785 canonical form.
 *
 * Output is a plain JavaScript string.  To get the bytes-on-the-wire
 * RFC 8785 strictly demands, encode the result with a UTF-8 encoder
 * (`new TextEncoder().encode(canonicalJson(...))`).  The hashing helpers
 * in this package handle that step internally.
 *
 * @throws RangeError if any number in the tree is NaN or non-finite.
 * @throws TypeError on a cyclic structure.
 */
export function canonicalJson(value: JsonValue): string {
  return encode(value, new WeakSet());
}

// ─── Recursive encoder ────────────────────────────────────────────────────

function encode(value: JsonValue, seen: WeakSet<object>): string {
  if (value === null) return "null";

  switch (typeof value) {
    case "boolean": return value ? "true" : "false";
    case "number":  return encodeNumber(value);
    case "string":  return encodeString(value);
  }

  if (Array.isArray(value)) {
    if (seen.has(value)) {
      throw new TypeError("canonicalJson: cycle detected at array");
    }
    seen.add(value);
    const parts: string[] = [];
    for (const item of value) parts.push(encode(item, seen));
    seen.delete(value);
    return "[" + parts.join(",") + "]";
  }

  // Plain object.
  const obj = value as { readonly [key: string]: JsonValue };
  if (seen.has(obj)) {
    throw new TypeError("canonicalJson: cycle detected at object");
  }
  seen.add(obj);
  const keys = Object.keys(obj).sort(); // UTF-16 code-unit order
  const parts: string[] = [];
  for (const key of keys) {
    parts.push(encodeString(key) + ":" + encode(obj[key]!, seen));
  }
  seen.delete(obj);
  return "{" + parts.join(",") + "}";
}

// ─── Numbers ──────────────────────────────────────────────────────────────

function encodeNumber(n: number): string {
  if (!Number.isFinite(n)) {
    throw new RangeError(
      `canonicalJson: ${n} is not a valid JSON number ` +
      `(NaN and ±Infinity have no JSON representation)`
    );
  }
  // -0 and +0 must serialise to the same form.
  if (n === 0) return "0";
  // String(n) already gives the IEEE-754 shortest-form spelling
  // for finite doubles — that matches RFC 8785 in every case we
  // care about.  (RFC 8785 says "shortest-decimal IEEE-754 form
  // per ECMA-262 §7.1.12.1," which is exactly what `String(n)` does.)
  return String(n);
}

// ─── Strings ──────────────────────────────────────────────────────────────

function encodeString(s: string): string {
  // Fast-path: scan once; if no character needs escaping we just
  // wrap in quotes.
  if (!hasEscape(s)) return '"' + s + '"';

  let out = '"';
  for (let i = 0; i < s.length; i++) {
    const ch = s.charCodeAt(i);
    switch (ch) {
      case 0x08: out += "\\b"; break;
      case 0x09: out += "\\t"; break;
      case 0x0a: out += "\\n"; break;
      case 0x0c: out += "\\f"; break;
      case 0x0d: out += "\\r"; break;
      case 0x22: out += '\\"'; break;  // "
      case 0x5c: out += "\\\\"; break; // \
      default:
        if (ch < 0x20) {
          // Other control character — \uXXXX with lower-case hex.
          out += "\\u" + ch.toString(16).padStart(4, "0");
        } else {
          out += s[i];
        }
    }
  }
  return out + '"';
}

function hasEscape(s: string): boolean {
  for (let i = 0; i < s.length; i++) {
    const ch = s.charCodeAt(i);
    if (ch < 0x20 || ch === 0x22 /* " */ || ch === 0x5c /* \ */) return true;
  }
  return false;
}
