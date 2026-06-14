# coding-adventures-spice-netlist-parser

Small SPICE3 netlist parser that builds `spice_engine.Circuit` objects.

```python
from spice_netlist_parser import parse_netlist

netlist = parse_netlist("""
* RC low pass
V1 in 0 PULSE(0 1 0 1n 1n 10n 20n)
R1 in out 1k
C1 out 0 1u
.tran 1n 20n method=gear2
.end
""")

netlist.circuit.elements
netlist.analyses
```

This parser slice supports `R`, `C`, `L`, `V`, `I`, `D`, `Q`, `M`, `G`, `E`,
`F`, and `H` elements, `.model` cards for Shockley diode parameters (`IS`,
`VT`), BJT parameters (`NPN`/`PNP`, `IS`, `BF`/`BETA_F`, `VT`, `CJE`, `CJC`,
`TF`, `TR`), and Level-1
MOSFET parameters (`NMOS`/`PMOS`, `VT0`/`VTO`, `KP`, `LAMBDA`, `GAMMA`, `PHI`,
`W`, `L`, `IS`, `N_SUB`/`NSUB`, `T_NOM`/`TNOM`, `CGSO`, `CGDO`, `CGBO`,
`CBS`, `CBD`), capacitor `IC=<voltage>`
and inductor `IC=<current>` initial conditions, SPICE engineering suffixes,
PWL/PULSE/SIN/EXP source forms, comments, `.end`, `.subckt` / `X` instance
expansion, and `.op`, `.tran`, `.dc`, `.ac`, `.tf`, `.sens`, `.mc`, `.noise`,
`.temp`, `.print`, `.plot`, `.save`, `.probe`, `.measure`, `.four`, and
`.options` analysis cards. Transient cards can carry `method=euler|trap|gear2`;
when omitted,
`netlist.transient_method()` falls back to `.options method=<...>` if present.
Selected `.options` keys can also be turned into engine-call arguments with
`netlist.dc_op_kwargs()` and `netlist.transient_kwargs()`.
Runnable `.op`, `.dc`, `.ac dec` / `.ac log`, and `.tran` cards can be planned
and executed directly:

```python
from spice_netlist_parser import run_netlist

results = run_netlist("""
V1 in 0 DC 1 AC 1
R1 in out 1k
R2 out 0 1k
.op
.ac dec 1 1k 1k
.end
""")
```

`.save`, `.probe`, `.print`, and `.plot` cards can be applied to executed
analysis results with `netlist.select_outputs(results)`. Supported `.measure`
cards can be evaluated with `netlist.measure_results(results)`; the first
execution subset supports `FIND ... AT=<value>` plus `MAX`, `MIN`, `AVG`, and
`RMS` over optional `FROM=<value>` / `TO=<value>` ranges.

Deck-level `.temp` cards can be resolved into Kelvin with
`netlist.operating_temperature_kelvin()`, and
`netlist.noise_temperature_kelvin(noise_card)` applies the SPICE precedence
where an explicit `.noise temp=<kelvin>` overrides the deck operating
temperature.
