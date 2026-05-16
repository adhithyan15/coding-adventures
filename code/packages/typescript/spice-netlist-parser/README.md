# @coding-adventures/spice-netlist-parser

Small SPICE3 netlist parser that builds `@coding-adventures/spice-engine`
`Circuit` objects.

```ts
import { parseNetlist } from "@coding-adventures/spice-netlist-parser";

const netlist = parseNetlist(`
* RC low pass
V1 in 0 PULSE(0 1 0 1n 1n 10n 20n)
R1 in out 1k
C1 out 0 1u
.tran 1n 20n
.end
`);

netlist.circuit.elements();
netlist.analyses;
```

This parser supports `R`, `C`, `L`, `V`, `I`, `D`, `Q`, `M`, `G`, `E`, `F`, and `H`
elements, `.model <name> D(...)` diode cards with `IS` and `VT` parameters,
`.model <name> NPN(...)` / `.model <name> PNP(...)` BJT cards with `IS`,
`BF` / `BETA_F`, and `VT` parameters, `.model <name> NMOS(...)` /
`.model <name> PMOS(...)` MOSFET cards with Level-1 `VT0` / `VTO`, `KP`,
`LAMBDA`, `GAMMA`, `PHI`, `W`, `L`, `IS`, `N_SUB` / `NSUB`, and `T_NOM` /
`TNOM` parameters, capacitor `IC=<voltage>` and inductor `IC=<current>`
initial conditions,
SPICE engineering suffixes, PWL/PULSE/SIN/EXP source forms, comments, `.end`,
`.subckt` / `X` instance expansion, and `.op`, `.tran`, `.dc`, `.ac`, and
`.tf`, `.sens`, `.mc`, and `.noise` analysis cards.
