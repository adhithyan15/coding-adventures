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

Languages with a LANG84 release inventory (`code/grammars/<dir>/releases.csv`;
today Ruby, Perl and Python) are generated as **release families**: the release list comes from
the CSV, each row past stage `none` must have its own `<prefix><release>.tokens`
and `.grammar`, and releases whose pairs are identical apart from comments and
line numbers share one embedded module named `g_<first 12 hex of SHA-256>`.
The generated file exposes `SUPPORTED_RELEASES`, `token_grammar(release)` or
`parser_grammar(release)` (exact IDs only, `None` otherwise) and
`grammar_class(release)`; parser tables also carry the full `INVENTORY` so a
frontend can tell an unknown release from one whose pair is not written yet.
The unversioned pair, when a family keeps it, is `unversioned_parser_grammar()`
(and, for Perl, `unversioned_token_grammar()`), never the `""` release.
A family may also keep pre-LANG84 minor-line pairs (Python's `python3.12.*`)
for a legacy version API: they go in a separate `pub mod legacy_versions` with
its own selector, so no release ID can reach them. Release pairs are always
validated, even when a package's legacy grammars need `force`.

The CSV is read by header name, and a row is refused, failing the whole
generation, when it is quoted, has a different field count from the header,
has an unsafe ID (outside `[A-Za-z0-9._-]`, empty, or containing `..`,
because each ID becomes a file name) or a duplicate one, or names an unknown
`grammar_status` or `grammar_stage`. Perl's inventory predates the LANG84
schema's `grammar_stage` column, so its family derives the stage by the
spec's migration rule (`pending` → `none`, otherwise `P`).

```bash
grammar-tools generate-rust-compiled-grammars ruby     # 5 pairs
grammar-tools generate-rust-compiled-grammars perl     # 117 pairs -> 5 + 2 tables
grammar-tools generate-rust-compiled-grammars python   # 5 pairs -> 3 + 1 tables, + 6 minor lines
```

See `code/specs/LANG84-grammar-driven-versioned-frontends.md`.

Known validator gaps that are already handled safely by the consuming crates
are encoded in the Rust command itself, so repo regeneration does not require a
shell wrapper.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more validation errors |
| 2 | Usage error |
