# Changelog

All notable changes to the `coding-adventures-javascript-lexer` crate will be documented in this file.

## [0.8.0] - 2026-06-15 — gap-044b: complex template substitutions

### Fixed
- Template literal substitutions with non-identifier expressions now lex without
  error on es2025.  Affected patterns included `${obj.name}`, `${a + b}`,
  `${f()}`, `${{a: 1}}`, `${x ? y : z}`, and multiple substitutions in one
  literal.  Root cause: F10 flat-mode transitions inside `${...}` overwrote the
  active group (to "div" / "default"), causing the closing `}` to be consumed as
  RBRACE instead of TEMPLATE_TAIL.  The fix tracks template entry depths in the
  `lexer` crate (GrammarLexer) and overrides the group at match time.
- 7 new regression tests covering the above shapes.

## [0.7.0] - 2026-06-14

### Added
- **Template-substitution lexer modes (gap-044, first slice)** — `es2025.tokens`
  now wires up template literal `${...}` substitutions via two new flat modes:
  - `template` — active immediately after `TEMPLATE_HEAD` or `TEMPLATE_MIDDLE`
    (both of which end with `${`).  It is a *flat* (set-mode) target so it
    inherits the default group's patterns (NAME, NUMBER, …) while placing its
    own `TEMPLATE_TAIL` / `TEMPLATE_MIDDLE` patterns first.  An empty body
    (`` `${}` ``) is handled by the own patterns matching the opening `}`.
  - `template_div` — active after a NAME is emitted inside `${...}` (mirrors
    the role of `div` in the outer expression: `SLASH`/`SLASH_EQUALS` override
    plus `TEMPLATE_TAIL`/`TEMPLATE_MIDDLE` at own priority so the closing `}`
    is recognised before the inherited `RBRACE`).

  New transitions added (in first-match-wins order before the general
  value-producing rule):
  ```
  on TEMPLATE_HEAD              -> set-mode template
  on TEMPLATE_MIDDLE            -> set-mode template
  on NAME in template           -> set-mode template_div
  on NAME in template_div       -> set-mode template_div
  on TEMPLATE_TAIL              -> set-mode div
  ```

  Closes closurec gap-044 for the common `${singleIdentifier}` case.
  Documented limitation: expressions containing operators (`.`, `+`, `(`, …)
  or nested `{ }` reset the mode back to `default`/`div`, losing the template
  context.  Full brace-depth support is a follow-up.

- The compiled `_grammar.rs` was regenerated from `es2025.tokens`.

## [0.6.0] - 2026-06-13

### Added
- **F10 regex-vs-division mode table (ES2025)** — `es2025.tokens` now
  declares `start_mode: default`, a flat `div` mode (`group div:` whose
  `SLASH`/`SLASH_EQUALS` patterns override `REGEX`), and a `transitions:`
  table encoding Acorn's `exprAllowed`: value-producing tokens
  (NAME/NUMBER/STRING/REGEX/`)`/`]`/`this`/`super`/`true`/`false`/`null`)
  enter `div` mode (a following `/` is DIVISION); operators, openers and
  expression-keywords (`return`/`typeof`/`in`/`new`/…) return to `default`
  (a following `/` starts a REGEX). The shared `GrammarLexer` interprets
  the table (no hand-written per-language callback). This makes `a/b/c`
  lex as three divisions and `return/re/` lex `/re/` as one REGEX token —
  closing closurec gap-092/gap-115/gap-119.
