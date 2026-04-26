# FPGA Place-and-Route Bridge

## Overview

A bridge between HNL netlists and the existing `fpga` package's JSON config format (defined in `F01-fpga.md`). Takes a generic HNL netlist (or a tech-mapped HNL where cells are LUTs and FFs), packs the logic into LUTs, places packed CLBs onto the fabric grid, routes signals through the switch matrix, and emits a JSON config that the `fpga` simulator can run directly.

Three sub-problems:
1. **LUT packing** — convert combinational logic into LUTs (≤ K-input truth tables); pack pairs into CLB slices.
2. **Placement** — assign each CLB to a physical (row, col) coordinate.
3. **Routing** — find paths through switch matrices connecting LUT outputs to LUT inputs.

For v1, simulated annealing for placement and PathFinder-style maze routing. Both are well-known algorithms with simple implementations and good educational value. More sophisticated alternatives (analytical placement, negotiation-based routing) are future work.

The output is `fpga` package's existing JSON schema, so existing visualizers and simulators run on the result without changes.

## Layer Position

```
HNL (generic or stdcell, but typically generic is more LUT-friendly)
       │
       ▼
fpga-place-route-bridge.md  ◀── THIS SPEC
       │
       ├──► LUT packing
       ├──► Placement (simulated annealing)
       ├──► Routing (PathFinder)
       │
       ▼
fpga JSON config (per F01-fpga.md schema)
       │
       ▼
fpga package simulator → simulation results
       │
       ▼
fpga-bitstream.md (real iCE40 bitstream emission)
```

## Concepts

### LUT packing

A K-input LUT can implement any boolean function of K variables (truth table of 2^K entries). The packing problem: cover the combinational HNL with LUTs minimizing total LUT count.

For our `fpga` package (4-input LUTs, 2 LUTs per slice, 2 slices per CLB):

1. **Cone packing**: starting from each combinational output, walk back through gates collecting predecessors until either:
   - The cone has > 4 distinct inputs → split.
   - The cone hits a sequential boundary (DFF) → seal.
2. **Truth-table generation**: for each 4-input cone, evaluate every input combination to produce a 16-entry truth table.
3. **Slice packing**: pair LUTs that share input signals or are connected sequentially into the same slice.

Example: a 1-bit full-adder (sum + cout) packs into 2 LUTs (sum is one 3-input function of a/b/cin; cout is another). They go into one slice.

### Placement: simulated annealing

Cost function: total wirelength estimated by half-perimeter of bounding box (HPWL) of each net. For each net:
```
HPWL(net) = (max_x - min_x) + (max_y - min_y)
```

Algorithm:
```
T = T_start
for iteration in range(N):
    pick two random CLBs; tentatively swap them
    delta_cost = compute_cost_change()
    if delta_cost < 0:
        accept swap
    elif random() < exp(-delta_cost / T):
        accept swap
    else:
        revert swap
    T *= cooling_factor
```

Typical `T_start = average net cost / 5`; `cooling_factor = 0.95`; iterations until `T < 0.001 × T_start`.

For a 4-bit adder with ~6 CLBs, annealing converges in milliseconds. A full RISC-V scalar core (~5K LUTs ≈ 1.2K CLBs) takes ~1 minute.

### Routing: PathFinder

PathFinder iteratively routes nets, allowing congestion, then negotiates: each routing resource has a usage cost that grows when overused. Steps:

1. Initialize: every routing resource has cost 1.
2. For each net (sorted by criticality):
   - Use Dijkstra/A* to find a shortest path from driver to each sink.
   - Update resource usage.
3. After all nets routed: for each over-used resource, increase its cost.
4. If all nets routed without overuse, stop. Otherwise repeat from step 2.

Convergence: typically 2-5 iterations for the small-scale designs we target. PathFinder is the algorithm behind nextpnr; ours is a simpler instructional version.

## Public API

```python
from dataclasses import dataclass


@dataclass
class FpgaConfig:
    """Internal in-memory representation; serialized to F01-fpga.md JSON."""
    rows: int
    cols: int
    clbs: dict[tuple[int, int], "ClbConfig"]
    routes: list["RouteSegment"]
    io_pins: dict[str, "IoPin"]


@dataclass
class ClbConfig:
    slice0: "SliceConfig"
    slice1: "SliceConfig"


@dataclass
class SliceConfig:
    lut_a_truth_table: list[int]   # 16 entries for 4-input LUT
    lut_b_truth_table: list[int]
    ff_a_enabled: bool
    ff_b_enabled: bool
    ff_a_init: int = 0
    ff_b_init: int = 0
    output_mux_a: str = "combinational"   # or "registered"
    output_mux_b: str = "combinational"


@dataclass
class FpgaBridge:
    fabric_rows: int = 4
    fabric_cols: int = 4
    
    def map(self, hnl: "Netlist") -> FpgaConfig:
        """Full pipeline: pack LUTs → place → route → emit config."""
        ...
    
    def pack(self, hnl: "Netlist") -> "PackedDesign": ...
    def place(self, packed: "PackedDesign") -> "PlacedDesign": ...
    def route(self, placed: "PlacedDesign") -> FpgaConfig: ...
    
    def emit_json(self, config: FpgaConfig, path: str) -> None: ...
```

