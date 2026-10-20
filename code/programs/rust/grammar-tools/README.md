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

Some families are versioned: one module per release (C#, Java, Python, …).
Perl is the first **inventory-driven** family (LANG84). Its release list
comes from `code/grammars/perl/releases.csv`, read by header name: every
non-pending row is compiled from its own `perl<release>.tokens`/`.grammar`,
and releases whose grammars match after dropping comment lines and zeroing
line numbers share one module, named `g_<first 12 hex of SHA-256>` of that
canonical form. The generated file exports `INVENTORY_RELEASES`,
`SUPPORTED_RELEASES`, `token_grammar(release)` / `parser_grammar(release)`
(returning `None` for pending or unknown IDs, never a fallback) and
`unversioned_*()` for the unversioned pair. Release IDs must match
`[A-Za-z0-9._-]` without `..`, because each one becomes a file name.

```bash
grammar-tools generate-rust-compiled-grammars perl   # 117 pairs -> 5 + 2 tables
```

Known validator gaps that are already handled safely by the consuming crates
are encoded in the Rust command itself, so repo regeneration does not require a
shell wrapper.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more validation errors |
| 2 | Usage error |
