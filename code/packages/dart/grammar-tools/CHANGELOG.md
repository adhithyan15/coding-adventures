# Changelog

## Unreleased

- Fix: `parseTokenGrammar` normalizes Rust-regex syntax Dart's
  `RegExp` rejects — `(?i:...)` scoped groups become explicit ASCII
  case pairs (letters and same-case ranges, inside and outside
  classes) and Annex-B-only `\-` becomes `\x2D` (identical in every
  position). Other inline flags fail fast with a clear error.
  Required by the mermaid grammars (`er`, `requirement`); no
  shipped lane parses grammars at runtime, so nothing else changes.
- Feature (F10 transitions): `parseTokenGrammar` now parses the
  `start_mode:` directive and the `transitions:` section (`on TOKENS
  [in MODE] [if COND && ...] -> ACTION, ...`) into the new
  `ModeTransition` / `TransitionAction` / `TransitionCondition`
  types; `validateTokenGrammar` checks start-mode/mode/action-target
  resolution, the 4096-rule cap, `refuse`/conditions under
  `mode: indentation`, and trigger emittability; the compiler emits
  `startMode` and `transitions`.
- Fixed the Dart emitters rendering `$` bare in string literals (JSON
  escaping leaves it alone, but Dart reads it as interpolation), which
  made any grammar containing `$` (`css.tokens`) generate unparseable
  code. All string emissions now escape `$` as `\$`.
- Fixed the `.tokens` literal scanner skipping `\`-escapes when hunting
  the closing quote: literals are escape-blind by construction (Rust
  uses `rest.find('"')`, Go uses `strings.Index`), so `"\"` is the
  one-backslash pattern. The old scan rejected `apl.tokens` SCAN as
  unclosed.

## 0.1.0

- Initial Dart port of `grammar-tools`.
- Added `.tokens` parsing and validation.
- Added `.grammar` parsing and validation.
- Added cross-validation and source compilation helpers.
