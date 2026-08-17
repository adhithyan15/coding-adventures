# Changelog — grammar-tools (TypeScript program)

## [2.0.1] - 2026-08-17

### Added
- `--force` / `-f` flag on `compile-tokens`/`compile-grammar` — skip grammar validation and compile anyway (mirrors the flag the Ruby/Go/Rust ports already had). Needed because several `.grammar` files in `code/grammars/` (csharp, java, ecmascript es2018+, typescript ts3.0+) have pre-existing "unreachable rule" or "undefined rule reference" validation warnings that otherwise block compilation of their downstream `_grammar.ts` files — see the changelogs of `csharp-parser`, `java-parser`, `javascript-parser`, and `typescript-parser` for exactly which files needed it and why.

## [2.0.0] - 2026-03-26

### Added
- `compile-tokens <file.tokens> [-o out.ts]` — compiles a `.tokens` file to TypeScript source.
- `compile-grammar <file.grammar> [-o out.ts]` — compiles a `.grammar` file to TypeScript source.
- `--output` / `-o` flag — write generated code to a file instead of stdout.
- Status messages ("Compiling X... OK → path") go to stderr; generated code goes to stdout.
- 8 new CLI tests for compile commands.

## [1.0.0] - 2026-03-26

### Added
- Initial release. Replaces `src/cli.ts` in the library package.
- `validate`, `validate-tokens`, `validate-grammar` commands.
- Uses `@coding-adventures/cli-builder` for `--help`, `--version`, and parsing.
- Exit codes 0/1/2 identical to all other language implementations.
