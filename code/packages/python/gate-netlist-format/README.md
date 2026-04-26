# gate-netlist-format

HNL (Hardware NetList) data structures and JSON round-trip. The canonical format used downstream of synthesis. See [`code/specs/gate-netlist-format.md`](../../../specs/gate-netlist-format.md).

## Quick start

```python
from gate_netlist_format import (
    Netlist, Module, Port, Net, Instance, NetSlice, Direction, Level,
)

# Build a 4-bit adder netlist by hand
adder = Module(
    name="adder4",
    ports=[
        Port("a",   Direction.INPUT,  4),
        Port("b",   Direction.INPUT,  4),
        Port("cin", Direction.INPUT,  1),
        Port("sum", Direction.OUTPUT, 4),
        Port("cout",Direction.OUTPUT, 1),
    ],
    nets=[Net("c0", 1), Net("c1", 1), Net("c2", 1)],
    instances=[
        Instance(
            name="u_fa0",
            cell_type="full_adder",
            connections={
                "a":    NetSlice("a", (0,)),
                "b":    NetSlice("b", (0,)),
                "cin":  NetSlice("cin", (0,)),
                "sum":  NetSlice("sum", (0,)),
                "cout": NetSlice("c0", (0,)),
            },
        ),
        # ...
    ],
)

nl = Netlist(top="adder4", modules={"adder4": adder, ...})
report = nl.validate()
assert report.ok

nl.to_json("adder4.hnl.json")
```

## v0.1.0 scope

- HNL data classes: Netlist, Module, Port, Net, NetSlice, Instance.
- JSON round-trip via `to_json` / `from_json` / `to_json_str` / `from_json_str`.
- Schema version frozen at 0.1.0; rejects major-version mismatches on load.
- 27 built-in cell types (BUF, NOT, AND2-4, OR2-4, NAND2-4, NOR2-4, XOR2-3, XNOR2-3, MUX2, DFF + variants, DLATCH, TBUF, CONST_0/1).
- Validation rules R1-R7, R11: top exists, cell types resolve, input pins connected, connection keys are real pins, width match, net references exist, bits in range, no transitive self-instantiation.
- `Netlist.stats()` for cell counts.

## Out of scope (v0.2.0)

- EDIF / BLIF importer + exporter (the spec covers the format; will land alongside synthesis).
- Validation rules R8 (single-driver per net), R10 (combinational-loop detection).
- Streaming readers for >100K-cell designs.

MIT.
