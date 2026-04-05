# Changelog

All notable changes to `coding_adventures_ecmascript_es1_lexer` will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial release
- `CodingAdventures::EcmascriptEs1Lexer.tokenize(source)` method that tokenizes ES1 source code
- Loads `ecmascript/es1.tokens` grammar file and delegates to `GrammarLexer`
- Supports ES1 keywords: `break`, `case`, `continue`, `default`, `delete`, `do`, `else`, `for`, `function`, `if`, `in`, `new`, `return`, `switch`, `this`, `typeof`, `var`, `void`, `while`, `with`, `true`, `false`, `null`
- Supports ES1 operators: `==`, `!=`, `<=`, `>=`, `<`, `>`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=`, `>>>=`, `&&`, `||`, `!`, `++`, `--`, `?`
- Supports bitwise operators: `&`, `|`, `^`, `~`, `<<`, `>>`, `>>>`
- Supports numeric literals: decimal, float, hex (0x), scientific notation
- Supports string literals: single and double quoted with escape sequences
- Full test suite with SimpleCov coverage >= 80%
