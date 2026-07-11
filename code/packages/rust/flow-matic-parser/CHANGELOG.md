# Changelog

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
