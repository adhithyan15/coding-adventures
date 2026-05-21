/**
 * toml.ts — docs-frontmatter TOML adapter.
 *
 * =============================================================================
 * WHY THIS FILE EXISTS
 * =============================================================================
 *
 * The repo already ships a full-fidelity TOML 1.0 parser
 * (`@coding-adventures/toml-parser`, ~12 grammar rules covering the entire
 * spec).  This file is a **thin adapter** that:
 *
 *   1. Hands the source text to `parseTOML`, getting back an `ASTNode` tree.
 *   2. Walks that tree and enforces the docs-frontmatter subset.
 *   3. Returns a flat `Record<string, unknown>` suitable for downstream
 *      site-structure packages (sidebar position, draft flag, etc.).
 *
 * We could just expose the full parser, but docs frontmatter has a
 * **deliberately narrow contract** — same as the YAML side of this package.
 * If someone slips a `[server]` table or an inline `{x = 1, y = 2}` into
 * their page header, they probably typed the wrong delimiter.  Failing
 * loudly at parse time is friendlier than silently producing nested
 * objects that downstream site code never expects.
 *
 * =============================================================================
 * SUPPORTED SUBSET
 * =============================================================================
 *
 *   ✓ `key = value` lines with bare keys (`[A-Za-z_][A-Za-z0-9_\-]{0,127}`)
 *   ✓ Scalars: BASIC_STRING, ML_BASIC_STRING, LITERAL_STRING,
 *     ML_LITERAL_STRING, INTEGER (dec/hex/oct/bin with underscores),
 *     FLOAT (decimal/scientific/inf/nan), TRUE, FALSE, OFFSET_DATETIME,
 *     LOCAL_DATETIME, LOCAL_DATE, LOCAL_TIME (datetime tokens kept as their
 *     RFC 3339 string — we never construct a `Date`)
 *   ✓ Single-line and multi-line arrays of scalars
 *   ✓ Inline `# comments` after values (the toml-lexer strips them)
 *   ✓ Blank lines and comment lines between expressions
 *
 *   ✗ Table headers (`[server]`)
 *   ✗ Array-of-tables headers (`[[products]]`)
 *   ✗ Dotted keys (`a.b.c = 1`)
 *   ✗ Quoted keys (`"127.0.0.1" = value`)
 *   ✗ Inline tables (`{x = 1, y = 2}`)
 *   ✗ Arrays-of-arrays / arrays-of-tables
 *
 * All rejected constructs throw `TypeError` with a `forme-doc-frontmatter:`
 * prefix.
 *
 * =============================================================================
 * SECURITY POSTURE
 * =============================================================================
 *
 *   - Output object built via `Object.create(null)` — no prototype chain,
 *     so `__proto__: bad` as a key cannot mutate `Object.prototype`.
 *   - Reserved-key list rejects every `Object.prototype` method name plus
 *     `__proto__` / `constructor` / `prototype`.  See the YAML parser for
 *     the rationale on the widened list.
 *   - Size caps prevent pathological inputs from exhausting memory:
 *     1 MB source, 1000 keys, 64 KB per value.
 *   - No `eval`, no `new Function`, no `JSON.parse` reviver.
 *   - Capabilities: `[]` — `@coding-adventures/toml-parser` v0.1.1+ is
 *     pure-transform (precompiled grammar, no fs:read), so this package
 *     stays pure-transform too.
 *
 * =============================================================================
 * AST SHAPE (FROM toml-parser)
 * =============================================================================
 *
 * Parsing `title = "Hello"\ndraft = false` yields roughly:
 *
 *     document {
 *       children: [
 *         expression {
 *           children: [keyval {
 *             children: [
 *               key { children: [simple_key { children: [BARE_KEY("title")] }] },
 *               EQUALS,
 *               value { children: [BASIC_STRING("Hello")] }
 *             ]
 *           }]
 *         },
 *         NEWLINE,
 *         expression { … draft = false … }
 *       ]
 *     }
 *
 * The walker pattern-matches `ruleName` at each layer.  Anything that
 * doesn't fit the subset throws.
 *
 * @module toml
 */

import { parseTOML } from "@coding-adventures/toml-parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

// ─────────────────────────────────────────────────────────────────────────
// Constants — caps and rejection lists
// ─────────────────────────────────────────────────────────────────────────

const BARE_KEY_RE = /^[A-Za-z_][A-Za-z0-9_\-]{0,127}$/;

