---
category: Rust
---

# rustfmt on a crate's lib.rs also rewrites its generated _grammar.rs, so regenerate after formatting

**What went wrong.** LANG84 phase 4a regenerated `perl-lexer/src/_grammar.rs`
and `perl-parser/src/_grammar.rs` with `grammar-tools
generate-rust-compiled-grammars perl`, then ran `rustfmt --edition 2021
perl-lexer/src/lib.rs perl-parser/src/lib.rs …` to format the hand-written
code. rustfmt formats every out-of-line module a file declares, so
`mod _grammar;` pulled the generated tables in too and rewrote their long
`match` arms. The crates still compiled and every test passed; only a
byte comparison against a fresh regeneration (`cmp`) showed the committed
file was no longer the generator's output. The repository's regeneration
checks (`rust-grammar-regen-check`, the Mosaic job) diff exactly that, so a
formatted table fails them, and the next person to regenerate gets a
whitespace-only diff they did not cause.

**Fix.** Regenerate after formatting, and compare the result with `cmp`
before committing.

**Next time.** Format hand-written files with `rustfmt --skip-children`
(nightly) or by naming only leaf files, never a `lib.rs` that declares a
generated module; or simply run the generator last. Treat any generated file
as owned by its generator: the check for it is "regenerating produces no
diff", not "it compiles".