- The compiled `_grammar.rs` was regenerated from the grammars. This run
  also picks up the previously-deferred generic (`""`-version) lexer
  module that the generator emits but the committed file had been stale
  against (noted in CLOC12.99's gap-096 resolution).

## [0.5.1] - 2026-06-12

### Fixed
- **REGEX flag class (CORRECTNESS, ES2024/ES2025)** — the `REGEX` token
  pattern in `es2024.tokens` and `es2025.tokens` had the flag character
  class `[dgimsvy]`, which accidentally omitted the ES2015 `u` (unicode)
  flag — a typo introduced when `v` (unicodeSets, ES2024) was added. As
  a result a regex such as `/x/gimsuy` lexed as the truncated regex
  `/x/gims` followed by a stray identifier `uy`. Corrected the class to
  the full ES2024 set `[dgimsuvy]` (d, g, i, m, s, u, v, y) in both
  source grammars and regenerated the compiled lexer pattern. New tests
  `es2025_regex_accepts_all_modern_flags_as_one_token` and
  `es2024_regex_accepts_u_flag`. Unblocks closurec gap-096.

## [0.5.0] - 2026-05-21

### Added
- New dependency on `coding_adventures_correlation_vector` for `CVLog` and `Origin` types.
- `pub struct TokenWithCv { pub token: Token, pub cv: String }` — token paired with its CV identifier.
- `tokenize_javascript_with_cv(source, source_file, EsVersion, &mut CVLog) -> Result<Vec<TokenWithCv>, String>` — tokenize and assign a CV ID per token per CLOC03 §"Stage 1 — Lexer".
- Per-token `Origin` records: `source = source_file`, `location = "line:col"` from `Token.line` and `Token.column`. No `Contribution` appended (lexing is creation, not modification — per CLOC03).
- Module docs added a CV plumbing section linking to CLOC03.
- 4 new tests:
  - `tokenize_with_cv_assigns_an_id_per_token` — every token gets a non-empty CV.
  - `tokenize_with_cv_ids_are_unique` — uniqueness across the full token stream.
  - `tokenize_with_cv_entries_resolvable_in_log` — `cv.get(id)` returns an entry whose `Origin.source` matches the requested `source_file` and whose `location` is `"line:col"`.
  - `tokenize_with_cv_disabled_log_still_returns_tokens` — `CVLog::new(false)` keeps the API shape but skips storage (per CLOC03's production fast path).

### Notes
- The string-based and typed APIs from prior versions remain untouched; this PR is purely additive.
- The `cv` field on `TokenWithCv` is `String` because `correlation-vector` represents IDs as `String` today.
- This is the first concrete CLOC03 plumbing — the parser will consume `TokenWithCv` and inherit CV IDs onto AST nodes in a follow-up PR.

## [0.4.0] - 2026-05-21

### Added
- New dependency on `coding-adventures-javascript-tokens` for the shared `EsVersion` enum.
- `create_javascript_lexer_typed(source, EsVersion) -> GrammarLexer` — infallible typed constructor; no unknown-version error path.
- `tokenize_javascript_typed(source, EsVersion) -> Result<Vec<Token>, String>` — typed tokenizer; only error is tokenization itself.
- `pub const DEFAULT_ES_VERSION: EsVersion = EsVersion::Es2025;` — typed default. New code should prefer this over the string `DEFAULT_VERSION`.
- New tests covering the typed APIs: `tokenize_typed_es2015`, `default_es_version_constant_is_es2025`, `all_typed_versions_load`, `create_lexer_typed_returns_grammar_lexer`.

### Notes
- The existing `&str`-based APIs (`create_javascript_lexer`, `tokenize_javascript`, `DEFAULT_VERSION`) are kept for backwards compatibility. The typed APIs are the preferred surface going forward.
- This PR is part of CLOC02 Phase 1 — see CLOC01/CLOC02 in `code/specs/` for the broader rollout.

## [0.3.0] - 2026-05-20

### Removed
- Dropped support for the empty-string `""` "generic" version that pointed at the stub `code/grammars/javascript.tokens`. The stub was a 35-line proof-of-concept subset; the full ES1 through ES2025 grammars under `code/grammars/ecmascript/` supersede it.
- Removed the embedded `mod generic` block (~228 lines) from `_grammar.rs`.

### Changed
- `DEFAULT_VERSION` is now `"es2025"` (was `""`). Callers passing the old empty-string version now get `Err` with the supported-versions list.
- Crate docstring no longer mentions the "generic" grammar.

### Added
- `default_version_resolves_to_es2025` test verifies the new default.

### Migration
- Replace `tokenize_javascript(source, "")` with `tokenize_javascript(source, "es2025")` (or another explicit ES version).

### Notes
- This PR is the Rust-only first step of CLOC01 Phase 1 stub retirement. The stub `.tokens`/`.grammar` files remain on disk for now because the Go, Python, TypeScript, and Ruby ports still embed them. Those ports get equivalent follow-up PRs; once all are migrated, the stub source files will be deleted.

## [0.2.0] - 2026-04-05

### Changed
- `create_javascript_lexer(source, version)` now accepts a `version: &str` parameter and returns `Result<GrammarLexer, String>` instead of panicking.
- `tokenize_javascript(source, version)` now accepts a `version: &str` parameter and returns `Result<Vec<Token>, String>` instead of panicking.

### Added
- Version-aware grammar selection: pass `""` for the generic grammar or one of `"es1"`, `"es3"`, `"es5"`, `"es2015"`–`"es2025"` for versioned ECMAScript grammars stored in `grammars/ecmascript/`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")` rather than string formatting.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- New tests: `test_versioned_es2015`, `test_all_versioned_grammars`, `test_unknown_version_returns_err`, `test_create_lexer_unknown_version`.

## [0.1.0] - 2026-03-21

### Added
- `create_javascript_lexer(source)` — factory function that loads `javascript.tokens` and returns a configured `GrammarLexer`.
- `tokenize_javascript(source)` — convenience function that tokenizes JavaScript source and returns `Vec<Token>`.
- Loads grammar from `javascript.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering variable declarations, keywords, arithmetic operators, multi-character operators, strings, numbers, comments, delimiters, whitespace, function expressions, arrow operators, and the factory function.
