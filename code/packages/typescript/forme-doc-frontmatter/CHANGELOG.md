# Changelog — @coding-adventures/forme-doc-frontmatter

## 0.2.0 — 2026-05-21

### Changed (breaking under the hood, but no public API change)

- **Replaced the in-house TOML parser with the repo's full TOML
  1.0 parser plus an AST walker.**  `src/toml.ts` no longer
  re-implements TOML — it imports `parseTOML` from
  `@coding-adventures/toml-parser`, walks the resulting
  `ASTNode` tree, and enforces the docs-frontmatter subset
  (rejecting table headers, array-of-tables, dotted keys,
  quoted keys, inline tables, and arrays-of-arrays).

  **Why:** the repo already shipped a full-fidelity TOML 1.0
  parser.  Having a second tiny TOML parser here was duplicated
  code with subtly different behaviour (the in-house version
  didn't handle multi-line strings, all four datetime forms,
  underscore separators in numbers, hex/oct/bin integers,
  `inf`/`nan`, or the full basic-string escape set).  Walking
  the real AST gives us all of that for free.

  **Capability impact:** `required_capabilities.json` stays at
  `[]`.  This depends on `@coding-adventures/toml-parser` v0.1.1
  (where the grammar was precompiled into `_grammar.ts` so the
  parser is itself pure-transform — see toml-parser's 0.1.1
  changelog).  If you bump toml-parser to an earlier version
  that reads the grammar at runtime, this package's capabilities
  would cascade to `[fs:read]`.

  **Public API:** unchanged.  `parseToml(source)` still returns
  `Record<string, unknown>`; `extractFrontmatter` still returns
  the same `FrontmatterResult`.  All existing positive-path
  test cases pass with no modifications.

  **Error messages changed:** surface-syntax errors (malformed
  TOML the lexer/parser rejects before the walker ever runs)
  now carry the upstream parser's messages with `line:col`
  rather than our hand-rolled "TOML line N is not a 'key =
  value' pair" wording.  Subset-violation errors (table
  headers, inline tables, etc.) are still our own
  `forme-doc-frontmatter:` messages.

### Added

- **TOML coverage broadened to full lexer-supported scalars:**
  multi-line basic + literal strings, hex/oct/bin integers with
  underscore separators, scientific-notation floats, `+inf` /
  `-inf` / `inf` / `nan`, `\b` / `\f` / `\/` / `\uXXXX` /
  `\UXXXXXXXX` escapes, all four datetime tokens.
- **Subset-rejection tests** for every newly-relevant construct
  the upstream parser accepts but the docs subset doesn't:
  dotted keys, quoted keys (basic + literal), inline tables,
  array-of-inline-tables, arrays-of-arrays, hyphen-leading
  bare keys, 300-char bare keys.

### Dependencies

- Added `@coding-adventures/toml-parser` + its full transitive
  `file:` chain (`toml-lexer`, `parser`, `grammar-tools`,
  `lexer`, `directed-graph`, `cli-builder`, `state-machine`)
  as runtime deps.  `BUILD` chain-installs them leaf-to-root
  before running `npm install` here.

### Tests

135 tests across 3 files (was 100).  Coverage **98.88% line /
97.85% branch** (was 96.12% line).  The remaining uncovered
lines in `toml.ts` are defensive throws marked
`/* v8 ignore start */ … /* v8 ignore stop */` because the
upstream grammar's structural guarantees make them
genuinely unreachable.

### Version

`0.1.0` → `0.2.0` (minor bump: no public API break, but
behaviour widens significantly because the underlying parser
is now full TOML 1.0).

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
