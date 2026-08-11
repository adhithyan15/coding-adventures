# Changelog

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
