# Perl lexer (Rust)

Tokenizes LANG81's bounded Perl 5.38 `print` arithmetic subset using the
checked-in `code/grammars/perl/perl.tokens` definition. This is not a full
Perl tokenizer.

It also carries the compiled token grammar of every checked release pair in
`code/grammars/perl/releases.csv` (LANG84). The 118 partial pairs share 5
distinct grammars, so `src/_grammar.rs` holds one module per distinct grammar
and maps each release ID to its module:

```rust
use coding_adventures_perl_lexer::{release_token_grammar, SUPPORTED_RELEASES};

let grammar = release_token_grammar("5.004_72").expect("a checked pair");
assert!(release_token_grammar("1.001").is_none()); // pending: no fallback
assert_eq!(SUPPORTED_RELEASES[0], "1.000");
```

Regenerate after editing a pair or flipping a CSV row with
`grammar-tools generate-rust-compiled-grammars perl`; this crate's tests fail
until the tables match the files.

The pilot integer token admits `0` or a nonzero first decimal digit. It
rejects leading-zero multi-digit forms at parse time; octal literals remain
outside this bounded grammar.
