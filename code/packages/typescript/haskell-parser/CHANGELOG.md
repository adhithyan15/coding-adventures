# Changelog

All notable changes to the Haskell Parser (TypeScript) package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: now imports a pre-compiled `_grammar[_<version>].ts` per Haskell edition instead of `readFileSync`-ing the `.grammar` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.
- Corrected the module doc comment's version table (see haskell-lexer's matching changelog entry for detail).

## [0.1.0] - 2026-04-11

### Added
- Initial release of the TypeScript Haskell parser package.
- `parseHaskell(source, version?)` function that parses Haskell source code into generic `ASTNode` trees. The `version` parameter selects the Haskell edition: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"` (default: `"21"`).
- `createHaskellParser(source, version?)` function returning a configured `GrammarParser` instance before parsing begins.
- Loads `haskell{version}.grammar` files from `code/grammars/haskell/`.
- Delegates tokenization to `@coding-adventures/haskell-lexer`.
- Supports `var_declaration`, assignments, expression statements, and operator precedence.
- Clear error thrown for unrecognised version strings.
- Comprehensive test suite with v8 coverage.
