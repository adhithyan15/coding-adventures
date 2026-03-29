# Changelog — coding-adventures-typescript-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.typescript_lexer`.
- `tokenize(source)` — tokenizes a TypeScript string using the shared
  `typescript.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/typescript.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative
  to the installed module, avoiding hardcoded absolute paths.
- Full token set: all JavaScript tokens (NAME, NUMBER literal, STRING
  literal, LET, CONST, VAR, IF, ELSE, WHILE, FOR, DO, FUNCTION, RETURN,
  CLASS, IMPORT, EXPORT, FROM, AS, NEW, THIS, TYPEOF, INSTANCEOF, TRUE,
  FALSE, NULL, UNDEFINED, and all operators and delimiters) plus
  TypeScript-specific keywords: INTERFACE, TYPE, ENUM, NAMESPACE, DECLARE,
  READONLY, PUBLIC, PRIVATE, PROTECTED, ABSTRACT, IMPLEMENTS, EXTENDS,
  KEYOF, INFER, NEVER, UNKNOWN, ANY, VOID, NUMBER (keyword), STRING
  (keyword), BOOLEAN, OBJECT, SYMBOL, BIGINT.
- Comprehensive busted test suite covering: inherited JavaScript keywords,
  TypeScript-specific keywords, access modifiers, primitive type keywords,
  TypeScript constructs (type annotations, generics, interfaces, enums,
  abstract classes, implements/extends, keyof, as), whitespace handling,
  position tracking, and error cases.
- `required_capabilities.json` declaring `filesystem:read` (reads grammar
  file at startup).
- `BUILD` and `BUILD_windows` scripts with transitive dependency
  installation in leaf-to-root order.
