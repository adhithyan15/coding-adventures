# Changelog

## 0.1.1 — 2026-07-14

### Fixed — defense-in-depth recursion-depth cap

`create_flow_matic_parser`/`try_parse_flow_matic` built their
`GrammarParser` with no recursion-depth cap. Tracing every rule in this
grammar (`program = {statement} [program_end]`, `statement`'s clauses are
all flat) confirms there is **no recursive shape at all** — no rule
references `statement`/`clause`/`program` back, directly or through any
repetition/optional. There is no adversarial deep-nesting DoS vector to
calibrate against here.

Added `MAX_RULE_DEPTH` set to the shared crate's generic
`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH` (128) anyway, for
defense-in-depth and consistency with the rest of this sweep — comfortably
above the grammar's real maximum call depth, so it can never reject a
legitimate FLOW-MATIC program. One new regression test confirms a program
with 200 flat (non-nested) numbered statements still parses cleanly under
the cap.

## 0.1.0 — FLOW-MATIC parser (PL06)

- Grammar-driven parser over `code/grammars/flow_matic/flow_matic.grammar`,
  wrapping `parser::GrammarParser`. Public API: `parse_flow_matic` /
  `try_parse_flow_matic` / `create_flow_matic_parser`. CST rooted at `"program"`.
- Grammar covers the demonstrated language: numbered operations of
  `;`-separated clauses ended by `.`; `INPUT`/`OUTPUT`/`HSP`; `COMPARE … WITH …`
  with the three-way `IF`/`OTHERWISE` branch and the `END OF DATA` condition;
  `TRANSFER`/`MOVE`, `JUMP`, `READ-ITEM`/`WRITE-ITEM`, `TEST … AGAINST …`,
  `REWIND`, `CLOSE-OUT FILES`, `STOP`; and the trailing `(END)` program marker.
- Operation labels `(0)` vs field qualifiers `(A)` are disambiguated
  structurally (`LPAREN NUMBER RPAREN` vs `NAME LPAREN NAME RPAREN`), and the
  `CLOSE-OUT FILES C ; D` name-separator overlap is resolved by PEG greediness.
- Tests parse each clause type and the full canonical inventory-pricing program
  end to end.
