# Changelog — grammar-tools (Go program)

## [2.2.0] - 2026-05-16

### Added (LANG63 — grammar-driven Twig lexer and CST parser)

Two new CLI commands that generate Twig source code from grammar files:

#### `compile-tokens-twig <file.tokens> [-o output.tw]`

Parses a `.tokens` file and generates the full `lexer.tw` module for the
self-hosted Twig compiler.  The generated file is a grammar-driven dispatch
chain identical in behaviour to the LANG58 hand-written lexer, but produced
mechanically so that changes to `twig.tokens` automatically propagate to the
Twig source.

Delegates to `GenerateTwigLexer` in the `grammar-tools` library package
(`twig_codegen.go`).

#### `compile-grammar-twig <file.grammar> [<file.tokens>] [-o output.tw]`

Parses a `.grammar` file (and optionally its companion `.tokens` file for
keyword / token-kind information) and generates the full `cst-parser.tw`
module — a grammar-driven recursive-descent CST parser.  Each grammar rule
becomes one Twig function; repetitions, optionals, and alternation sub-arms
produce additional named helper functions.

Delegates to `GenerateTwigParser` in the `grammar-tools` library package
(`twig_codegen.go`).

Both commands accept `--force` to skip validation before compilation and write
to stdout when `-o` is omitted.

## [2.1.0] - 2026-03-26

### Added
- `--package` / `-p` flag for `compile-tokens` and `compile-grammar` commands — sets the Go
  package name in generated output. Defaults to `"generated"` when omitted. Required when writing
  to a real package directory so the generated file has the correct `package` declaration.

## [2.0.0] - 2026-03-26

### Added
- `compile-tokens <file.tokens> [-o output.go]` — compiles a `.tokens` file to Go source code.
- `compile-grammar <file.grammar> [-o output.go]` — compiles a `.grammar` file to Go source code.
- `--output` / `-o` flag — write generated code to a file instead of stdout.
- 8 new CLI tests for compile commands.

## [1.0.0] - 2026-03-26

### Added
- Initial release. Replaces `cmd/grammar-tools/main.go` in the library package.
- `validate`, `validate-tokens`, `validate-grammar` commands.
- Uses `cli-builder` for `--help`, `--version`, and argument parsing.
- Exit codes 0/1/2 identical to all other language implementations.
