# PL06 — FLOW-MATIC (B-0, 1955–1959)

## Overview

**FLOW-MATIC** — originally **B-0** ("Business Language version 0") — was the
first English-like data-processing programming language. Grace Hopper and her
team at Remington Rand (the UNIVAC division) developed it between roughly 1955
and 1959, and it ran on the **UNIVAC I**, the first commercial computer sold in
the United States.

Its historical importance is hard to overstate: FLOW-MATIC is the direct
ancestor of COBOL. When the CODASYL committee designed COBOL in 1959, FLOW-MATIC
was the most influential of the existing languages they drew from. Three of its
ideas passed straight into COBOL — and into this excavation:

1. **English-keyword verbs** — `COMPARE`, `TRANSFER`, `MOVE`, `WRITE-ITEM`
   instead of numeric opcodes, so business logic reads like instructions.
2. **Hyphenated data names** — `PRODUCT-NO`, `UNIT-PRICE`, `FILE-A`. An
   identifier may contain internal hyphens. This is the defining lexical trait
   of the COBOL family.
3. **Description separated from procedure** — operation `(0)` *describes* the
   files; later operations *process* them. COBOL turned this into its DATA
   DIVISION / PROCEDURE DIVISION split.

This spec is the on-ramp to a future COBOL frontend (`PL0x-cobol`). FLOW-MATIC
gives us the English-verb, hyphenated-name machinery in a language whose
free-form listings do **not** yet require COBOL's fixed-column card handling.

## Historical Context

### The B-0 → FLOW-MATIC → COBOL lineage

Hopper had already built the **A-0 system** (1952), one of the first compilers,
to prove that a computer could translate symbolic instructions into machine
code. Many contemporaries did not believe a machine could usefully process
English-like commands, or that businesses would trust it. **B-0** (1955–1959)
answered by making the *language itself* readable to businesspeople, not just
engineers. Remington Rand released it commercially as **FLOW-MATIC** around
1958–1959. Its success was the working proof the COBOL committee built on.

### What a program looks like

A FLOW-MATIC program is a sequence of numbered **operations**. Each operation is
a parenthesised operation number, then one or more **clauses** separated by `;`,
terminated by a period `.`. The canonical published example is an
inventory-pricing run that merges a product file against a price file:

```
(0)  INPUT INVENTORY FILE-A PRICE FILE-B ; OUTPUT PRICED-INV FILE-C
       UNPRICED-INV FILE-D ; HSP D .
(1)  COMPARE PRODUCT-NO (A) WITH PRODUCT-NO (B) ;
       IF GREATER GO TO OPERATION 10 ;
       IF EQUAL GO TO OPERATION 5 ;
       OTHERWISE GO TO OPERATION 2 .
(2)  TRANSFER A TO D .
(3)  WRITE-ITEM D .
(4)  JUMP TO OPERATION 8 .
(5)  TRANSFER A TO C .
(6)  MOVE UNIT-PRICE (B) TO UNIT-PRICE (C) .
(7)  WRITE-ITEM C .
(8)  READ-ITEM A ; IF END OF DATA GO TO OPERATION 14 .
(9)  JUMP TO OPERATION 1 .
```

Reading it top to bottom: operation `(0)` names the input and output files and
routes the printer (`HSP`, high-speed printer); `(1)` compares the product
numbers of the two current records and branches three ways; the remaining
operations transfer, price, and write records, then read the next record and
loop. Even without knowing the language you can follow the business logic — the
entire point of Hopper's design.

### Free-form, not punched-card-rigid

Unlike COBOL's later fixed 80-column card format, FLOW-MATIC listings are
free-form: tokens are separated by whitespace and an operation ends at a period,
so a physical line break carries no meaning. An operation may wrap across
several printed lines (as `(0)` does above). This is why the lexer needs **no**
`pre_tokenize` column-strip hook — see "Lexer" below.

## Layer Position

```
                 FLOW-MATIC source (a UNIVAC listing)
                          ↓
          ┌───────────────────────────────────┐
          │  Lexer  (flow_matic.tokens)       │   ← this spec, PR1
          │  NUMBER, NAME, KEYWORD, ( ) . ;   │
          └───────────────────────────────────┘
                          ↓ Vec<Token>
          ┌───────────────────────────────────┐
          │  Parser (flow_matic.grammar)      │   ← this spec, PR2
          │  program → statement → clause     │
          └───────────────────────────────────┘
                          ↓ GrammarASTNode (CST)
          ┌───────────────────────────────────┐
          │  (future) IR / interpreter        │   ← out of scope here
          └───────────────────────────────────┘
```

