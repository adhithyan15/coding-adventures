# Changelog — @coding-adventures/forme-doc-frontmatter

## 0.1.0 — 2026-05-20

Initial release.  First concrete DOC00 v0 package — strip YAML
or TOML frontmatter from a markdown source string.

Pure transform: `string` → `{ body, frontmatter, format }`.
Tiny in-house YAML/TOML parsers; no `eval`, no `new Function`,
no prototype pollution.

### Added

- `extractFrontmatter(source): FrontmatterResult` — main
  entry.  Detects YAML (`---\n...\n---`) or TOML
  (`+++\n...\n+++`) frontmatter, parses it, returns the
  body without delimiters + the parsed metadata + format
  marker.  Throws `TypeError` if a delimiter opens but
  never closes, or if inner content is unparseable.
- `parseYaml(source)` — tiny YAML subset parser (flat maps,
  scalars, inline + multi-line arrays).  Standalone export
  for testing.
- `parseToml(source)` — tiny TOML subset parser (flat maps,
  scalars, inline arrays).  Standalone export for testing.
- `FrontmatterResult` type.

### Spec adherence

No external spec to fully adhere to (full YAML / full TOML
are out of scope).  Subset documented in README.

### Supported subset (intentionally narrow)

- Flat key/value maps.  No nested tables / inline tables /
  array-of-tables / nested arrays.
- Scalar types: integers (safe-integer range only), floats,
  booleans, null (YAML only), quoted strings (single or
  double), bare strings (YAML), RFC 3339 dates as strings
  (TOML).
- Arrays of scalars: inline `[a, b, c]` (both formats) or
  multi-line `- item` lists (YAML only).
- YAML quoted-string escapes: `\\` and the matching quote.
- TOML basic-string escapes: `\\`, `\"`, `\n`, `\t`, `\r`.
- TOML literal strings (`'...'`) — no escapes.
- TOML inline comments (`# ...` after a value).
- CRLF line endings.
- UTF-8 BOM at start of file (silently stripped).

### Behavioural notes

- Both delimiters must be on their own line.
- The body is returned verbatim from after the closing
  delimiter's newline — no trim, no normalisation.  Caller
  applies whatever they want.
- Same input → identical output.
- No input mutation.

### Security posture

Ten concerns explicitly addressed (pre-push review found four
hardening gaps — all fixed below):

- **No `eval` / `new Function`** in either parser.  Pure
  character-walk implementations.
- **No `JSON.parse`** — even with a reviver, prototype-
  pollution interactions are subtle.
- **Output object built via `Object.create(null)`** — no
  `Object.prototype` link, so downstream consumers can't be
  tricked into reading inherited values even if a key slipped
  past the allowlist.
- **Key allowlist rejects** `__proto__`, `constructor`,
  `prototype` outright in both parsers.
- **Duplicate top-level keys rejected** — catches typos +
  defends against "last write wins" attacks.
- **Strict malformed-input rejection**: indented continuation
  lines (YAML), structural chars in bare scalars,
  safe-integer overflow, unrecognised escapes, unterminated
  quoted strings — all throw with line numbers.
- **Widened reserved-key list.**  In addition to the three
  obvious sinks (`__proto__`, `constructor`, `prototype`),
  the parsers reject every `Object.prototype` method name
  (`toString`, `valueOf`, `hasOwnProperty`,
  `isPrototypeOf`, `propertyIsEnumerable`, `toLocaleString`,
  `__defineGetter__`, `__defineSetter__`,
  `__lookupGetter__`, `__lookupSetter__`).  Parser output is
  already a null-prototype object so we can't pollute *our*
  output — but callers commonly `Object.assign({}, fm)` into
  a normal object, after which `obj.toString` becomes a
  string and code that does `String(obj)` breaks.
- **Input size caps.**  Source ≤ 1 MB, ≤ 1000 keys per block,
  ≤ 64 KB per value.  Prevents a malicious / pathological
  input from exhausting memory.  Generous for real-world
  docs frontmatter; bounded enough that a 100 MB block of
  dashes can't lock up the process.
- **Error message truncation.**  Quoted values in error
  messages are truncated to 200 chars with ellipsis, so a
  10 MB unparseable value doesn't produce a 10 MB error
  message that floods logs.
- **CRLF-in-body normalisation documented.**  The extractor
  splits on `/\r?\n/` and rejoins with `\n`, which normalises
  CRLF line endings in the body to LF.  The doc comment
  acknowledges this explicitly (was previously claimed as
  "passthrough verbatim").

### Capabilities

`[]` — pure transform.  No I/O, network, fs, shell, env.

### Tests

107 tests across 3 files:

- `yaml.test.ts` (36) — scalars (string bare/double/single,
  integer pos/neg, float, true/false, null, ~, date as
  string); inline arrays (empty, scalar, quoted with comma,
  mixed types); multi-line arrays; multiple keys / blank
  lines / comments; security (3 reserved keys + null
  prototype + duplicate key); error matrix (indented
  continuation, non key:value, leading-digit key, empty
  scalar, safe-int overflow, structural chars in bare
  scalar, unescaped quote inside double, unsupported escape,
  supported escapes for `\\` and `\"`, unterminated inline
  list string, inline list with escape inside quoted item).
- `toml.test.ts` (34) — scalars (basic + literal strings,
  escape sequences, integer pos/neg/signed, float, booleans,
  RFC 3339 date / datetime); arrays (empty, strings,
  integers); multiple keys + comments (line + inline + `#`
  inside string); security (3 reserved keys + null prototype
  + duplicate key); error matrix (tables rejected,
  non-key=value, unrecognised scalar, safe-int overflow,
  unescaped quotes, unsupported escape, unterminated string
  including inline-comment scan + inline array, empty value,
  inline array with escapes).
- `extract.test.ts` (19) — no frontmatter (plain markdown,
  empty, delimiter-not-on-own-line); YAML (basic, CRLF,
  multi-key + array, missing close throws, BOM stripped);
  TOML (basic, array + date, missing close throws); input
  validation (non-string, null); purity / determinism (same
  input identical, no input mutation); malformed frontmatter
  rejected (YAML `__proto__` propagates, TOML table syntax);
  real-world Hugo + Jekyll/VuePress examples.

Coverage: **95.51% line / 96.33% branch** across all source
files with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **No nested tables / inline tables / array-of-tables.**
- **No multi-line strings** (TOML's `"""..."""` literal /
  YAML's `|` and `>` block-scalar indicators).
- **No anchors / aliases / custom tags** (full YAML feature).
- **No JSON-style frontmatter** (Hugo supports `{`-prefixed
  JSON; we don't — conflicts with markdown that legitimately
  starts with `{`).
- **No `Date` object construction** — TOML / YAML date strings
  are passed through verbatim; callers parse if needed.
