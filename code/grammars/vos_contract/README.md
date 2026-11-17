# `vos_contract`: the grammar of `contract.vos`

`vos_contract.tokens` and `vos_contract.grammar` describe `contract.vos`, the part of a
verified-OS contract package that is not Warrant: the interface name, the tiers it
defines, the configuration names with their values, and the per-operation ceilings
([VOS02](../../specs/VOS02-interface-contracts-and-profile-linking.md) sections 2.5 and
6.8, decision D-GD6).

```text
vos-contract 1
name vos-sched-contract
iface sched
tier any
tier canonical
config N 8
ceiling dispatch steps 160
end
```

The pair is read by the generic `GrammarLexer` and `GrammarParser`; there is no
hand-written lexer or parser anywhere in the path.  It is consumed by
[`rust/vos-manifest`](../../packages/rust/vos-manifest), which embeds both files with
`include_str!` and adds the checks a grammar cannot make: the class and length of each
name, that a number fits `u64`, sorted and unique rows, row-count caps, and the
canonical-form comparison (parse, re-emit, compare byte for byte).

## What the grammar fixes and what it leaves

| Grammar | After the parse (`vos-manifest`) |
|---|---|
| order of rows, the shape of each row | version word is `1` |
| `DEC` is `0` or has no leading zero, at most 20 digits | `DEC` fits `u64` |
| a space between words is skipped; any other byte is a lexical error | one spelling: re-emit equals input |
| words are matched by value (`"tier"`) | `PKG`/`IDENT`, `OPNAME`, `KEY` classes and 32-byte cap; tiers, metrics; sorted, unique; row caps |

## Validation

`rust/vos-manifest/tests/grammars.rs` runs `validate_token_grammar`,
`validate_parser_grammar` and `cross_validate` over the pair (no issues allowed) and
probes the engine facts the grammar's comments rely on.