// See yaml.ts for the rationale on the widened reject list.  Briefly:
// the output object is null-prototype, so we cannot pollute *our* output,
// but callers commonly `Object.assign({}, fm)` into a normal object — at
// which point `obj.toString` becomes a string instead of a function and
// any `String(obj)` blows up.  Rejecting at parse time gives a crisp error
// instead.
const RESERVED_KEYS = new Set<string>([
  "__proto__", "constructor", "prototype",
  "toString", "valueOf", "hasOwnProperty", "isPrototypeOf",
  "propertyIsEnumerable", "toLocaleString",
  "__defineGetter__", "__defineSetter__",
  "__lookupGetter__", "__lookupSetter__",
]);

const MAX_SOURCE_BYTES = 1024 * 1024; // 1 MB
const MAX_KEYS = 1000;
const MAX_VALUE_LEN = 64 * 1024;       // 64 KB per scalar token text
const MAX_ERROR_VALUE_LEN = 200;

function shorten(s: string): string {
  return s.length > MAX_ERROR_VALUE_LEN ? `${s.slice(0, MAX_ERROR_VALUE_LEN)}…` : s;
}

// ─────────────────────────────────────────────────────────────────────────
// Type guards
// ─────────────────────────────────────────────────────────────────────────

/**
 * Discriminate ASTNode from Token inside an `ASTNode.children` array.
 * Tokens have `type` but not `ruleName`; ASTNodes always have `ruleName`.
 */
function isASTNode(child: ASTNode | Token): child is ASTNode {
  return typeof (child as ASTNode).ruleName === "string";
}

// ─────────────────────────────────────────────────────────────────────────
// Public entry
// ─────────────────────────────────────────────────────────────────────────

/**
 * Parse a TOML frontmatter block (the content between `+++` delimiters,
 * not including the delimiters themselves) into a flat object.
 *
 * Throws `TypeError` on any input outside the supported subset.
 */
export function parseToml(source: string): Record<string, unknown> {
  // Cheap upfront cap — fail fast before we hand a giant string to the
  // lexer or rely on the per-value cap to catch it.
  if (source.length > MAX_SOURCE_BYTES) {
    throw new TypeError(
      `forme-doc-frontmatter: TOML source exceeds ${MAX_SOURCE_BYTES}-byte cap (got ${source.length})`,
    );
  }

  // Delegate full lex+parse to @coding-adventures/toml-parser.  Any
  // surface-syntax error (unbalanced brackets, bad escape sequences,
  // unterminated strings, etc.) surfaces here as either ParseError or
  // a lexer error — we let it propagate unwrapped because its message
  // already includes line/column and is clearer than anything we'd add.
  const document = parseTOML(source);
  return walkDocument(document);
}

// ─────────────────────────────────────────────────────────────────────────
// Walker — one function per grammar rule we care about
// ─────────────────────────────────────────────────────────────────────────

/**
 * Walk the top-level `document` node.  Its children are an interleaved
 * sequence of `NEWLINE` tokens and `expression` AST nodes (see toml.grammar).
 */
function walkDocument(doc: ASTNode): Record<string, unknown> {
  /* v8 ignore start */
  // Unreachable: parseTOML always returns a "document" node (top-level rule).
  if (doc.ruleName !== "document") {
    throw new TypeError(
      `forme-doc-frontmatter: expected top-level "document" node, got ${JSON.stringify(doc.ruleName)}`,
    );
  }
  /* v8 ignore stop */
  const out: Record<string, unknown> = Object.create(null);
  for (const child of doc.children) {
    // NEWLINE tokens are noise at this level — they delimit expressions.
    if (!isASTNode(child)) continue;
    /* v8 ignore start */
    // Unreachable: `document = { NEWLINE | expression }` — only those two.
    if (child.ruleName !== "expression") {
      throw new TypeError(
        `forme-doc-frontmatter: unexpected node ${JSON.stringify(child.ruleName)} in TOML document`,
      );
    }
    /* v8 ignore stop */
    walkExpression(child, out);
  }
  return out;
}

/**
 * Walk an `expression` node.  Per the grammar, expression =
 * array_table_header | table_header | keyval.  We reject the two table
 * forms outright and process the keyval into `out`.
 */
