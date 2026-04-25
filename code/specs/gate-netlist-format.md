# Gate Netlist Format (HNL)

## Overview

A **netlist** is the data structure that says, literally, "this output pin connects to that input pin, through that gate." It is the lingua franca of digital hardware downstream of behavioral description: synthesis produces a netlist, technology mapping rewrites a netlist, place & route consumes a netlist, GDSII is one extra step from a netlist. If HIR (the Hardware IR — see `hdl-ir.md`) describes *what the circuit means*, a netlist describes *what the circuit is*: a graph of cells and wires.

This spec defines **HNL** (Hardware NetList), the canonical netlist format used everywhere in the stack downstream of `synthesis.md`. HNL is a JSON-serializable Python data model. The format is opinionated:

- **JSON-first** — human-readable, diffable, embeddable in tests, scriptable from any language.
- **Hierarchy-preserving** — modules contain instances of other modules; flattening is a separate pass, not a wire-format requirement.
- **Cell-typed** — every instance has a `cell_type` that names either a built-in primitive (`AND`, `OR`, `DFF`, ...) or a user module.
- **Strongly typed** — width-checked nets, direction-checked ports, validated multi-driver / undriven invariants.
- **Round-trippable with industry formats** — bidirectional importers/exporters for **BLIF** (yosys/ABC interchange) and **EDIF** (FPGA tool interchange).

### Why a custom format?

| Format | Strength | Weakness |
|---|---|---|
| **EDIF** (Electronic Design Interchange Format) | Industry-standard interchange; FPGA tools speak it. | LISP-like S-expressions; verbose; no clean Python ergonomics; underspecified in places. |
| **BLIF** (Berkeley Logic Interchange Format) | Compact; ABC and yosys speak it natively. | Loses hierarchy (designed to be flat); no first-class hierarchical modules; no width-typed buses. |
| **Verilog structural** | Universal; any tool reads it. | A *language*, not a data structure. Parsing required to use it as a netlist. |
| **JSON HNL (this spec)** | Python-native, hierarchical, diffable, embeddable in tests, easy to validate. | Not industry-standard; we own its evolution. |

**Recommendation: use HNL as the canonical internal format; import/export EDIF and BLIF at the boundaries.** This pattern matches how every modern toolchain works — yosys's internal RTLIL is its own data structure, exported to Verilog/BLIF/JSON when needed.

### Generality

Although the worked example in this spec is the 4-bit adder (~30 cells), the same HNL data model scales without change to:

