---
category: Rust
---

# A huge diff from regenerating a committed _grammar.rs may be only rustfmt, so format before calling it drift

LANG84 phase 6a avoided touching any `es*.grammar` file because regenerating
`javascript-parser/src/_grammar.rs` with
`grammar-tools generate-rust-compiled-grammars javascript` produced a
65,302-line diff. It wrote this up as old generator drift, and that steered
the phase's plan.

In phase 6d the same regeneration was needed. Running `rustfmt --edition 2021`
on the generated file reproduced the committed file **byte for byte**: the
committed copy is simply the generator's output after `rustfmt`. The "drift"
was formatting. The lexer's `_grammar.rs` happens to be committed unformatted,
so it never showed this.

**Do this:**
1. Before treating a regeneration diff as drift, run `rustfmt` on the
   generated file and diff again.
2. If the diff disappears, keep the formatted output; that is the committed
   convention for that file.
3. If it doesn't, check which fields change. 6d's real change, three header
   lines added to each `.grammar`, showed up only as `line_number` fields.

A large diff is a question to answer, not a reason to avoid touching a
grammar.
