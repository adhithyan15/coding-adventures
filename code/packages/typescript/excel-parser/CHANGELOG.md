# Changelog

## 0.2.0

- **Browser-safe**: `parseExcelFormula` now consumes the grammar from the
  pre-compiled, embedded `_grammar.ts` (`PARSER_GRAMMAR`) instead of reading
  and parsing `code/grammars/excel.grammar` from disk at runtime. This drops
  the `fs`/`path`/`url` imports, so the package finally lives up to its
  "pure TypeScript" billing and bundles straight to the browser (the VisiCalc
  HTML demo bundles it transitively via `@coding-adventures/spreadsheet-engine`).
  The embedded grammar was verified structurally identical (order-insensitive)
  to the on-disk grammar, so the change is behaviour-preserving — all 562
  parser tests pass unchanged. Also shaves one grammar-parse step per call.
  Regenerate `_grammar.ts` with `grammar-tools compile-grammar excel.grammar`
  if `excel.grammar` ever changes.

## 0.1.0

- Initial Excel formula parser implementation
