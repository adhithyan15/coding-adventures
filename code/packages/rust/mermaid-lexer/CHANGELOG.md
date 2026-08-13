# Changelog

## 0.45.0

- Tokenize quadrant point class definitions, class references, and inline styles.

## 0.44.0

- Add a portable Mermaid 11.16.1 quadrant-chart token grammar and lexer.

## 0.43.0

- Preserve internal percent signs in Mermaid state identifiers while retaining single-percent bare states.

## 0.42.0

- Tokenize Mermaid state HTML line-break variants, including whitespace before the optional slash, as semantic `LINE_BREAK` tokens.

## 0.41.0

- Skip pinned state-diagram `#` comments without consuming entities or hexadecimal style colors.

## 0.40.0

- Tokenize Mermaid state entities distinctly from hash colors.

## 0.39.0

- Tokenize the pinned state `hide empty description` directive.

## 0.38.0

- Recognize state scale-width statements from the pinned grammar.

## 0.37.0

- Recognize state diagram title statements from the pinned grammar.

## 0.36.0

- Tokenize state concurrent-region dividers distinctly from transition arrows.

## 0.35.0

- Tokenize state composite-group braces from the pinned grammar.

## 0.34.0

- Tokenize state click links, href markers, URLs, and tooltips.

## 0.33.0

- Tokenize state accessibility titles and single-line or multiline descriptions.

## 0.32.0

- Tokenize multiline state note terminators and floating-note strings.

## 0.31.0

- Tokenize attached state note keywords from the pinned Mermaid grammar.

## 0.30.0

- Tokenize Mermaid state `:::` inline class separators from the pinned grammar.

## 0.29.0

- Recognize Mermaid state `classDef` and `class` statements in the portable lexer grammar.

## 0.28.0

- Tokenize state inline-style separators and hexadecimal colors without folding them into labels.

## 0.27.0

- Tokenize angle-bracket and bracket Mermaid state fork/join markers.

## 0.26.0

- Tokenize both Mermaid 11.16.1 state choice marker spellings.

## 0.25.0

- Add a grammar-driven Mermaid 11.16.1 state-diagram lexer for declarations, transitions, directions, and edge states.

## 0.23.0

- Match and canonicalize Mermaid 11.16.1 sequence keywords case-insensitively.

## 0.22.0

- Skip Mermaid 11.16.1 sequence `#` comments while preserving numeric and named entities.

## 0.21.0

- Tokenize sequence `wrap:` and `nowrap:` directives atomically.

## 0.20.0

- Tokenize Mermaid sequence HTML line-break variants atomically.

## 0.19.0

- Tokenize Mermaid sequence numeric and named entity codes atomically.

## 0.18.0

- Tokenize sequence `hsl()` and `hsla()` colors atomically.

## 0.17.0

- Tokenize semicolons as sequence statement terminators.

## 0.16.0

- Tokenize multiline sequence accessibility-description blocks atomically.

## 0.15.0

- Tokenize sequence `accTitle` and `accDescr` statements.

## 0.14.0

- Tokenize sequence actor `details` references.

## 0.13.0

- Tokenize sequence actor property objects, including nested JSON values.

## 0.12.0

- Tokenize sequence actor-link URLs and JSON link maps.

## 0.11.0

- Reuse functional color tokens for sequence `rect` highlights.

## 0.10.0

- Tokenize autonumber decimals with Mermaid's two-place precision limit.

## 0.9.0

- Tokenize sequence central-connection markers.

## 0.8.0

- Tokenize all Mermaid 11.16.1 solid and dotted half-arrow forms.

## 0.7.0

- Tokenize inline sequence participant configuration objects.

## 0.6.0

- Tokenize sequence participant boxes and CSS functional colors.

## 0.5.0

- Added Mermaid sequence participant lifecycle tokens.

## 0.4.0

- Added tokens for every Mermaid 11.16.1 sequence control block and branch separator.

## 0.3.0

- Added a portable Mermaid 11.16.1 sequence token grammar and lexer entrypoint.

## 0.2.0

- Added the shared Mermaid 11.16.0 pie-chart token grammar and lexer entrypoint.

## 0.1.0

- Added a grammar-driven Mermaid flowchart lexer backed by `code/grammars/mermaid.tokens`.
