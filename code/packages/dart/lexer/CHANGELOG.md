# Changelog

## Unreleased

- Fix: indentation mode now applies the reference's built-in
  whitespace rule — ` `, `\t` and `\r` are skipped between tokens
  unconditionally, not just inside brackets. Grammars without a
  `WHITESPACE` skip (every `python*.tokens`) previously failed on
  ordinary inter-token spaces.
- Feature (F10 transitions): `GrammarLexer` applies the declarative
  `transitions:` table after each emitted token (built-in NEWLINE
  included), with `start_mode` initialization, flat-mode default
  inheritance (`set-mode` targets; `push` targets stay exclusive and
  win ties), `in`-mode/value/`if` guards, and all six actions. The
  callback still runs first and the table refines; `refuse` throws a
  payload-blind `LexerError`. Unknown groups and start modes fall
  back to default rather than throwing. New read-only `activeGroup`
  / `groupStackDepth` observability.
- Feature (F04 core): new `GrammarLexer` class with named pattern
  groups and an on-token callback driving `LexerContext`
  (push/pop/emit/suppress/skip-toggle/peek/lookbehind), gap flags,
  keyword/reserved handling, and string escape processing. Diagnostics
  stay payload-blind per Standing Rule #20 while keeping every
  reference assertion prefix; custom types keep grammar names
  verbatim. Deferred to later slices: guards, transitions, brackets,
  templates, ranges, layout mode.

- Feature (F04 hooks slice): `GrammarLexer.addPreTokenize` and
  `addPostTokenize`, mirroring the reference's `Box<dyn Fn>` hook
  stages — source transforms compose left-to-right and replace the
  source before lexing; token transforms compose left-to-right over
  the EOF-inclusive list after lexing.

- Hardening (CWE-400): `grammarTokenize` now matches with
  `RegExp.matchAsPrefix(source, position)` and `String.startsWith` instead of
  copying `source.substring(position)` for every attempt, so lexing is linear
  in source size instead of quadratic. Regexes now see the real preceding text,
  so lookbehind and `\b` behave as written and patterns must not rely on `^`
  meaning "start of this token".
- Fix: `grammarTokenize` advanced its UTF-16 `position` once per code point, so
  any astral character (for example an emoji) misaligned all later offsets and
  produced a spurious lexical error. It now advances by two code units for
  astral code points while `column` still counts code points, leaving existing
  line/column numbers unchanged.
- Standing Rule #20: every lexer error message and id moved to
  `lib/src/messages.dart` and made static and payload-blind (CWE-209); the
  offending character, keyword, and string contents are no longer echoed.
  `LexerError(message, line, column)` is unchanged; it gains an optional
  `errorId`.
- BUILD now runs `dart analyze --fatal-infos` and a 95% coverage gate (100%
  achieved).

- Strip matching single, double, and triple delimiters from named string token
  types such as `BASIC_STRING` and `ML_LITERAL_STRING`.

## 0.1.0

- Initial release.
- Added `grammarTokenize()` for grammar-driven lexers backed by `TokenGrammar`.
