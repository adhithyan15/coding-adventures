# Changelog — @coding-adventures/forme-aot-script-tag-emitter

## 0.1.0 — 2026-05-19

Initial release.  Sixteenth FM00 v0 stage package — HTML
`<script src="...">` tag emitter with Subresource Integrity (SRI)
format validation and crossorigin / referrerpolicy / async /
defer / nomodule support.

Pure transform: `ScriptTag | ScriptTag[]` → HTML string.
Two-pass validate-then-emit so callers never see partial output.

### Added

- `generateScriptTags(input): string` — main entry.  Accepts a
  single `ScriptTag` or array; returns one tag per line.
- `validateScriptSrc(url)` — http(s)://-or-root-relative URL
  accept-list (same pattern as the sibling FM00 emitters).
- `validateIntegrity(value)` — SRI string format validator.
  Accepts one or more whitespace-separated `<algo>-<base64>`
  tokens with algo ∈ `{sha256, sha384, sha512}` and base64
  matching the algo's expected digest length (44 / 64 / 88
  chars).  Standard base64 only (no URL-safe `- _` variants).
- `validateScriptType(value)` — allowlist of
  `{module, importmap}`.
- `validateCrossOrigin(value)` — `{anonymous, use-credentials}`.
- `validateReferrerPolicy(value)` — all eight spec-defined
  Referrer Policy values.
- `escapeHtmlAttr` / `stripAsciiControl` — single-pass HTML
  attribute escape + C0 control strip.
- Types: `ScriptTag`, `ScriptType`, `CrossOrigin`,
  `ReferrerPolicy`.

### Spec adherence

Implements HTML Living Standard `<script>` element semantics +
Subresource Integrity (W3C SRI) + Referrer Policy enum.  No
divergence.

### Behavioural notes

- **Attribute order**: `type → src → integrity → crossorigin →
  referrerpolicy → async → defer → nomodule`.
- **Boolean attributes** emit as bare attribute names
  (`async`, `defer`, `nomodule`), spec-canonical form.
- **`async` + `defer`** simultaneously → throws (spec says
  `defer` is silently ignored; we treat the combination as a
  caller bug rather than hide it).
- **Empty array input** → empty string output.
- **Single object** treated as a one-element array (same output
  as `[{...}]`).
- **Reproducibility.**  Same input → byte-identical output.
  No input mutation.

### Security posture

Eight concerns explicitly addressed (pre-push review found
three issues and they're fixed below):

- **URL scheme injection.**  Every `src` runs through
  `validateScriptSrc`.  `javascript:`, `data:`, `file:`,
  `vbscript:`, protocol-relative `//host`, backslash-variant
  `/\host`, bare relative, empty, non-string all rejected.
- **HTML attribute injection.**  Every interpolated value
  passes through `escapeHtmlAttr`.  Single-pass character-
  class replacement covers all five HTML entities + strips
  ASCII control bytes.
- **SRI integrity format validation.**  Malformed `integrity=`
  silently disables SRI in some browsers and allows MITM
  substitution.  The validator pins algo prefix, base64
  charset (standard only — not URL-safe), length, AND per-algo
  padding count.  Per-algo padding is the subtle one — a sha256
  string with `==` instead of `=` is the right total length
  (44 chars) but decodes to 31 bytes, not 32, and browsers
  silently disable SRI rather than throw.  Fixed: pad count
  must be exactly `(3 - bytes % 3) % 3` for each algo.
- **Object-prototype walk defence.**  `SRI_ALGOS` is a `Map`
  (not a plain object) so attacker-supplied algo names like
  `"__proto__"`, `"toString"`, `"hasOwnProperty"` can't return
  truthy values from `Object.prototype` and bypass the
  "unknown algo" branch.
- **ASCII control bytes in `src` are rejected** at the
  validator (not silently stripped by `escapeHtmlAttr`).
  Otherwise `/\tevil` would pass the root-relative check
  (`url[1]` is tab, not `/` or `\`) and then become `/evil`
  after escape — a different file than the caller asked for.
- **`type` / `crossorigin` / `referrerpolicy` allowlists.**
  Defend against typo-driven bugs.  `type` deliberately
  excludes legacy `text/javascript` etc. (equivalent to
  omission in modern browsers; including them adds typo
  surface).
- **`async` + `defer` conflict** rejected as caller bug rather
  than silently ignored.
- **Fail-fast.**  Validation completes for every entry BEFORE
  any tag is emitted.  An exception means the caller has
  nothing to write.

### Capabilities

`[]` — pure transform.  No I/O, network, fs, shell, env.

### Tests

139 tests across 2 files:

- `validate.test.ts` (93) — `validateScriptSrc` accept (https,
  http, case-insensitive scheme, root-relative, bare /, multi-
  segment) + reject matrix (javascript:, data:, file:,
  vbscript:, protocol-relative, backslash-variant, bare
  relative, empty, non-string, null, undefined, long-URL
  truncation in error, NUL / tab / newline / DEL / ESC control
  bytes); `validateIntegrity` accept (sha256 / sha384 / sha512
  single + multi-algo, whitespace collapsing, trimming, base64
  with +//, mixed alphabet) + reject (non-string, null, empty,
  whitespace-only, md5/sha1, no dash, dash at start / end,
  wrong length for sha256 / 384 / 512, invalid base64 chars
  including URL-safe `_` and triple padding, bad second token,
  wrong padding per algo (sha256 `==`, sha384 with padding,
  sha512 `=` instead of `==`), `__proto__` / `toString` /
  `hasOwnProperty` algo names (Object.prototype walk defence),
  error-contains-bad-algo); `validateScriptType`
  (2 accept + reject legacy MIMEs + case-sensitive + empty +
  non-string); `validateCrossOrigin` (2 accept + reject 'true'
  / empty / non-string); `validateReferrerPolicy` (all 8 values
  parametrised + case-sensitive + deprecated 'never' + empty +
  non-string); `escapeHtmlAttr` (5 entities + composite +
  control bytes + non-string coercion); `stripAsciiControl`
  (control-byte matrix + printable preservation).
- `generate.test.ts` (46) — single tag (minimal / module /
  importmap / SRI+crossorigin / each boolean attr /
  referrerpolicy / absolute URL / false-booleans-omitted);
  attribute order (full 7-attr order, defer before nomodule);
  array (multi-tag newline-joined, empty, single same as
  object, order preserved); URL validation (each forbidden
  scheme); SRI integrity (sha384 verbatim, two-algo verbatim,
  md5 rejected, wrong length rejected); allowlist validation
  (type / crossorigin / referrerpolicy each); async+defer
  conflict (both rejected, async-only ok, defer-only ok);
  boolean type-checking (non-bool async / defer / nomodule
  rejected); input shape (null entry, non-object, bad-entry-
  index-in-error); HTML escaping (ampersand in src);
  control bytes in src REJECTED (NUL + tab); fail-fast (mid-array bad integrity, no
  output); purity / determinism (byte-identical, no input
  mutation including arrays); full real-world example with
  verbatim line check.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **No inline `<script>...code...</script>`** — external `src`
  only.  Inline has a fundamentally different trust boundary
  and needs a dedicated emitter.
- **No `blocking="render"`** attribute (low adoption; v1 may
  add).
- **No `fetchpriority`** attribute (deferred).
- **No SRI hash computation** — caller supplies the integrity
  string; this package validates format only.
