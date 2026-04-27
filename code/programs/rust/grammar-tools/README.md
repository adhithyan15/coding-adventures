# grammar-tools (Rust program)

A standalone CLI for validating `.tokens` and `.grammar` files, compiling them
to embedded Rust modules, and regenerating repo-wide Rust `_grammar.rs` files.
This program wraps `grammar-tools` (the Rust library package) behind a
`cli-builder` interface.

## Usage

```bash
grammar-tools validate css.tokens css.grammar
grammar-tools validate-tokens css.tokens
grammar-tools validate-grammar css.grammar
grammar-tools compile-tokens code/grammars/sql.tokens -o code/packages/rust/sql-lexer/src/_grammar.rs
grammar-tools compile-grammar code/grammars/sql.grammar -o code/packages/rust/sql-parser/src/_grammar.rs
grammar-tools generate-rust-compiled-grammars
grammar-tools generate-rust-compiled-grammars sql dartmouth_basic mosaic
grammar-tools --help
```

`generate-rust-compiled-grammars` walks `code/packages/rust`, finds `*-lexer`
and `*-parser` crates with matching files under `code/grammars`, and writes
`src/_grammar.rs` for each target. Optional arguments filter by package name
(`sql-lexer`), package stem (`sql`), or grammar stem (`dartmouth_basic`).

Known validator gaps that are already handled safely by the consuming crates
are encoded in the Rust command itself, so repo regeneration does not require a
shell wrapper.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more validation errors |
| 2 | Usage error |
