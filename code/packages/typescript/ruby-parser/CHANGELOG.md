# Changelog

All notable changes to the Ruby Parser (TypeScript) package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- `_grammar.ts` had drifted far behind `ruby.grammar`: `def`/`class`/`module`/`if`/`unless`/`while`/`until`/`case`/`begin`/`return`/`break`/`next`/`redo`/`retry`/`yield`/`alias`/`undef`/multi-assignment/modifier/rightward-assignment/index-assignment statement rules, pattern matching, blocks/lambdas, and the full operator-precedence chain were all missing from the compiled grammar, so real Ruby source using any of them would fail to parse or silently fall through to a generic expression rule. Because `parser.ts` never actually imported this file (it read `ruby.grammar` from disk directly), the staleness had zero runtime impact until now. Regenerated from the current `ruby.grammar`, and added regression tests (`def`/`class`/`if`/`while`/`case`/`begin` statements) that fail against the old compiled grammar and pass against the fix.
- Eliminated runtime grammar loading: `parseRuby` now imports the (now-correct) pre-compiled `_grammar.ts` instead of `readFileSync`-ing `ruby.grammar` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript Ruby parser package.
- `parseRuby()` function that parses Ruby source code into generic `ASTNode` trees.
- Loads `ruby.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/ruby-lexer`.
- Supports assignments, method calls, arithmetic expressions, and operator precedence.
- Comprehensive test suite with v8 coverage.
