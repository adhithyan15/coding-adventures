# cobol-parser (coding-adventures-cobol-parser)

Parser for **COBOL-60** — the syntactic layer of the COBOL frontend. It
tokenizes with [`coding-adventures-cobol-lexer`](../cobol-lexer) (which strips the
80-column card format and lexes PICTURE strings) and feeds the tokens to the
generic `parser::GrammarParser` driving the compiled `cobol.grammar`
(`code/grammars/cobol/cobol.grammar`). Nothing is hand-written.

Implements the parser layer of [PL07](../../../specs/PL07-cobol-60.md).

## API

```rust
use coding_adventures_cobol_parser::{parse_cobol, try_parse_cobol};

let cst = parse_cobol("000100 IDENTIFICATION DIVISION.\n000200 PROGRAM-ID. HELLO.\n\
                       000300 PROCEDURE DIVISION.\n000400 MAIN.\n000500     STOP RUN.");
assert_eq!(cst.rule_name, "program");
```

`parse_cobol` returns a generic `GrammarASTNode` CST rooted at `"program"`;
consumers walk it by `rule_name`.

## What it parses

The **demonstrated language** of PL07: a four-division program (IDENTIFICATION
and PROCEDURE required; ENVIRONMENT and DATA optional), `WORKING-STORAGE` data
entries with level numbers / `PICTURE` / `VALUE`, and PROCEDURE paragraphs of
sentences built from the core verbs (`MOVE`, `DISPLAY`, `ACCEPT`,
`ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE … GIVING`, `COMPUTE … = <expr>` with a
precedence-layered arithmetic expression (`+ - * / **`, unary sign, parentheses)
and optional `ROUNDED` / `ON SIZE ERROR`, `PERFORM`, `GO TO`, `IF … ELSE`,
`EVALUATE`, `STRING … DELIMITED BY … INTO`, `UNSTRING … DELIMITED BY … INTO`,
`STOP RUN`), plus a minimal
ENVIRONMENT (CONFIGURATION / INPUT-OUTPUT sections).
An operand may carry a **reference-modification** suffix —
`operand = NAME [ LPAREN operand COLON [ operand ] RPAREN ] | literal` — so a
data-name can be written `WS-NAME(2:3)` or `WS-NAME(3:)` (omitted length); a bare
NAME still parses exactly as before.

The long tail (full FD record descriptions, `REDEFINES`/`OCCURS`, `88`
conditions, editing PICTUREs, the complete reserved-word set) is future work; see
PL07.

## Regenerating the grammar

`src/_grammar.rs` is generated from `code/grammars/cobol/cobol.grammar`:

```
cargo run --release --manifest-path code/programs/rust/grammar-tools/Cargo.toml \
  -- generate-rust-compiled-grammars cobol
```
