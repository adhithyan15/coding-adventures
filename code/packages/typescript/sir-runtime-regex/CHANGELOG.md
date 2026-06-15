# Changelog

## 0.1.0 — initial release

First release of the SIR regex runtime for TypeScript/JavaScript, per
`code/specs/sir-runtime.md`. Provides the runtime landing point for the SIR
`regex` builtin — the Ruby→SIR frontend lowers `/pat/flags` to
`BuiltinCall("regex", [StrLit pattern, StrLit flags])`, which previously had no
TypeScript runtime, so emitted code using a regex failed.

### Added

- `compile(pattern, flags=""): RegExp` — build a Ruby-dialect `RegExp`,
  translating Ruby inline flags to JS flags (`i`→`i`, `m`→`s` since Ruby `/m` is
  dot-matches-newline/dotAll, `x`→strip whitespace+comments since JS has no
  extended flag; unknown chars ignored) and **always** including the JS `m` flag
  because Ruby's `^`/`$` are always line anchors. Flags are de-duplicated.
- `isMatch(pattern, s): boolean` — unanchored search (Ruby `=~`/`match?`
  semantics); accepts a `RegExp` or a raw string; a `RegExp` is cloned without
  `g`/`y` so a stale `lastIndex` cannot leak across calls.
- `matchData(pattern, s): string | null` — matched substring (`match[0]`) or
  `null`; minimal model of Ruby `String#match`.
- `stripExtended(pattern): string` — best-effort `/x` subset: strip unescaped
  whitespace and `#` comments while preserving escaped whitespace/`#`.
- `Val` — the universal SIR value type alias at this boundary.
- Full standard layout: `package.json`, `tsconfig.json`, `vitest.config.ts`,
  `BUILD`, `BUILD_windows`, `required_capabilities.json` (no capabilities),
  README. vitest suite at 100% coverage; `tsc --strict` clean.

### Design note

The Ruby→JS divergence is concentrated in the flag mapping, the unconditional
JS `m` flag (Ruby line anchors), and the `x`/extended approximation (JS has no
extended-mode flag). The shared Perl-compatible syntax passes straight through
to the `RegExp` engine.
