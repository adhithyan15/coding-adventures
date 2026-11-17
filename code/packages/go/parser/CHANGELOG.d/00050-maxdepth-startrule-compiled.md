## [0.4.0] - 2026-11-16

### Security
- **Fixed a process-killing stack overflow.** The packrat engine recursed once
  per nesting level with no bound, so input such as `f(` followed by 200,000
  `[` and 200,000 `]` and `)` ended in `fatal error: stack overflow`, which Go
  cannot recover from. Any program parsing untrusted text with this package
  (json-parser and every other `*-parser`) could be taken down by one request.
  Rule recursion is now counted and refused past a limit. Existing callers are
  protected by the default (`DefaultMaxDepth` = 10,000 rule levels, about 16 MB
  of stack at most, against a 1 GB cap) with no code change.
- The refusal is sticky: once the limit is hit every in-flight matcher fails
  and `Parse` reports the error even if an enclosing optional or repetition
  would have backtracked past it and "succeeded" on a truncated prefix.

### Added
- `Options{MaxDepth, StartRule, Trace, PreParse, PostParse}` and
  `NewGrammarParserWithOptions`.
- `ErrMaxDepth` (returned wrapped in `*GrammarParseError`, match with
  `errors.Is`), `DefaultMaxDepth`, `HardMaxDepth` (50,000; larger requests are
  clamped, there is no "unlimited").
- `StartRule`: parse from any named rule; an unknown name is an error
  (`ErrUnknownStartRule`).
- `CompileGrammar(pg) (*Compiled, error)` and `(*Compiled).Parse(tokens, opts)`:
  compile once (rule ids, pre-quoted literals, pre-mapped token types), parse
  many inputs, safe for concurrent use, memo storage pooled between parses.
- `GrammarParseError.Err` and `Unwrap`.
- Benchmarks `BenchmarkParseJSONLike`, `BenchmarkParseExpression`,
  `BenchmarkParseSmallLine` and compiled-reuse variants.
- Tests for deep nesting (1,000,000 brackets), the hostile `f([[[...` shape,
  backtracking past the limit, StartRule, reuse across goroutines, every
  element kind, and the previously untested `WalkAST` / `FindNodes`.

### Changed
- Memoization is now two flat slices (a per-position list of slots) instead of
  `map[[2]int]*memoEntry`; the rule body appends into a single children slice
  instead of concatenating per element. Roughly 2.2x faster and about 55% fewer
  allocations on the benchmark inputs. Left-recursion seed growing is unchanged
  and covered by tests; a randomised differential run against the old engine
  (36,000 token streams over nine grammars) found identical trees and errors.
- `NewGrammarParser` / `NewGrammarParserWithTrace` are thin wrappers over
  `CompileGrammar`. A nil grammar is now reported by `Parse` as an error
  rather than a nil-pointer panic.
- Documented that the parser performs no error recovery and fails closed.

### Fixed
- Reading past the end of the token list now yields a synthetic EOF instead of
  repeating the last token, so a list with no trailing EOF (for example a lone
  `(`) is a syntax error rather than unbounded recursion.
- A trailing NEWLINE token with no EOF no longer spins the newline skipper
  forever, and a repetition body that matches nothing no longer loops forever.
- `Parse` recovers panics from hooks and returns them as errors.
- An empty token list no longer panics: it is a syntax error for a grammar
  that cannot match nothing (a start rule that can match nothing still
  succeeds on it, as it does on a list holding only EOF).

### Changelog layout
- Converted the monolithic `CHANGELOG.md` into `CHANGELOG.d/` (one file per
  entry, content unchanged).
