# flow-matic-parser (coding-adventures-flow-matic-parser)

Parser for **FLOW-MATIC** (B-0, 1955–1959) — the syntactic layer of the
frontend. It tokenizes with [`coding-adventures-flow-matic-lexer`](../flow-matic-lexer)
and feeds the tokens to the generic `parser::GrammarParser` driving the compiled
`flow_matic.grammar` (`code/grammars/flow_matic/flow_matic.grammar`). Nothing is
hand-written.

Implements the parser layer of [PL06](../../../specs/PL06-flow-matic.md).

## API

```rust
use coding_adventures_flow_matic_parser::{parse_flow_matic, try_parse_flow_matic};

let cst = parse_flow_matic("(2) TRANSFER A TO D .");
assert_eq!(cst.rule_name, "program");
```

`parse_flow_matic` returns a generic `GrammarASTNode` CST rooted at `"program"`;
consumers walk it by `rule_name`.

## What it parses

The **demonstrated language** — the constructs of the canonical
inventory-pricing program: numbered operations of `;`-separated clauses ended by
`.`, `INPUT`/`OUTPUT`/`HSP` file description, `COMPARE … WITH …` with the
three-way `IF … / OTHERWISE …` branch, `TRANSFER`/`MOVE`, `JUMP`,
`READ-ITEM`/`WRITE-ITEM`, `TEST … AGAINST …`, `REWIND`, `CLOSE-OUT FILES`, and
`STOP`, plus the trailing `(END)` program marker.

Two structural quirks, both tested:
- The three-way `IF … ; IF … ; OTHERWISE …` chain is one statement whose clauses
  are `;`-separated.
- `CLOSE-OUT FILES C ; D` uses `;` to separate *file names* within one clause;
  PEG greediness consumes `; D` inside the clause before the statement's own
  clause loop sees it.

## Regenerating the grammar

`src/_grammar.rs` is generated from `code/grammars/flow_matic/flow_matic.grammar`:

```
cargo run --release --manifest-path code/programs/rust/grammar-tools/Cargo.toml \
  -- generate-rust-compiled-grammars flow_matic
```
