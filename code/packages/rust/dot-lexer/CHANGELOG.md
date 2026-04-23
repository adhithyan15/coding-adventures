# Changelog — dot-lexer

## 0.1.0

Initial release.

- `tokenise(source: &str) -> LexResult` public API
- All DOT keyword tokens: `strict`, `graph`, `digraph`, `node`, `edge`, `subgraph`
- Punctuation: `{ } [ ] = ; , :`
- Edge operators: `->` (Arrow), `--` (DashDash)
- ID flavours: unquoted, numeral, double-quoted (with escape handling), HTML (balanced `<…>`)
- Line and block comment skipping
- Error recovery: collects errors, continues scanning
- Line and column tracking in every token
