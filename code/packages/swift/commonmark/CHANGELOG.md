# Changelog — commonmark (Swift)

All notable changes to this package are documented here.

## 0.1.0 — Initial release

- `toHtml(_ markdown: String) -> String` — single-function public API
- Chains `CommonmarkParser.parse(_:)` → `DocumentAstToHtml.render(_:)`
- 30+ end-to-end test cases covering all supported CommonMark features
