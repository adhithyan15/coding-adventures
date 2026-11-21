# coding_adventures_lexer

Generic hand-written tokenizer for Dart plus a formal DFA describing the
top-level dispatch logic.


## Grammar-driven lexing (`grammarTokenize`)

`grammarTokenize(source, grammar)` lexes `source` with a compiled `TokenGrammar`
in time linear in the source length: every definition is matched in place with
`RegExp.matchAsPrefix`, never against a copy of the remaining input.

- **Positions**: offsets are UTF-16 code units internally while `column` counts
  code points, so astral characters (emoji) lex correctly and report the same
  columns as before.
- **Regex context**: patterns are matched inside the full source, so lookbehind
  and `\b` see the preceding text. Do not use `^` to mean "start of this token".
- **Diagnostics**: `LexerError` messages are static and never echo caller source,
  keywords, or string contents (CWE-209); only `line` and `column` locate the
  failure. All messages and ids live in `lib/src/messages.dart`.
