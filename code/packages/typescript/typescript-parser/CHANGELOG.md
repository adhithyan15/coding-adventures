# Changelog

All notable changes to the TypeScript Parser (TypeScript) package will be documented in this file.

## [0.2.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `parseTypeScript`/`createTypeScriptParser` now import a pre-compiled `_grammar[_ts<version>].ts` per TypeScript edition instead of `readFileSync`-ing the `.grammar` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.
- Compiled every versioned `.grammar` file with the new `--force` flag (see csharp-parser's changelog): all 6 have pre-existing `type_annotation`/`type_predicate`-unreachable validation warnings, ts3.0+ additionally has `async_generator_expression`-unreachable, and ts5.0/ts5.8 additionally have `formal_parameter`/`formal_parameters` **undefined rule reference** errors — a dangling reference to a rule name that doesn't exist anywhere in the grammar. These are pre-existing content bugs in the grammar spec files (worth a follow-up fix), not something introduced by this change: the runtime path never validated before either, so behavior is unchanged, just no longer re-parsed from disk on every call.

## [0.2.0] - 2026-04-05

### Added
- `parseTypescript(source, version?)` — optional `version` parameter accepting
  `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`, `"ts5.0"`, or `"ts5.8"`.
  When omitted (or empty string), the generic grammars are used — backwards-compatible
  with v0.1.x.
- Versioned grammar support loads parser grammar from `code/grammars/typescript/<version>.grammar`
  and delegates to the versioned lexer grammar automatically.
- Clear error thrown for unrecognised version strings.
- Expanded test suite covering all six TS version strings, empty-string version,
  and error cases.

### Changed
- `parseTypescript` signature is now `(source: string, version?: string): ASTNode`
  — fully backwards-compatible; existing callers with one argument are unaffected.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript TypeScript parser package.
- `parseTypescript()` function that parses TypeScript source code into generic `ASTNode` trees.
- Loads `typescript.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/typescript-lexer`.
- Supports `var_declaration` (let/const/var), assignments, expression statements, and operator precedence.
- Comprehensive test suite with v8 coverage.
