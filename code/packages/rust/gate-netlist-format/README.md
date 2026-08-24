# `gate-netlist-format`

HNL — the Hardware NetList format. The canonical interchange format of the ASIC
synthesis pipeline.

A netlist is the answer to "what gates, wired how?" It sits *below* the HIR
(which still has `if` and `+` in it) and *above* physical layout (which has
rectangles and metal layers). Everything in the back end either produces one of
these or consumes one, which is exactly why it deserves its own crate rather than
living inside whichever tool happened to need it first.

```
HIR  →  (synthesis)  →  HNL[GENERIC]  →  (tech-mapping)  →  HNL[STDCELL]
                                                                │
                                            asic-floorplan  ◄───┘
                                            asic-placement
                                            asic-routing
                                            gdsii-writer
```

---

## Two levels, one type

A `Netlist` carries a `Level`, and the level is the whole story about how
abstract it is:

| Level | Instances name | Produced by | Meaning |
|-------|----------------|-------------|---------|
| `GENERIC` | built-in cells — `AND2`, `OR2`, `DFF`, … | `synthesis` | Technology-independent primitives. Correct logic, no silicon committed to yet |
| `STDCELL` | real library cells — `sky130_fd_sc_hd__and2_1` | `tech-mapping` | One-to-one with physical standard cells in an actual process |

Keeping both levels in one type — rather than two near-identical structs — is
what lets the validator, the statistics pass and the JSON codec be written once.
`BUILTIN_CELL_TYPES` and `CellTypeSig` define the GENERIC vocabulary, so
"is `AND3` a real primitive, and how many pins does it have?" has a single
answer.

## The JSON schema (`format: "HNL"`, `version: "0.1.0"`)

```json
{
  "format": "HNL", "version": "0.1.0", "level": "generic", "top": "adder4",
  "modules": {
    "adder4": {
      "ports": [{"name": "a", "dir": "input", "width": 4}],
      "nets":  [{"name": "_n0", "width": 1}],
      "instances": [{"name": "xor_0", "type": "XOR2",
                     "connections": {"A": {"net": "a", "bits": [0]},
                                     "B": {"net": "b", "bits": [0]},
                                     "Y": {"net": "_n0", "bits": [0]}}}]
    }
  }
}
```

Note that a connection is a `NetSlice` — a net *plus a bit list* — not just a net
name. Buses are first-class, so wiring bit 2 of a 4-bit bus to a single-bit cell
pin is expressible directly instead of requiring the producer to pre-split every
bus into scalar nets.

## API surface

| Item | Role |
|------|------|
| `Netlist`, `Module`, `Instance`, `Net`, `Port`, `NetSlice`, `Direction`, `Level` | The data model |
| `NetlistError` | Parse / construction failures, named rather than stringly-typed |
| `ValidationReport` | Structural checks — dangling nets, width mismatches, unknown cell types |
| `NetlistStats` | Cell and net counts, for pipeline reporting |
| `BUILTIN_CELL_TYPES`, `CellTypeSig` | The GENERIC cell vocabulary and pin signatures |

## Consumers

`synthesis` (produces GENERIC), `tech-mapping` (GENERIC → STDCELL), `drc-lvs`
and `fpga-place-route-bridge`.

## Testing

```sh
cargo test -p gate-netlist-format -- --nocapture
```
