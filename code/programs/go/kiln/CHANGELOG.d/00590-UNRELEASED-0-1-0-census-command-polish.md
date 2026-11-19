## [0.1.0] - UNRELEASED

### Changed (census PR C, final polish: `kiln record census`)

- A `parse_panic` makes the census exit 4 (like a mismatch): a trace that
  crashes a reader is soak-relevant. KILN04's exit-code text is updated.
- `first_parse` is the first timed parse whatever its outcome, with an
  `accepted` flag, so it reflects the cold start even when that parse refused.
- Redaction boundaries are tighter: a prefix must be preceded by the start of the
  text, whitespace, a quote, `=`, `:`, `,`, `(`, `[`, `<` or a one-letter `-X`
  flag (`-L/tmp/x`), so `+`, `@` and `~` no longer start a path
  (`/usr/include/c++/tmp` stays as it is).
- A drift test parses internal/traced and internal/trace and requires every
  reason constant to map to itself in the census's closed lists, so a new reason
  cannot silently become `other`.
