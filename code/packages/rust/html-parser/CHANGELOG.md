# Changelog

All notable changes to the `coding-adventures-html-parser` crate will be
documented in this file.

## [0.1.0] - 2026-05-02

### Added
- Initial HTML parser crate that consumes `coding-adventures-html-lexer` tokens
  and builds a `dom-core` document.
- Stack-of-open-elements tree construction seed with void element handling,
  adjacent text merging, simple implied end tags, and unmatched end-tag
  diagnostics.
- Parser-driven tokenizer handoff for RCDATA, RAWTEXT, script data, and
  PLAINTEXT elements, preserving text-mode DOM content instead of lexing it as
  ordinary data-state markup.
- Implied `html`, `head`, and `body` document shell normalization, including
  preservation of explicit shell attributes and legacy omitted-wrapper pages.
- Scripting-aware parse options for parser-controlled tokenizer handoff, so
  `noscript` becomes RAWTEXT with scripting enabled and ordinary fallback
  markup with scripting disabled.
- Parser-approved initial tokenizer contexts, including foreign-content CDATA
  section fragments backed by the typed lexer CDATA context.
- Parser-approved initial script tokenizer contexts for script data, escaped,
  dash/dash-dash, less-than, and double-escaped substates backed by the typed
  lexer script-substate context helper.
