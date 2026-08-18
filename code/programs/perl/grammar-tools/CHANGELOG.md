# Changelog — grammar-tools (Perl program)

## [1.0.0] - 2026-08-17

### Added
- Initial release.
- `validate`, `validate-tokens`, `validate-grammar` commands, cross-validating
  or individually validating `.tokens`/`.grammar` files.
- `compile-tokens`/`compile-grammar` commands — compile a `.tokens`/`.grammar`
  file into Perl source code that embeds the grammar as native (blessed)
  data structures, via `CodingAdventures::GrammarTools::Compiler`.
- `--output`/`-o` flag — write generated code to a file instead of stdout.
- Plain `@ARGV` parsing (no cli-builder package available for Perl).
- Exit codes 0/1/2 identical to the Ruby/Go/Rust/TypeScript/Elixir/Lua/Python
  counterparts.
- Part of the runtime-grammar-loading elimination campaign: this CLI is what
  Perl's `*-lexer`/`nib-parser` packages now use, at dev time, to generate
  the compiled `_Grammar.pm` modules they `require` at runtime instead of
  reading `.tokens`/`.grammar` files from disk.