function walkExpression(expr: ASTNode, out: Record<string, unknown>): void {
  // Find the inner AST node (skip any stray tokens — there shouldn't be any).
  const inner = expr.children.find(isASTNode);
  /* v8 ignore start */
  // Unreachable: `expression = array_table_header | table_header | keyval`.
  if (!inner) {
    throw new TypeError(
      `forme-doc-frontmatter: empty TOML expression node`,
    );
  }
  /* v8 ignore stop */
  if (inner.ruleName === "table_header") {
    throw new TypeError(
      `forme-doc-frontmatter: TOML tables ([section]) are not supported in docs frontmatter`,
    );
  }
  if (inner.ruleName === "array_table_header") {
    throw new TypeError(
      `forme-doc-frontmatter: TOML arrays-of-tables ([[section]]) are not supported in docs frontmatter`,
    );
  }
  /* v8 ignore start */
  // Unreachable: alternation above is exhaustive.
  if (inner.ruleName !== "keyval") {
    throw new TypeError(
      `forme-doc-frontmatter: unexpected expression node ${JSON.stringify(inner.ruleName)}`,
    );
  }
  /* v8 ignore stop */

  // keyval = key EQUALS value
  const children = inner.children;
  const keyNode = children[0];
  const valueNode = children[2];
  /* v8 ignore start */
  // Unreachable: `keyval = key EQUALS value` guarantees both ASTNodes at [0]/[2].
  if (!keyNode || !valueNode || !isASTNode(keyNode) || !isASTNode(valueNode)) {
    throw new TypeError(
      `forme-doc-frontmatter: malformed TOML keyval node`,
    );
  }
  /* v8 ignore stop */

  const key = walkKey(keyNode);

  if (RESERVED_KEYS.has(key)) {
    throw new TypeError(
      `forme-doc-frontmatter: TOML key ${JSON.stringify(key)} is a JS prototype-pollution sink name`,
    );
  }
  if (!BARE_KEY_RE.test(key)) {
    throw new TypeError(
      `forme-doc-frontmatter: TOML key ${JSON.stringify(shorten(key))} does not match the docs-frontmatter bare-key pattern (letters/digits/_/- starting with letter or _, max 128 chars)`,
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

  out[key] = walkValue(valueNode, key);
}

/**
 * Walk a `key` node.  Grammar: key = simple_key { DOT simple_key }.
 * The docs subset only accepts a single bare simple_key — multi-part
 * dotted keys and quoted simple_keys are rejected.
 */
function walkKey(keyNode: ASTNode): string {
  const simpleKeys = keyNode.children.filter(isASTNode);
  /* v8 ignore start */
  // Unreachable: `key = simple_key { DOT simple_key }` requires ≥1.
  if (simpleKeys.length === 0) {
    throw new TypeError(
      `forme-doc-frontmatter: empty TOML key`,
    );
  }
  /* v8 ignore stop */
  if (simpleKeys.length > 1) {
    throw new TypeError(
      `forme-doc-frontmatter: TOML dotted keys (a.b.c) are not supported in docs frontmatter`,
    );
  }
  const sk = simpleKeys[0]!;
  // simple_key wraps exactly one token (per the grammar alternation).
  const tok = sk.children[0];
  /* v8 ignore start */
  // Unreachable: simple_key alternation always yields exactly one Token.
  if (!tok || isASTNode(tok)) {
    throw new TypeError(
      `forme-doc-frontmatter: malformed TOML simple_key node`,
    );
  }
  /* v8 ignore stop */
  if (tok.type !== "BARE_KEY") {
    // Includes BASIC_STRING / LITERAL_STRING (quoted keys) and any of the
    // value-token-as-key alternatives (TRUE, INTEGER, LOCAL_DATE, …).
    throw new TypeError(
      `forme-doc-frontmatter: TOML non-bare key (${tok.type}) is not supported in docs frontmatter`,
    );
  }
  return tok.value;
}

/**
 * Walk a `value` node.  Its first child is either a scalar token or a
 * compound AST node (`array` | `inline_table`).  Inline tables are
 * rejected outright; arrays recurse but reject non-scalar elements.
 */
function walkValue(valueNode: ASTNode, key: string): unknown {
  const first = valueNode.children[0];
  /* v8 ignore start */
  // Unreachable: `value` alternation always produces at least one element.
  if (!first) {
    throw new TypeError(
      `forme-doc-frontmatter: empty TOML value for key ${JSON.stringify(key)}`,
    );
  }
  /* v8 ignore stop */
  if (isASTNode(first)) {
    if (first.ruleName === "inline_table") {
      throw new TypeError(
        `forme-doc-frontmatter: TOML inline tables ({x = 1, y = 2}) are not supported in docs frontmatter`,
      );
    }
    if (first.ruleName === "array") {
      return walkArray(first, key);
    }
    /* v8 ignore start */
    // Unreachable: value's compound alternatives are only array / inline_table.
    throw new TypeError(
      `forme-doc-frontmatter: unexpected TOML value node ${JSON.stringify(first.ruleName)}`,
    );
    /* v8 ignore stop */
  }
  return decodeScalarToken(first, key);
}

/**
 * Walk an `array` node.  Grammar: array = LBRACKET array_values RBRACKET.
 * `array_values` is a flat sequence of `value` nodes interleaved with
 * NEWLINE and COMMA tokens.  We only keep the `value` children and
 * recurse — rejecting any nested array / inline_table because the docs
 * subset is "arrays of scalars only".
 */
function walkArray(arrayNode: ASTNode, key: string): unknown[] {
  const arrayValues = arrayNode.children.find(
    (c): c is ASTNode => isASTNode(c) && c.ruleName === "array_values",
  );
  if (!arrayValues) return [];

  const out: unknown[] = [];
  for (const child of arrayValues.children) {
    if (!isASTNode(child)) continue; // skip NEWLINE / COMMA tokens
    /* v8 ignore start */
    // Unreachable: array_values only contains NEWLINE/COMMA tokens and value nodes.
    if (child.ruleName !== "value") {
      throw new TypeError(
        `forme-doc-frontmatter: unexpected node ${JSON.stringify(child.ruleName)} inside TOML array (key ${JSON.stringify(key)})`,
      );
    }
    /* v8 ignore stop */
    const v = walkValue(child, key);
    // Arrays-of-arrays / arrays-of-inline-tables aren't part of the
    // docs subset.  Plain object / array values mean the element was a
    // compound — reject explicitly so the failure mode is "your TOML
    // is too rich" not "you got a weird nested structure downstream".
    if (Array.isArray(v) || (v !== null && typeof v === "object")) {
      throw new TypeError(
        `forme-doc-frontmatter: TOML arrays-of-arrays / arrays-of-tables are not supported in docs frontmatter (key ${JSON.stringify(key)})`,
      );
    }
    out.push(v);
  }
  return out;
}

// ─────────────────────────────────────────────────────────────────────────
// Scalar decoding
// ─────────────────────────────────────────────────────────────────────────

/**
 * Convert a scalar token (from the toml-lexer) into its JS value.
 *
 * The toml-lexer strips outer quotes from string tokens but leaves
 * escape sequences as raw text (its `escapes: none` mode).  That means
 * BASIC_STRING / ML_BASIC_STRING values need a second pass to interpret
 * `\n`, `\t`, `\\`, `\uXXXX`, etc.  LITERAL_STRING / ML_LITERAL_STRING
 * values are taken verbatim.
 *
 * Datetime tokens are returned as their RFC 3339 string — we never
 * construct a `Date`, because docs frontmatter callers may want to
 * format the string differently or pass it through to downstream
 * tooling that does its own parsing.
 */
function decodeScalarToken(tok: Token, key: string): unknown {
  if (tok.value.length > MAX_VALUE_LEN) {
    throw new TypeError(
      `forme-doc-frontmatter: TOML value for ${JSON.stringify(key)} exceeds ${MAX_VALUE_LEN}-byte cap`,
    );
  }

  switch (tok.type) {
    case "BASIC_STRING":
    case "ML_BASIC_STRING":
      return decodeBasicEscapes(tok.value, tok.line);

    case "LITERAL_STRING":
    case "ML_LITERAL_STRING":
      return tok.value;

    case "TRUE":  return true;
    case "FALSE": return false;

    case "INTEGER":
      return decodeInteger(tok.value);

    case "FLOAT":
      return decodeFloat(tok.value);

    // Datetimes — return as the canonical RFC 3339 string.
    case "OFFSET_DATETIME":
    case "LOCAL_DATETIME":
    case "LOCAL_DATE":
    case "LOCAL_TIME":
      return tok.value;

    /* v8 ignore start */
    // Unreachable: the cases above exhaust value's scalar alternation.
    default:
      throw new TypeError(
        `forme-doc-frontmatter: unsupported TOML scalar token type ${JSON.stringify(tok.type)} for key ${JSON.stringify(key)}`,
      );
    /* v8 ignore stop */
  }
}

/**
 * Decode a TOML INTEGER token.  Handles decimal, hex (0x), octal (0o),
 * binary (0b), an optional leading `+`/`-` sign, and underscore
 * separators (`1_000_000`).
 */
function decodeInteger(raw: string): number {
  const stripped = raw.replace(/_/g, "");
  // Pull off sign so we can route the rest through parseInt cleanly.
  let sign = 1;
  let body = stripped;
  if (body.startsWith("+")) { body = body.slice(1); }
  else if (body.startsWith("-")) { sign = -1; body = body.slice(1); }

  let n: number;
  if (/^0x/i.test(body))      n = parseInt(body.slice(2), 16);
  else if (/^0o/i.test(body)) n = parseInt(body.slice(2), 8);
  else if (/^0b/i.test(body)) n = parseInt(body.slice(2), 2);
  else                        n = parseInt(body, 10);

  n = sign * n;
  if (!Number.isSafeInteger(n)) {
    throw new TypeError(
      `forme-doc-frontmatter: TOML integer ${JSON.stringify(raw)} exceeds safe integer range`,
    );
  }
  return n;
}

/**
 * Decode a TOML FLOAT token.  Handles `inf`, `nan`, scientific notation,
 * decimal floats, optional sign, and underscore separators.
 */
function decodeFloat(raw: string): number {
  const stripped = raw.replace(/_/g, "");
  // Special values — parseFloat handles "Infinity" but not "inf".
  if (/^[+-]?inf$/i.test(stripped)) {
    return stripped.startsWith("-") ? -Infinity : Infinity;
  }
  if (/^[+-]?nan$/i.test(stripped)) {
    return NaN;
  }
  return parseFloat(stripped);
}

/**
 * Apply TOML basic-string escape sequences to the lexer's raw value.
 *
 * Supported escapes (per TOML 1.0):
 *   \\ \" \n \t \r \b \f \/  \uXXXX  \UXXXXXXXX
 *
 * Anything else after a `\` is rejected — we don't silently pass unknown
 * escapes through, because they're almost always typos.
 */
function decodeBasicEscapes(raw: string, lineNo: number): string {
  let out = "";
  for (let i = 0; i < raw.length; i++) {
    const ch = raw[i]!;
    if (ch !== "\\") { out += ch; continue; }

    const next = raw[i + 1];
    /* v8 ignore start */
    // Unreachable: BASIC_STRING regex `([^"\\\n]|\\.)*` requires every
    // backslash to be followed by some character; a trailing lone backslash
    // would have failed to lex.  Kept as defense-in-depth.
    if (next === undefined) {
      throw new TypeError(
        `forme-doc-frontmatter: TOML trailing backslash escape on line ${lineNo}`,
      );
    }
    /* v8 ignore stop */

    if (next === "\\") { out += "\\"; i++; continue; }
    if (next === '"')  { out += '"';  i++; continue; }
    if (next === "n")  { out += "\n"; i++; continue; }
    if (next === "t")  { out += "\t"; i++; continue; }
    if (next === "r")  { out += "\r"; i++; continue; }
    if (next === "b")  { out += "\b"; i++; continue; }
    if (next === "f")  { out += "\f"; i++; continue; }
    if (next === "/")  { out += "/";  i++; continue; }

    if (next === "u") {
      const hex = raw.slice(i + 2, i + 6);
      if (hex.length !== 4 || !/^[0-9a-fA-F]{4}$/.test(hex)) {
        throw new TypeError(
          `forme-doc-frontmatter: TOML invalid \\u escape on line ${lineNo} (need 4 hex digits)`,
        );
      }
      out += String.fromCharCode(parseInt(hex, 16));
      i += 5; // consume \uXXXX (we'll i++ at loop end → 6 total)
      continue;
    }
    if (next === "U") {
      const hex = raw.slice(i + 2, i + 10);
      if (hex.length !== 8 || !/^[0-9a-fA-F]{8}$/.test(hex)) {
        throw new TypeError(
          `forme-doc-frontmatter: TOML invalid \\U escape on line ${lineNo} (need 8 hex digits)`,
        );
      }
      out += String.fromCodePoint(parseInt(hex, 16));
      i += 9;
      continue;
    }

    throw new TypeError(
      `forme-doc-frontmatter: TOML unsupported escape \\${next} on line ${lineNo}`,
    );
  }
  return out;
}