## Worked Example — 4-bit Adder

Input HNL: 20 generic gates (per `synthesis.md` output).

Pack:
- Each full-adder produces 2 LUTs: one 3-input for `sum = a^b^cin`, one 3-input for `cout = (a&b) | ((a^b)&cin)`.
- 4 full-adders × 2 LUTs = 8 LUTs total.
- Pack pairs into slices: 4 slices = 4 CLBs (one slice per CLB; second slice empty for clarity).

Place: 4 CLBs on a 2×2 sub-grid. Initial: random. After ~100 SA iterations: linear arrangement (each FA next to the next, minimizing carry-net wirelength).

Route:
- 4 input-carry nets (one per FA).
- 12 input bits (4 a, 4 b, 1 cin, 4 sum, 1 cout = 14, but a/b are inputs and sum/cout are outputs of the adder; total nets ≈ 20).
- Each net routed in <10 ms.

Output: a JSON config matching F01-fpga.md schema, ~6 KB. Loads in the existing FPGA simulator; running stimulus produces correct sums.

## Worked Example — 32-bit ALU

~600 generic gates.

Pack: most ALU operations fit naturally into 4-LUTs. Comparator and mux trees are 4-LUT-friendly. Total ~150 LUTs = ~75 slices = ~38 CLBs.

Place: a 6×7 grid of CLBs. SA takes ~1 sec.

Route: ~200 nets. PathFinder converges in 2-3 iterations.

Output: ~25 KB JSON config.

## Edge Cases

| Scenario | Handling |
|---|---|
| Combinational logic with > 4 inputs in a single cone | Split into multiple LUTs (one per output); use intermediate wire. |
| LUTs with > 16 truth-table entries (i.e., > 4 inputs) | Cannot fit; split. |
| Placement with N CLBs > rows × cols | Reject; need bigger fabric. |
| Routing congestion that doesn't resolve | After 10 PathFinder iterations, give up; report unrouted nets. |
| Memory inference (BRAM) | Defer to BRAM mapping; out of v1 scope (most of our examples don't use BRAM). |
| Carry chains | Use FPGA carry-chain hardware (configured in the slice's carry chain field). |
| Tristate outputs | Generally not supported by inner-fabric LUTs; reject for FPGA path. |
| Hierarchical HNL | Flatten before packing. |

## Test Strategy

### Unit (95%+)
- LUT truth-table generation: AND2, OR2, XOR2 → correct 16-entry tables.
- Cone packing: respects 4-input constraint.
- HPWL calculation: matches hand-computed for small examples.
- SA: makes monotonic progress on a simple example.
- PathFinder: routes a 2-node net through a small fabric.

### Integration
- 4-bit adder maps to F01 fpga JSON; F01 sim produces correct output.
- 32-bit ALU maps cleanly; F01 sim produces correct output for a stimulus suite.
- Cross-check vs `nextpnr-ice40` on a small benchmark: our routing congestion within 2× of nextpnr's.

### Property
- Determinism: same HNL + same SA seed → same JSON config.
- Equivalence: F01 sim of mapped FPGA == hardware-vm sim of source HNL.

## Conformance

| Reference | Coverage |
|---|---|
| `F01-fpga.md` JSON schema | Full output |
| `F02-graph-foundations.md` directed-graph | Used for routing graph |
| nextpnr (oracle for cross-check) | Compared on small benchmarks |
| iCE40 / ECP5 (real FPGAs) | Out of scope for this spec; see `fpga-bitstream.md` |

## Open Questions

1. **6-LUT support** — modern FPGAs use 6-input LUTs. Our `fpga` package is 4-LUT. Stay with 4-LUT for v1.
2. **Analytical placement** — quadratic placement with B2B nets. Future.
3. **Negotiation-based routing** (the actual nextpnr algorithm) — our PathFinder is simpler; close enough for teaching.

## Future Work

- 6-LUT support (parameterize fabric).
- Analytical placement.
- Real FPGA targets (iCE40, ECP5, Xilinx 7-series).
- Incremental P&R for ECO flows.
- BRAM/DSP block packing.
- Clock-domain-aware placement.
