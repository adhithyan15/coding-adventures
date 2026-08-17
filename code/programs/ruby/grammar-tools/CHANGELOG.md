# Changelog — grammar-tools (Ruby program)

## [2.1.0] - 2026-08-17

### Added
- `generate-compiled-grammars --lane ruby` — regenerates only the Ruby-target `_grammar.rb` files. Unlike the default (every language), this never builds the Go/Rust binaries and never shells out to Python or Node, so it's safe to run in a CI job that only has a Ruby toolchain available. Backed by `CompiledGrammarGenerator#run_ruby_only`, which reuses the exact same `compile_ruby_*` calls as the full run — nothing is duplicated.
- Extended the generator's coverage to include `css`, `lisp`, `nib`, `dartmouth_basic`, `mosaic`, `brainfuck`, and `csharp` (versioned) for Ruby, closing the gap that let several `_grammar.rb` files go ungenerated or drift stale unnoticed (see `coding_adventures_ruby_parser` v0.1.2, `coding_adventures_javascript_lexer` v0.2.1, and `coding_adventures_javascript_parser` v0.2.1 for real drift this caught on first run).

## [2.0.0] - 2026-03-26

### Added
- `compile-tokens <file.tokens> [-o out.rb]` — compiles a `.tokens` file to Ruby source code.
- `compile-grammar <file.grammar> [-o out.rb]` — compiles a `.grammar` file to Ruby source code.
- `--output` / `-o` flag — write generated code to a file instead of stdout.
- Status messages ("Compiling X... OK → path") go to stderr; generated code goes to stdout.
- 8 new CLI tests for compile commands.

## [1.0.0] - 2026-03-26

### Added
- Initial release. Replaces `bin/grammar-tools` in the library package.
- `validate`, `validate-tokens`, `validate-grammar` commands.
- Uses `CodingAdventures::CliBuilder` for `--help`, `--version`, and parsing.
- Exit codes 0/1/2 identical to all other language implementations.