**Rust crates:** `code/packages/rust/flow-matic-lexer/` (PR1),
`code/packages/rust/flow-matic-parser/` (PR2).
**Grammar files:** `code/grammars/flow_matic/flow_matic.tokens` and
`flow_matic.grammar`.

Both crates are thin wrappers over the shared `lexer::GrammarLexer` /
`parser::GrammarParser` — nothing is hand-written, per repo convention.

## Scope

This spec covers the **frontend only**: lexing and parsing FLOW-MATIC into a
concrete syntax tree. It deliberately stops there — there is no IR, bytecode, or
interpreter yet. The goal is to establish the English-verb, hyphenated-name
frontend machinery that the COBOL excavation will reuse.

The grammar targets the **demonstrated language** — the constructs that appear in
the canonical inventory-pricing example plus their close relatives. FLOW-MATIC's
full verb set (arithmetic `ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE`, `SET`,
`EXECUTE`, `DEFINE`, …) is recognised by the **lexer** so that any period-era
program tokenises cleanly, but the **v1 parser grammar** covers the demonstrated
subset. Extending the parser to the full verb set is a natural follow-on.

## Lexer (`flow_matic.tokens`)

Implemented in PR1. The lexer is a **pure grammar wrapper with no hooks** — the
simplest kind of frontend in the repo.

### Token inventory

| Token | Pattern | Examples | Notes |
|-------|---------|----------|-------|
| `NUMBER` | `[0-9]+` | `0`, `10`, `14` | Operation labels and branch targets |
| `NAME` | `[a-z][a-z0-9]*(-[a-z0-9]+)*` | `PRODUCT-NO`, `FILE-A`, `A` | Internal hyphens allowed |
| `KEYWORD` | the English verbs / clause words | `COMPARE`, `WITH`, `IF`, `GO`, `WRITE-ITEM` | Promoted from `NAME` |
| `LPAREN` / `RPAREN` | `(` / `)` | | Wrap both `(0)` labels and `(A)` qualifiers |
| `PERIOD` | `.` | | Terminates an operation |
| `SEMICOLON` | `;` | | Separates clauses |

Whitespace — **including newlines** — is skipped. Any other character is an
`UNKNOWN` lexical error.

### Two design decisions

**No custom operation-number token.** An operation label `(0)` and a field
qualifier `(A)` both lex as `LPAREN … RPAREN`. We do *not* invent an `OP_NUMBER`
token; the parser distinguishes them structurally — a label is `"(" NUMBER ")"`
at the head of a statement, a field reference is `NAME "(" NAME ")"`. Keeping the
lexer dumb here means the grammar, not a fragile regex, owns the distinction.

**Case is insignificant, original case preserved.** The UNIVAC's console and
printers were uppercase-only, exactly like the teletypes that later shaped
Dartmouth BASIC. Following the same reasoning BASIC uses, we set
`case_sensitive: false` (patterns match regardless of case) and
`# @case_insensitive true` (keyword lookup is case-folded). The result:

- **Keywords** are normalized to uppercase (`compare` → `KEYWORD("COMPARE")`).
- **NAME values** preserve the case they were typed in, so canonical all-caps
  source keeps `PRODUCT-NO` intact, while lowercase input still lexes correctly.

The hyphenated verbs `WRITE-ITEM`, `READ-ITEM`, and `CLOSE-OUT` are matched by
the `NAME` pattern and then promoted to `KEYWORD` via the keyword set — they are
listed there one per line (the grammar-tools parser stores each `keywords:` line
verbatim and does not split on spaces).

## Parser (`flow_matic.grammar`) — implemented in PR2 (#7963)

The parser wraps `parser::GrammarParser` (a recursive-descent PEG parser with
packrat memoization) and produces a generic CST rooted at `program`. The
implemented productions for the demonstrated subset:

