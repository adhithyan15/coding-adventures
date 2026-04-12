# Changelog

All notable changes to the Java Lexer (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release of the TypeScript Java lexer package.
- `tokenizeJava(source, version?)` function that tokenizes Java source code using the grammar-driven lexer. The `version` parameter selects the Java edition: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"` (default: `"21"`).
- `createJavaLexer(source, version?)` function returning a configured `GrammarLexer` instance before tokenization begins. Useful for attaching on-token callbacks for context-sensitive lexing.
- Loads `java{version}.tokens` grammar files from `code/grammars/java/`.
- Supports Java keywords: `class`, `public`, `private`, `static`, `void`, `int`, `if`, `else`, `while`, `for`, `return`, `new`, `this`, `true`, `false`, `null`, etc.
- Supports Java operators: `==`, `!=`, `>=`, `<=`, `&&`, `||`, `++`, `--`, etc.
- Supports delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`.
- Clear error thrown for unrecognised version strings.
- Comprehensive test suite with v8 coverage.
