# Changelog — grammar-tools (TypeScript program)

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
