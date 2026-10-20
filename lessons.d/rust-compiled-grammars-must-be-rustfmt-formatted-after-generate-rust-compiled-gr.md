---
category: Compiler / VM / language pipeline
---

# Rust compiled grammars must be rustfmt-formatted after generate-rust-compiled-grammars, or the diff is the whole file

`grammar-tools generate-rust-compiled-grammars javascript` writes unformatted
Rust: one array element per line, different wrapping. The committed
`javascript-parser/src/_grammar.rs` is rustfmt-formatted. So regenerating the
*unchanged* grammars produced a 64,000-line diff that looked like years of
drift (#17877). Running `rustfmt --edition 2021` on the regenerated file gave
a zero diff against the committed one.

Do this: after regenerating, run `rustfmt --edition 2021` on each
`_grammar.rs`, then diff. Before trusting any regeneration diff, regenerate
from the untouched grammars first: the baseline diff should be zero, so
whatever is left is your change. Revert generated files your change doesn't
own (here, `javascript-lexer/src/_grammar.rs` had real but unrelated
line-number drift) instead of folding them into the PR.
