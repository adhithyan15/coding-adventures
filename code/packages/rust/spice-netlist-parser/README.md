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

This parser supports `R`, `C`, `L`, `V`, `I`, `D`, `Q`, `M`, `G`, `E`, `F`, and
`H` elements, `.model <name> D(...)` diode cards with `IS` and `VT`
parameters, `.model <name> NPN|PNP(...)` BJT cards with `IS`, `BF` /
`BETA_F`, and `VT` parameters, `.model <name> NMOS|PMOS(...)` Level-1 MOSFET
cards with common SPICE aliases (`VT0` / `VTO`, `KP`, `LAMBDA`, `GAMMA`, `PHI`,
`W`, `L`, `IS`, `N_SUB` / `NSUB`, and `T_NOM` / `TNOM`), SPICE engineering
suffixes, independent-source `AC <magnitude> [phase]` forms, PWL/PULSE/SIN/EXP
source forms, comments, `.end`, `.subckt` / `X` instance expansion, and
`.op`, `.tran`, `.dc`, and `.ac` analysis cards.
