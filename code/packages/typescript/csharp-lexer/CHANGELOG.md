# Changelog

All notable changes to the C# Lexer (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release of the TypeScript C# lexer package.
- `tokenizeCSharp(source, version?)` function that tokenizes C# source code using the grammar-driven lexer. The `version` parameter selects the C# edition: `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`, `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"` (default: `"12.0"`).
- `createCSharpLexer(source, version?)` function returning a configured `GrammarLexer` instance before tokenization begins. Useful for attaching on-token callbacks for context-sensitive lexing.
- Loads `csharp{version}.tokens` grammar files from `code/grammars/csharp/`.
- Supports C# keywords: `class`, `namespace`, `using`, `public`, `private`, `protected`, `internal`, `static`, `void`, `int`, `string`, `bool`, `var`, `if`, `else`, `while`, `for`, `foreach`, `return`, `new`, `this`, `true`, `false`, `null`, `async`, `await`, `delegate`, `interface`, `struct`, `enum`, `abstract`, `sealed`, `override`, `virtual`, etc.
- Supports C#-specific operators: `??` (null-coalescing), `?.` (null-conditional), `=>` (lambda), `==`, `!=`, `>=`, `<=`, `&&`, `||`, `++`, `--`, etc.
- Supports delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`.
- Clear error thrown for unrecognised version strings.
- Comprehensive test suite with v8 coverage.
