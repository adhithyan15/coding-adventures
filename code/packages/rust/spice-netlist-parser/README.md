# spice-netlist-parser

Small SPICE3 netlist parser that builds `spice_engine::Circuit` values.

```rust
use spice_netlist_parser::parse_netlist;

let parsed = parse_netlist(r#"
* RC low pass
V1 in 0 PULSE(0 1 0 1n 1n 10n 20n)
R1 in out 1k
C1 out 0 1u
.tran 1n 20n
.end
"#)?;

assert_eq!(parsed.tran_cards().len(), 1);
```

This first parser slice supports `R`, `C`, `L`, `V`, `I`, and `G` elements,
SPICE engineering suffixes, PWL/PULSE/SIN/EXP source forms, comments, `.end`,
`.subckt` / `X` instance expansion, and `.op`, `.tran`, `.dc`, and `.ac`
analysis cards.