- A 32-bit ALU (~500 cells)
- A 4-stage RISC-V scalar core (~10K cells; see `arm1-gatelevel`, `intel4004-gatelevel` for reference designs already in the repo)
- A small SoC (~100K cells)
- Industrial ASIC blocks (millions of cells — though at that scale you'll want streaming readers, addressed in §10)

The data model is the same. Only the runtime constants change.

## Layer Position

```
HIR (behavioral or structural)              ◀── hdl-ir.md
        │
        ▼
┌───────────────────────────────┐
│  synthesis.md                  │
│  HIR → HNL (generic gates)     │
└───────────────────────────────┘
        │
        ▼
┌───────────────────────────────┐
│  HNL — generic                │  ◀── THIS SPEC
│  cells: AND/OR/NOT/XOR/DFF/...│
└───────────────────────────────┘
        │
        ├────────► tech-mapping.md ────► HNL — stdcell (cells: NAND2_X1, ...)
        │
        ├────────► fpga-place-route-bridge.md ────► fpga JSON config
        │
        └────────► real-fpga-export.md ────► structural Verilog ────► yosys/nextpnr
                                                                            │
                                                                            ▼
                                                                    HNL re-imported
                                                                    via BLIF/EDIF
```

**Inputs to this layer:**
- Synthesis output (canonical producer)
- BLIF importer (e.g., yosys-emitted designs)
- EDIF importer (e.g., third-party IP cores)
- Hand-written netlists (for testing / one-off circuits)

**Outputs from this layer:**
- Tech mapping (HNL → HNL with stdcell types)
- FPGA P&R bridge (HNL → JSON config of existing `fpga` package)
- Real-FPGA export (HNL → structural Verilog or EDIF for `yosys`/`nextpnr`)
- Validation tools (HNL → pass/fail report)

## Concepts

### A circuit is a hypergraph

A combinational gate-level circuit is a directed hypergraph. Each *net* is a hyperedge that connects one driver pin to one or more sink pins. Each *cell* is a node with input pins, output pins, and a behavioral function (the truth table of `AND`, `OR`, etc.).

```
   Net "x"            Net "y"
   driver: A.Y        driver: A.Y also (multi-driver — INVALID without tristate)
   sinks:  B.A, C.A   sinks:  D.A
   ─────────┐         ─────────┐
            │                  │
      ┌─────▼─────┐      ┌─────▼─────┐
      │  Cell A   │      │  Cell B   │ ...
      └───────────┘      └───────────┘
```

Three invariants distinguish a *valid* netlist from a soup of references:

1. **Single-driver per net** (unless explicitly tristate / wired-or — annotated).
2. **Width compatibility** — a 4-bit net cannot drive a 1-bit input pin.
3. **No floating sinks unless explicitly tied** — every input pin must have a defined source.

### Cells, ports, pins, nets — the mental model

| Concept | Meaning | Example |
|---|---|---|
| **Module** | A definition. Contains nets and instances. | `module adder4(input [3:0] a, b, output [4:0] sum);` |
| **Port** | An external interface point of a module. Has direction and width. | `input [3:0] a` |
| **Cell type** | The "kind" of a thing — either a built-in primitive (`AND`, `DFF`, ...) or another module. | `AND2`, `full_adder` |
| **Cell** (a.k.a. *Instance*) | An instantiation of a cell type inside a module. Has a name. | `u_fa0 : full_adder` |
| **Pin** | An interface point on a cell. Has direction and width. Comes from the cell type's port list. | `u_fa0.a` |
| **Net** | A wire inside a module. Connects pins. Has a name and width. | `c0` (the carry-out of bit 0) |

A pin is to a port what an instance is to a module: the *use site* vs the *definition site*.

### Built-in primitive cell types

HNL defines a fixed set of built-in cell types. The `synthesis.md` pass produces only these. Tech mapping (`tech-mapping.md`) replaces them with library cells.

| Type | Inputs | Outputs | Function |
|---|---|---|---|
| `BUF` | A | Y | Y = A |
| `NOT` | A | Y | Y = ¬A |
| `AND2`, `AND3`, `AND4` | A, B[, C[, D]] | Y | Y = A·B·… |
| `OR2`, `OR3`, `OR4` | A, B[, C[, D]] | Y | Y = A+B+… |
| `NAND2`, `NAND3`, `NAND4` | A, B[, C[, D]] | Y | Y = ¬(A·B·…) |
| `NOR2`, `NOR3`, `NOR4` | A, B[, C[, D]] | Y | Y = ¬(A+B+…) |
| `XOR2`, `XOR3` | A, B[, C] | Y | Y = A⊕B(⊕C) |
| `XNOR2`, `XNOR3` | A, B[, C] | Y | Y = ¬(A⊕B(⊕C)) |
| `MUX2` | A, B, S | Y | Y = S?B:A |
| `DFF` | D, CLK | Q | Q ← D on posedge CLK |
| `DFF_R` | D, CLK, R | Q | Q ← D on posedge CLK; Q ← 0 when R=1 |
| `DFF_S` | D, CLK, S | Q | Q ← D on posedge CLK; Q ← 1 when S=1 |
| `DFF_RS` | D, CLK, R, S | Q | Combined async reset + set |
| `DLATCH` | D, EN | Q | Q = D when EN=1, holds when EN=0 |
| `TBUF` | A, OE | Y | Y = OE?A:'Z' (tristate buffer) |
| `CONST_0`, `CONST_1` | — | Y | Y = 0 or Y = 1 (literal constants) |

Why these specifically? They are the closure of:
1. The cells the existing `logic-gates` package implements (so HNL is testable against the existing simulator);
2. The minimal set required to express any combinational function (NAND2 alone suffices, but performance and readability demand more);
3. The cells synthesis naturally infers from RTL (the `MUX2` cell is the natural target for `?:` and `if/else`; DFFs target sequential always blocks).

Wider variants (`AND4`, `OR4`) are conveniences. Synthesis produces them when natural; tech mapping decomposes them as needed for the target library.

### Hierarchy

A module can instantiate other modules. There is no requirement that a netlist be flat. Tools that need a flat view (typically place & route) call a `flatten()` pass that walks the hierarchy and inlines.

```
top
├── u_fa0 : full_adder
│   ├── u_x1 : XOR2
│   ├── u_x2 : XOR2
│   ├── u_a1 : AND2
│   ├── u_a2 : AND2
│   └── u_o1 : OR2
├── u_fa1 : full_adder
├── u_fa2 : full_adder
└── u_fa3 : full_adder
```

### Width-typed nets

Every net has a width N ≥ 1. Connections check that:

- A pin of width W connects only to a slice of the net of width W (a single-bit pin connects to `net[i]`; a 4-bit pin connects to `net[3:0]` or `net[7:4]` etc.).
- For a `connect`, the LHS port-or-net width equals the RHS port-or-net width.

Width inference is the responsibility of `synthesis.md` — by the time HNL exists, all widths are explicit.

## HNL JSON Schema

```json
{
  "format": "HNL",
  "version": "0.1.0",
  "level": "generic",
  "top": "adder4",
  "modules": {
    "adder4": {
      "ports": [
        { "name": "a",   "dir": "input",  "width": 4 },
        { "name": "b",   "dir": "input",  "width": 4 },
        { "name": "cin", "dir": "input",  "width": 1 },
        { "name": "sum", "dir": "output", "width": 4 },
        { "name": "cout","dir": "output", "width": 1 }
      ],
      "nets": [
        { "name": "c0", "width": 1 },
        { "name": "c1", "width": 1 },
        { "name": "c2", "width": 1 }
      ],
      "instances": [
        {
          "name": "u_fa0",
          "type": "full_adder",
          "connections": {
            "a":    { "net": "a",   "bits": [0] },
            "b":    { "net": "b",   "bits": [0] },
            "cin":  { "net": "cin", "bits": [0] },
            "sum":  { "net": "sum", "bits": [0] },
            "cout": { "net": "c0",  "bits": [0] }
          }
        },
        {
          "name": "u_fa1",
          "type": "full_adder",
          "connections": {
            "a":    { "net": "a",   "bits": [1] },
            "b":    { "net": "b",   "bits": [1] },
            "cin":  { "net": "c0",  "bits": [0] },
            "sum":  { "net": "sum", "bits": [1] },
            "cout": { "net": "c1",  "bits": [0] }
          }
        }
      ]
    },

    "full_adder": {
      "ports": [
        { "name": "a",    "dir": "input",  "width": 1 },
        { "name": "b",    "dir": "input",  "width": 1 },
        { "name": "cin",  "dir": "input",  "width": 1 },
        { "name": "sum",  "dir": "output", "width": 1 },
        { "name": "cout", "dir": "output", "width": 1 }
      ],
      "nets": [
        { "name": "axb",  "width": 1 },
        { "name": "ab",   "width": 1 },
        { "name": "axbc", "width": 1 }
      ],
      "instances": [
        { "name": "u_x1", "type": "XOR2",
          "connections": { "A": {"net":"a","bits":[0]}, "B": {"net":"b","bits":[0]}, "Y": {"net":"axb","bits":[0]} } },
        { "name": "u_x2", "type": "XOR2",
          "connections": { "A": {"net":"axb","bits":[0]}, "B": {"net":"cin","bits":[0]}, "Y": {"net":"sum","bits":[0]} } },
        { "name": "u_a1", "type": "AND2",
          "connections": { "A": {"net":"a","bits":[0]}, "B": {"net":"b","bits":[0]}, "Y": {"net":"ab","bits":[0]} } },
        { "name": "u_a2", "type": "AND2",
          "connections": { "A": {"net":"axb","bits":[0]}, "B": {"net":"cin","bits":[0]}, "Y": {"net":"axbc","bits":[0]} } },
        { "name": "u_o1", "type": "OR2",
          "connections": { "A": {"net":"ab","bits":[0]}, "B": {"net":"axbc","bits":[0]}, "Y": {"net":"cout","bits":[0]} } }
      ]
    }
  }
}
```

Schema notes:

- Top-level `level` is one of `"generic"` (built-in primitives only), `"stdcell"` (after tech mapping), or `"mixed"` (transitional / debug).
- `top` names the design's top module — every other module is reachable from it via instance graph traversal.
- `connections` maps **pin name on the cell type** → **net slice**. The `bits` array is little-endian, and its length must match the pin width.
- Bit-blasting style: a 4-bit `a` net is one entry in `nets`, but is referenced bit-by-bit in single-bit pin connections.
- Names follow Verilog identifier rules: `[A-Za-z_][A-Za-z0-9_$]*`. Special characters require quoting in EDIF/Verilog export — handled by writers, not by HNL itself.

## Python API

```python
from dataclasses import dataclass, field
from enum import Enum
from typing import Literal


class Direction(Enum):
    INPUT = "input"
    OUTPUT = "output"
    INOUT = "inout"


class Level(Enum):
    GENERIC = "generic"
    STDCELL = "stdcell"
    MIXED = "mixed"


@dataclass(frozen=True)
class Port:
    """An external interface point of a module."""
    name: str
    direction: Direction
    width: int  # >= 1

    def __post_init__(self) -> None:
        if self.width < 1:
            raise ValueError(f"port {self.name!r}: width must be >= 1, got {self.width}")


@dataclass(frozen=True)
class Net:
    """An internal wire of a module."""
    name: str
    width: int  # >= 1


@dataclass(frozen=True)
class NetSlice:
    """A reference to specific bits of a named net."""
    net: str
    bits: tuple[int, ...]  # little-endian; length == referencing pin width

    def width(self) -> int:
        return len(self.bits)


@dataclass(frozen=True)
class Instance:
    """An instantiation of a cell type inside a module."""
    name: str
    cell_type: str  # either a built-in primitive or another module name
    connections: dict[str, NetSlice]  # pin name → net slice
    parameters: dict[str, int | str] = field(default_factory=dict)


@dataclass
class Module:
    """A circuit definition."""
    name: str
    ports: list[Port] = field(default_factory=list)
    nets: list[Net] = field(default_factory=list)
    instances: list[Instance] = field(default_factory=list)

    def port(self, name: str) -> Port:
        for p in self.ports:
            if p.name == name:
                return p
        raise KeyError(f"module {self.name!r}: no port {name!r}")

    def net(self, name: str) -> Net:
        for n in self.nets:
            if n.name == name:
                return n
        raise KeyError(f"module {self.name!r}: no net {name!r}")


@dataclass
class Netlist:
    """A complete HNL document. The root data structure."""
    top: str
    modules: dict[str, Module] = field(default_factory=dict)
    level: Level = Level.GENERIC
    version: str = "0.1.0"

    @classmethod
    def from_json(cls, path: str) -> "Netlist":
        """Parse an HNL JSON file."""
        ...

    def to_json(self, path: str) -> None:
        """Serialize to canonical HNL JSON."""
        ...

    def validate(self) -> "ValidationReport":
        """Run all invariant checks."""
        ...

    def flatten(self) -> "Netlist":
        """Return a copy with all hierarchy inlined into the top module."""
        ...

    def stats(self) -> "NetlistStats":
        """Cell counts, net counts, max depth."""
        ...


@dataclass
class ValidationReport:
    """Result of validating a netlist."""
    errors: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.errors


@dataclass
class NetlistStats:
    cell_counts: dict[str, int]   # cell_type -> count (post-flatten)
    total_cells: int
    total_nets: int
    max_hierarchy_depth: int
```

## Built-in Cell Type Registry

Built-in cell types have a fixed pin signature stored in a registry that the validator consults.

```python
@dataclass(frozen=True)
class CellTypeSig:
    name: str
    inputs: list[str]   # input pin names
    outputs: list[str]  # output pin names
    pin_widths: dict[str, int]  # all pins are 1-bit unless overridden


BUILTIN_CELL_TYPES: dict[str, CellTypeSig] = {
    "BUF":   CellTypeSig("BUF",   ["A"],         ["Y"], {}),
    "NOT":   CellTypeSig("NOT",   ["A"],         ["Y"], {}),
    "AND2":  CellTypeSig("AND2",  ["A", "B"],    ["Y"], {}),
    "AND3":  CellTypeSig("AND3",  ["A","B","C"], ["Y"], {}),
    # ... and so on for AND4, OR2-4, NAND2-4, NOR2-4, XOR2-3, XNOR2-3
    "MUX2":  CellTypeSig("MUX2",  ["A","B","S"], ["Y"], {}),
    "DFF":   CellTypeSig("DFF",   ["D","CLK"],   ["Q"], {}),
    "DFF_R": CellTypeSig("DFF_R", ["D","CLK","R"],["Q"], {}),
    "DFF_S": CellTypeSig("DFF_S", ["D","CLK","S"],["Q"], {}),
    "DFF_RS":CellTypeSig("DFF_RS",["D","CLK","R","S"],["Q"], {}),
    "DLATCH":CellTypeSig("DLATCH",["D","EN"],    ["Q"], {}),
    "TBUF":  CellTypeSig("TBUF",  ["A","OE"],    ["Y"], {}),
    "CONST_0": CellTypeSig("CONST_0", [], ["Y"], {}),
    "CONST_1": CellTypeSig("CONST_1", [], ["Y"], {}),
}
```

For user-defined modules, the signature is derived from the module's port list at validation time.

## Validation Rules

The validator runs after every transformation. A netlist is valid iff:

| Rule | Severity | Description |
|---|---|---|
| **R1** Top module exists | Error | `netlist.top` names an entry in `netlist.modules`. |
| **R2** Cell types resolve | Error | Every instance's `cell_type` is either a built-in or a user module in `netlist.modules`. |
| **R3** Pin coverage | Error | Every input pin of every cell type has a connection. (Output pins may be unconnected — synthesis dead-code-eliminates these.) |
| **R4** Pin name matches signature | Error | Every key in `connections` is an actual pin name on the cell type. |
| **R5** Width match | Error | `connections[pin].width() == cell_type.pin_widths[pin]`. |
| **R6** Net referenced exists | Error | Every `NetSlice.net` names a real net in the module (or a port). |
| **R7** Bit indices in range | Error | Every `bits[i]` is `0 <= i < net.width`. |
| **R8** Single driver per net bit | Error | For each (net, bit), at most one output pin or one `input` port drives it (unless all drivers are `TBUF` — wired-OR exception). |
| **R9** Every used input has a driver | Error | For each (net, bit) referenced by an input pin, exactly one driver exists somewhere. |
| **R10** No combinational cycles | Error | Cycles in the cell graph are only allowed if at least one edge crosses a sequential cell (DFF/DLATCH). Detected by a graph walk that "cuts" sequential cells. |
| **R11** Module not self-instantiating | Error | A module cannot directly or transitively instantiate itself (no recursion in hardware). |
| **R12** Naming convention | Warning | Names match `[A-Za-z_][A-Za-z0-9_$]*`. |
| **R13** Output port driven | Warning | Every output port has at least one driver inside the module. |

The cycle check (R10) is the most expensive — it requires building the cell graph (V = cells + ports, E = pin connections via nets) and running cycle detection while marking edges from a sequential cell's input as "broken." The existing `directed-graph` package's cycle detection is the right tool.

## Worked Example 1 — 4-bit Adder (the smoke test)

The 4-bit ripple-carry adder. Top module `adder4` instantiates four `full_adder` modules; each `full_adder` is built from primitives. JSON shown above in §"HNL JSON Schema."

After flattening:

```
adder4 (flat)
├── u_fa0_u_x1 : XOR2    a[0], b[0]    → axb_0
├── u_fa0_u_x2 : XOR2    axb_0, cin    → sum[0]
├── u_fa0_u_a1 : AND2    a[0], b[0]    → ab_0
├── u_fa0_u_a2 : AND2    axb_0, cin    → axbc_0
├── u_fa0_u_o1 : OR2     ab_0, axbc_0  → c0
├── u_fa1_u_x1 : XOR2    a[1], b[1]    → axb_1
├── u_fa1_u_x2 : XOR2    axb_1, c0     → sum[1]
├── u_fa1_u_a1 : AND2    a[1], b[1]    → ab_1
├── u_fa1_u_a2 : AND2    axb_1, c0     → axbc_1
├── u_fa1_u_o1 : OR2     ab_1, axbc_1  → c1
├── u_fa2_*    : ... (5 more cells, mirroring u_fa1)
├── u_fa3_*    : ... (5 more cells)
```

**Stats:**
- 20 primitive cells (4 full-adders × 5 cells each)
- 5 internal nets (`axb_0..3`, `c0..2`, `sum[0..3]` are output port bits, `ab_0..3`, `axbc_0..3`)
- Max hierarchy depth: 1 (top → full_adder → leaf)

## Worked Example 2 — 32-bit ALU (mid-scale)

A 32-bit ALU computing `add`, `sub`, `and`, `or`, `xor`, `slt`, plus `eq` flag. Built as:

- Two 32-bit input nets `a`, `b`; 3-bit `op`; 32-bit `result`; 1-bit `zero`.
- One `adder32` instance (which contains 32 `full_adder` instances internally).
- 32 × `XOR2` for the bitwise XOR operation.
- 32 × `AND2`, 32 × `OR2`.
- A 32-bit, 8-input MUX tree to select among results.
- A 32-input NOR tree → `zero` flag (built from `NOR4` + `AND` reductions).

**Stats:**
- ~600 primitive cells flat
- Hierarchy depth: 3 (alu → adder32 → full_adder → primitives)
- Demonstrates HNL scaling: same JSON shape, just more entries.

## Worked Example 3 — Sequential design (FSM)

A 4-state Moore FSM (Red → Green → Yellow → Red) — the same "traffic light" example from `F01-fpga.md`. Demonstrates DFF usage and combinational-loop detection (the loop `state → next_state_logic → state` is broken by the DFF, so R10 passes).

```
top
├── u_state_0 : DFF       D=ns[0], CLK=clk, Q=state[0]
├── u_state_1 : DFF       D=ns[1], CLK=clk, Q=state[1]
├── u_ns0     : NOT       A=state[0], Y=ns[0]    -- next-state bit 0
├── u_ns1_a1  : AND2      A=state[0], B=¬state[1], Y=t1
├── u_ns1_a2  : AND2      A=¬state[0], B=state[1], Y=t2     (or computed via more cells)
└── u_ns1     : OR2       A=t1, B=t2, Y=ns[1]
```

(Schematic abbreviated; the full FSM is ~10 cells.)

## EDIF Importer / Exporter

EDIF (Electronic Design Interchange Format), version 2 0 0, is the historical FPGA interchange format. `nextpnr` accepts it. The format is LISP-like:

```
(edif design_name
  (edifVersion 2 0 0)
  (edifLevel 0)
  (keywordMap (keywordLevel 0))
  (library work
    (cell adder4
      (cellType GENERIC)
      (view netlist
        (viewType NETLIST)
        (interface
          (port a (direction INPUT) (array 4))
          (port b (direction INPUT) (array 4))
          (port sum (direction OUTPUT) (array 5)))
        (contents
          (instance u_fa0 (viewRef netlist (cellRef full_adder)))
          ...
          (net c0 (joined (portRef cout (instanceRef u_fa0))
                         (portRef cin  (instanceRef u_fa1)))))))))
```

### Mapping HNL → EDIF
| HNL | EDIF |
|---|---|
| `Module` | `cell ... (cellType GENERIC) (view netlist (viewType NETLIST) ...)` |
| `Port` | `(port name (direction INPUT/OUTPUT/INOUT) (array N))` for N>1 |
| `Net` | `(net name (joined ...))` listing all connected `portRef` |
| `Instance` | `(instance name (viewRef netlist (cellRef cell_type)))` |

### Mapping EDIF → HNL
- Treat each EDIF `cell` with a `view netlist` as a `Module`.
- Externally referenced cells (libraries) become unresolved `cell_type` references — synthesis or tech mapping must provide them.
- EDIF arrays map to `width > 1` nets; bit selection becomes `bits=[i]`.

### Edge cases
| Scenario | Handling |
|---|---|
| EDIF identifiers contain unsupported characters | EDIF rename construct (`(rename old "new"))`) — preserved as metadata. |
| Multi-view cells | We import only the `netlist` view; warn on others. |
| External library references | Imported as `cell_type` strings; require post-import resolution against an external library list. |

## BLIF Importer / Exporter

BLIF (Berkeley Logic Interchange Format) is what yosys and ABC speak natively. It is *flat* (loses hierarchy) and uses sum-of-products truth tables instead of named cell types.

```
.model adder4
.inputs a[0] a[1] a[2] a[3] b[0] b[1] b[2] b[3] cin
.outputs sum[0] sum[1] sum[2] sum[3] cout

.names a[0] b[0] cin sum[0]
100 1
010 1
001 1
111 1

.names a[0] b[0] cin c0
011 1
101 1
110 1
111 1

# ... more .names blocks for sum[1]..sum[3] and c1..c2 ...
.end
```

### HNL → BLIF
1. Flatten the netlist (BLIF is inherently flat).
2. For each cell, emit a `.names` block whose truth table is the cell's truth table.
3. Sequential cells (`DFF`, `DLATCH`) emit `.latch` records:
   ```
   .latch D Q re CLK 0
   ```
   (`re` = rising-edge; initial value 0.)
4. Top-level inputs and outputs become `.inputs` / `.outputs`.

### BLIF → HNL
1. Each `.names` block becomes one or more primitive cells. Common cases recognized: `00 0 / 11 1` → `XOR2`, etc. Unrecognized truth tables become "cube cells" — a future extension; for now, warn.
2. Each `.latch` becomes a `DFF` (or `DFF_R`/`DFF_S` if reset/set are present in the BLIF extension form).
3. The result is a single flat module named after the `.model` line.

### Edge cases
| Scenario | Handling |
|---|---|
| `.names` with an empty truth table | Output is constant 0 (BLIF convention). Emit `CONST_0`. |
| `.names` with all 1's truth table for a cube | Output is constant 1. Emit `CONST_1`. |
| Don't-care bits in the cube table (`-`) | Expand: the resulting cell's truth table covers all matching minterms. |
| Multi-output `.names` (BLIF extension) | Reject with error — not in BLIF base spec. |

## Edge Cases

| Scenario | Handling |
|---|---|
| Net width = 1 with a single-bit pin connection | `bits: [0]`. Always little-endian. |
| Pin connects to a slice of a wider net | `bits: [3,2,1,0]` — order matters; preserves bit order. |
| Output pin unused (no readers) | Not an error. Synthesis or downstream optimization will dead-code-eliminate. R3 only requires *input* pins to be connected. |
| Tristate output (multi-driver bus) | All drivers must be `TBUF` cells, and only one's `OE` is high at a time. R8 has a special-case carve-out for `TBUF`. |
| Self-loop combinational | `XOR2 with Y connected back to A` — caught by R10. |
| Sequential loop (state machine feedback) | `DFF.Q → ... → DFF.D` — allowed; R10 cuts the DFF edge. |
| Constant tied to input | Use `CONST_0` or `CONST_1` cells. (Some tools accept literal `1'b0` in connections; HNL requires explicit cells for visual uniformity.) |
| Generic cell in a stdcell-level netlist | Validator warns: level mismatch. |
| Two modules with the same name | Last write wins on `Netlist.modules` dict. Importers should detect and error on duplicate model definitions in BLIF/EDIF. |
| Pin width 0 (degenerate) | Forbidden. Width must be ≥ 1. |
| User module with no instances and no contents (a "blackbox") | Allowed if marked with `attributes.blackbox=true`. Validator skips internal checks; downstream tools must provide an implementation. |

## Conformance Matrix

| Standard / Format | Coverage | Notes |
|---|---|---|
| **EDIF 2.0.0** | Subset (`netlist` view only) | Adequate for synthesis/PnR interchange. |
| **EDIF 4.0.0** | Out of scope | Used for analog/multi-domain; not relevant to digital netlists. |
| **BLIF (Sentovich 1992)** | Full | All `.names`, `.latch`, `.model` records supported. |
| **BLIF-MV** | Out of scope | Multi-valued logic; not in our target. |
| **eblif** (yosys extended BLIF) | Partial | Wider DFF variants (`DFF_R`/`DFF_S`) supported; cell parameter records as `attributes`. |
| **Verilog structural** | See `real-fpga-export.md` | Verilog is a *language*; HNL → Verilog is a separate spec because it requires identifier escaping, generate-block flattening, etc. |

## Test Strategy

### Unit (target: 95%+ for the library)
- Round-trip: `Netlist → JSON → Netlist` is identity (modulo dict ordering).
- Validation rule R1–R13: one positive + one negative test each.
- Cycle detection: 20-cell ring oscillator (combinational cycle) → R10 fails. Same circuit with a DFF inserted → R10 passes.
- Width mismatch: 4-bit net to 1-bit pin → R5 fails with helpful diagnostic.
- Unknown cell type: instance `u_x : MYSTERY` with no module definition → R2 fails.
- Self-instantiation: module `m` instantiates `m` → R11 fails.

### Integration
- Synthesis output is HNL-valid for `arm1-gatelevel`, `intel4004-gatelevel` (existing reference designs).
- Round-trip HNL → BLIF → HNL: structural identity (modulo cube canonicalization).
- Round-trip HNL → EDIF → HNL: structural identity.
- Hand-written 4-bit adder JSON loads, validates, simulates correctly under `hardware-vm`.
- 32-bit ALU: validates in <100 ms; flatten produces ~600 cells; stats match expected.

### Property tests
- Random valid netlists (generated): `validate()` returns ok.
- Random valid netlists with single bit-flip mutation: at least 80% become invalid.
- Flattening preserves semantics: simulating a hierarchical netlist and the same netlist after flatten produces identical traces.

## Performance Notes

For mid-scale designs (≤ 100K cells), in-memory representation is fine. For larger designs, two extensions are anticipated:

1. **Streaming reader**: `Netlist.from_json_stream(file)` that yields modules one at a time (using `ijson`).
2. **Indexed module store**: backed by SQLite for designs that don't fit in RAM.

These are deferred to a future spec (`hnl-streaming.md`); v1 is in-memory only and assumes the user is on a machine where the netlist fits.

## Open Questions

1. **Should constants be cells or literal connection targets?**
   - *Option A* (current): `CONST_0`/`CONST_1` cells. Visually uniform, slightly verbose.
   - *Option B*: Literal `{"value": 0}` in connections. Compact, but breaks the "pin connects to net" mental model.
   - **Recommendation**: stick with cells; consider sugar in a future writer pass.

2. **Should HNL preserve attribute annotations from source HDL?**
   - Verilog/VHDL allow `(* attribute = "value" *)` annotations (timing, location hints). Some matter to downstream (e.g., `(* keep *)` prevents optimization).
   - **Recommendation**: yes, optional `attributes: dict[str, str]` field on Module / Instance / Net.

3. **Versioning** — when HNL evolves (it will), how do we handle older files?
   - Header has `version: "0.1.0"`. Add a migration table in a future revision; for v1, refuse to load mismatched majors.

4. **Hierarchical net naming for cross-module debugging** — when a flatten happens, internal nets get prefixed (e.g., `u_fa0.axb` becomes `u_fa0_axb`). This is unambiguous but ugly for debug. Consider a "preserve_hierarchical_names" flag.

5. **Should we have an "AIG" (And-Inverter Graph) level alongside generic and stdcell?**
   - AIG is what ABC operates on internally and enables huge optimization wins.
   - **Recommendation**: deferred. Start without AIG; add `level: "aig"` in a later spec if synthesis quality demands it.

## Future Work

- **Streaming readers/writers** for designs > 100K cells.
- **AIG level** for ABC-style optimization passes.
- **HNL-MV** (multi-valued logic) for representing 4-state simulation results in netlists.
- **Diff / merge tools** — given two HNL files, produce a structural diff.
- **Visualization** — render an HNL netlist as a graph (reuse the existing visualization tooling).
- **Provenance tracking** — annotate each cell with the HIR construct that produced it, for debugging synthesis output.
