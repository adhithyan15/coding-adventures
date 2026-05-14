# coding-adventures-spice-netlist-parser

Small SPICE3 netlist parser that builds `spice_engine.Circuit` objects.

```python
from spice_netlist_parser import parse_netlist

netlist = parse_netlist("""
* RC low pass
V1 in 0 PULSE(0 1 0 1n 1n 10n 20n)
R1 in out 1k
C1 out 0 1u
.tran 1n 20n
.end
""")

netlist.circuit.elements
netlist.analyses
```

This first parser slice supports `R`, `C`, `L`, `V`, `I`, and `G` elements,
SPICE engineering suffixes, PWL/PULSE/SIN/EXP source forms, comments, `.end`,
and `.op`, `.tran`, `.dc`, and `.ac` analysis cards.
