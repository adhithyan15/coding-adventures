# spice-netlist-parser

Small SPICE3 netlist parser that builds `spice_engine::Circuit` values.

```rust
use spice_netlist_parser::parse_netlist;

let parsed = parse_netlist(r#"
* RC low pass
V1 in 0 PULSE(0 1 0 1n 1n 10n 20n)
R1 in out 1k
C1 out 0 1u
.tran 1n 20n method=gear2
.end
"#)?;

assert_eq!(parsed.tran_cards().len(), 1);
```

This parser supports `R`, `C`, `L`, `V`, `I`, `D`, `Q`, `M`, `G`, `E`, `F`, and
`H` elements, `.model <name> D(...)` diode cards with `IS` and `VT`
parameters, `.model <name> NPN|PNP(...)` BJT cards with `IS`, `BF` /
`BETA_F`, `VT`, `CJE`, `CJC`, `TF`, and `TR` parameters,
`.model <name> NMOS|PMOS(...)` Level-1 MOSFET
cards with common SPICE aliases (`VT0` / `VTO`, `KP`, `LAMBDA`, `GAMMA`, `PHI`,
`W`, `L`, `IS`, `N_SUB` / `NSUB`, `T_NOM` / `TNOM`, `CGSO`, `CGDO`, `CGBO`,
`CBS`, and `CBD`), SPICE engineering
suffixes, capacitor `IC=<voltage>` and inductor `IC=<current>` initial
conditions, independent-source `AC <magnitude> [phase]` forms,
PWL/PULSE/SIN/EXP source forms, comments, `.end`, `.subckt` / `X` instance
expansion, and `.op`, `.tran`, `.dc`, `.ac`, `.tf`, `.sens`, `.mc`, `.noise`,
`.temp`, `.print`, `.plot`, and `.options` analysis cards.
Transient cards can carry `method=euler|trap|gear2`; when omitted,
`parsed.transient_method(None)?` falls back to `.options method=<...>` if
present.
