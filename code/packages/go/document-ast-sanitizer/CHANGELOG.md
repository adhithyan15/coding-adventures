# Changelog — document-ast-sanitizer (Go)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.2.0] — 2026-04-02

### Changed
- Wrapped all public functions with the Operations system (`StartNew[T]`):
  `Sanitize`, `StripControlChars`, `ExtractScheme`, `IsSchemeAllowed`.
- Every public call now has automatic timing, structured logging, and panic
  recovery via the capability-cage Operations infrastructure.
- Public API signatures are unchanged.

## [0.1.0] — 2026-03-24

### Added

- Initial implementation of the `document-ast-sanitizer` Go package.
- `Sanitize(doc *DocumentNode, policy SanitizationPolicy) *DocumentNode` —
  pure, immutable policy-driven AST transformation. Never mutates the input.
- `SanitizationPolicy` struct with full coverage of all Document AST node types.
- `RawFormatPolicy` type with `RawDropAll`, `RawPassthrough`, `RawAllowList`
  modes for fine-grained raw block/inline control.
- Three named presets:
  - `STRICT` — for user-generated content (drops raw HTML, allows http/https/mailto, converts images to alt text, clamps h1 → h2)
  - `RELAXED` — for semi-trusted content (allows HTML raw blocks, allows ftp)
  - `PASSTHROUGH` — no sanitization (identity transform)
- `StripControlChars(url string) string` — removes C0 controls and zero-width
  characters (U+200B/C/D, U+2060, U+FEFF) before scheme extraction.
- `ExtractScheme(url string) string` — extracts the lowercased URL scheme,
  treating relative URLs (colon after slash/question mark) as schemeless.
- `IsSchemeAllowed(url string, policy SanitizationPolicy) bool` — full
  two-step URL validation (strip controls → extract scheme → allowlist check).
- Complete node handling for all 18 Document AST node types.
- Empty-children promotion: container nodes with no surviving children are
  dropped (except `DocumentNode` which is always kept).
- Link child promotion: `dropLinks: true` promotes children to parent instead
  of silently dropping text.
- 62 unit tests covering all policy options, all three presets, all XSS vectors
  from the spec (null-byte bypass, zero-width bypass, uppercase scheme bypass,
  data:/blob:/vbscript: schemes), and immutability verification.
- 94.7% statement coverage.
- `BUILD` file for the monorepo build tool.
- `README.md` with usage examples and node-type coverage table.

### Implementation Notes

- The `MaxHeadingLevel` field uses `0` to mean "not set" (defaults to 6) and
  `-1` to mean "drop all headings" (the spec's `"drop"` option).
- The `MinHeadingLevel` field uses `0` to mean "not set" (defaults to 1).
- `AllowAllSchemes: true` is used by `PASSTHROUGH` to bypass scheme checking
  entirely without nil-slice ambiguity.
- The sanitizer follows the "fail-safe" principle: unknown node types are
  silently dropped rather than passed through.
