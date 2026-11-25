# pif-grammar

Parse harness and tests for the **Proof Interchange Format** grammar
([VER32](../../../specs/VER32-proof-interchange-format.md) section 4).

The canonical grammar is `code/grammars/pif/pif.tokens` and `pif.grammar`; see
[its README](../../../grammars/pif/README.md) for what it fixes and what it
leaves to the trusted reader. This crate embeds both files with `include_str!`
and runs them through the repository's `GrammarLexer` and `GrammarParser`.

```rust
use coding_adventures_pif_grammar::{lex, parse};

let tree = parse("pif 1\nlogic 1\nprofile native \"x\"\nfeatures\nend 0\n")?;
assert_eq!(tree.rule_name, "file");
assert!(lex("nstr 1  0 \"a\"\n").is_err()); // a doubled space is a lexical error
```

## Tests

`cargo test -p coding_adventures_pif_grammar` runs `tests/pif_grammar.rs`. The
VER32 section 4.6 worked example lives in `tests/fixtures/worked-example.pif`;
a test extracts the block from the spec and fails if the fixture has drifted.

## Where it fits

This is the grammar side of the Q1 differential (VER32 5.2, PIF-007): the
hand-written trusted reader (PIF-002) and a grammar-driven parser must accept
the same language. This crate is untrusted test infrastructure; it is not the
reader.