```
program      = { statement } [ program_end ] ;
program_end  = LPAREN "END" RPAREN ;
statement    = LPAREN NUMBER RPAREN clause { SEMICOLON clause } PERIOD ;

clause       = input_clause | output_clause | hsp_clause
             | compare_clause | if_clause | otherwise_clause
             | transfer_clause | move_clause | jump_clause
             | read_item_clause | write_item_clause
             | test_clause | rewind_clause | closeout_clause | stop_clause ;

field        = NAME LPAREN NAME RPAREN ;             (* PRODUCT-NO (A) *)
target       = "OPERATION" NUMBER ;
condition    = "GREATER" | "EQUAL" | "LESS" | ( "END" "OF" "DATA" ) ;

input_clause  = "INPUT"  file_pair { file_pair } ;   (* logical-name FILE-x pairs *)
output_clause = "OUTPUT" file_pair { file_pair } ;
file_pair     = NAME NAME ;
hsp_clause    = "HSP" NAME ;
compare_clause   = "COMPARE" field "WITH" field ;
if_clause        = "IF" condition "GO" "TO" target ;
otherwise_clause = "OTHERWISE" "GO" "TO" target ;
transfer_clause  = "TRANSFER" NAME "TO" NAME ;
move_clause      = "MOVE" field "TO" field ;
jump_clause      = "JUMP" "TO" target ;
read_item_clause  = "READ-ITEM"  NAME ;
write_item_clause = "WRITE-ITEM" NAME ;
test_clause     = "TEST" field "AGAINST" ( NAME | NUMBER ) ;
rewind_clause   = "REWIND" NAME ;
closeout_clause = "CLOSE-OUT" "FILES" NAME { SEMICOLON NAME } ;
stop_clause     = "STOP" [ LPAREN "END" RPAREN ] ;
```

**Divergences from the PR1 sketch, and why** (per repo spec-sync policy):

- **`END OF DATA` is a `condition`, and `read_item_clause` is just
  `"READ-ITEM" NAME`.** The PR1 sketch nested the end-of-file test inside
  `read_item_clause` as `[ ";" if_clause ]`. But in the source it is written as
  a `;`-separated *sibling* clause (`READ-ITEM A ; IF END OF DATA GO TO
  OPERATION 14`), so the implemented grammar treats it uniformly as another
  clause under the statement's `{ SEMICOLON clause }` loop — simpler and
  consistent with every other `;`-separated clause. `END OF DATA` therefore
  became a fourth alternative of the extracted `condition` rule.
- **`program_end` was added** so the canonical program's trailing `(END)`
  marker (`(17) STOP . (END)`) parses; the whole program now parses end to end.
- **Token *types* (`LPAREN`, `RPAREN`, …) rather than value literals** are used
  for punctuation, for clarity.

Two structural quirks, both with explicit tests:

1. **Three-way branch in one statement** — `COMPARE … ; IF GREATER … ; IF EQUAL
   … ; OTHERWISE …` is a single statement whose clauses are `;`-separated.
2. **`CLOSE-OUT FILES C ; D .`** — here `;` separates *file names* within one
   clause, overlapping the clause separator. **PEG greediness** resolves it: the
   inner `{ SEMICOLON NAME }` runs first and consumes `; D` before the
   statement's own `{ SEMICOLON clause }` loop ever sees it — no backtracking
   needed.

## Test Strategy

### Lexer tests (PR1, implemented)

- Hyphenated names lex as one token (`PRODUCT-NO`, not `PRODUCT` − `NO`).
- The hyphenated verbs (`WRITE-ITEM`, `READ-ITEM`, `CLOSE-OUT`) promote to
  `KEYWORD`, while a keyword-prefix word (`INVENTORY`) stays a `NAME`.
- `(0)` lexes as `( NUMBER )` and `(A)` as `( NAME )` — the label-vs-qualifier
  distinction the parser relies on.
- Newlines are insignificant: a wrapped operation tokenises identically to a
  one-line one.
- Whole clauses from the canonical program (`COMPARE … WITH …`, `IF GREATER GO
  TO OPERATION 10`, `READ-ITEM A ; IF END OF DATA …`) produce the expected
  streams, and the two-operation program head tokenises with one EOF.

### Parser tests (PR2, implemented)

- Each clause type produces the expected CST shape.
- The three-way `IF/OTHERWISE` branch parses as one statement (two `if_clause` +
  one `otherwise_clause`), and `READ-ITEM A ; IF END OF DATA …` parses the
  end-of-file test as a sibling `if_clause` with an `END OF DATA` condition.
- `CLOSE-OUT FILES C ; D .` parses with both file names under one clause (exactly
  one `closeout_clause`, and no stray second clause).
- The full canonical program — all 18 operations plus the `(END)` marker —
  parses end to end.

## Future Extensions

| Feature | Notes |
|---------|-------|
| Full arithmetic verbs in the grammar | `ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE`, `SET` — lexed today, parsed later |
| `EXECUTE` / `DEFINE` subroutine forms | Recognised by the lexer; grammar TBD |
| IR / interpreter | Run the canonical program; out of scope for the frontend |
| COBOL frontend (`PL0x`) | Reuses the hyphenated-name + English-verb machinery; adds fixed-column `pre_tokenize` hook |
