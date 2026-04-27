# Synthesis (HIR → HNL)

## Overview

Synthesis turns *behavioral* HIR (processes, conditionals, loops, arithmetic operators) into *structural* HNL (a netlist of primitive gates and flip-flops). It is where intent becomes hardware.

The transform is opinionated about what *can* be synthesized. Constructs without physical interpretation — `wait for 10 ns`, `$display`, `force`, file I/O — are rejected with clear errors. Constructs that have a physical interpretation but require recognizing patterns — "this `always @(posedge clk)` is a flip-flop"; "this `if/else` chain is a multiplexer" — are inferred via well-known idioms documented in IEEE 1076.6 and Verilog synthesis lore.

This spec defines:
1. The synthesizable subset of HIR (what is and isn't accepted).
2. RTL inference rules: when does a Process become a flip-flop vs combinational logic vs a latch?
3. Operator → gate-network mapping (adders, multiplexers, comparators).
4. FSM extraction.
5. Memory inference (arrays → register files or BRAMs).
6. Optimization passes (constant folding, dead-code elimination, common subexpression).
7. The HIR → HNL projection.

The output is `gate-netlist-format.md` HNL with `level=generic` (built-in primitive cells: AND, OR, NOT, XOR, DFF, MUX2, etc.). `tech-mapping.md` then specializes to standard cells.

### Generality

The 4-bit adder synthesizes to ~20 gates. A 32-bit ALU to ~600. The ARM1 reference (existing in repo as `arm1-gatelevel`) is roughly 20K gates. The largest design we expect to handle in v1 is ~100K gates; beyond that, hierarchy preservation and optimization scaling become bottlenecks.

## Layer Position

```
HIR (behavioral)
    │
    ▼
synthesis.md  ◀── THIS SPEC
    │ (RTL inference + optimization + projection)
    ▼
HNL (level=generic)   (gate-netlist-format.md)
    │
    ▼
tech-mapping.md   →   HNL (level=stdcell)
    │
    ▼
ASIC backend / FPGA bridge
```

## Concepts

### The synthesizable subset

| HIR construct | Synthesizable? | Notes |
|---|---|---|
| Module / Port / Net / Instance | Yes | Always. |
| Continuous assignment | Yes | Becomes combinational logic. |
| Process with sensitivity = `[posedge clk]` | Yes | Sequential (FF inference). |
| Process with sensitivity = `[posedge clk, posedge reset]` | Yes | Async-reset FF. |
| Process with sensitivity = `[*]` (combinational) | Yes | Combinational logic. |
| Process with sensitivity = mixed clock + level | Maybe | Latch — flagged with warning. |
| Process with `wait for N ns` | No | Reject. |
| Process with `wait until` | Maybe | Only `wait until rising_edge(clk)` recognized. |
| `assert`, `report` | Synth-skip | Compiled out (preserved in HIR for sim). |
| `$display`, `$monitor`, `$finish` | Synth-skip | Compiled out. |
| `initial` | Synth-skip | Used for sim only; rejected for synth (with warning). |
| `force`, `release` | No | Reject. |
| `function` (pure) | Yes | Inlined. |
| `task` | Yes (limited) | Inlined; no automatic; no recursion. |
| `for/while` loops with constant bounds | Yes | Unrolled. |
| `for/while` with non-constant bounds | No | Reject. |
| `case/casex/casez` | Yes | Becomes mux trees. |
| `if/else` | Yes | Becomes mux. |
| Integer arithmetic | Yes | Mapped to standard arithmetic blocks. |
| Real / float | No | Reject. |
| `time` type | No | Reject (sim-only). |
| Records / arrays | Yes | Bit-blasted. |
| File I/O | No | Reject. |
| User-defined types | Yes (after bit-blast) | |

The synthesizer walks the HIR, encounters each construct, and either projects it to HNL or emits a clear rejection with a source-location-aware diagnostic.

### Process classification

Every Process is classified into one of:

- **Sequential (clocked)**: sensitivity is `[posedge clk]` or similar. Body assigns to certain signals; those signals become flip-flop outputs. The flip-flop's D input is computed by a combinational network derived from the rest of the body.
- **Combinational**: sensitivity is `[*]` or covers all read signals. The body computes outputs as a pure function of inputs. No flip-flop; output is a wire.
- **Latch**: sensitivity covers some-but-not-all read signals; or `if/else` doesn't cover all paths. Synthesis warns (latches are usually unintended) and infers a level-sensitive D-latch.
- **Reset-asynchronous**: sensitivity is `[posedge clk, posedge reset]` or similar. Body has an `if (reset) ... else ...` shape; the `reset` branch sets initial value asynchronously.

### Combinational synthesis: from statements to mux trees

A combinational process body becomes a pure function from inputs to outputs. The synthesizer compiles the body into a **dataflow graph** of operations, then projects each operation to gates.

```verilog
always @(*) begin
  if (sel == 2'b00) y = a;
  else if (sel == 2'b01) y = b;
  else if (sel == 2'b10) y = c;
  else y = d;
end
```

Synthesis:
1. Walk the body; build a dataflow graph.
2. Each `if/else` becomes a 2:1 mux selecting between the then and else branches.
3. Chains of `if/else` collapse to a balanced mux tree.
4. Operators are projected: `==` to a comparator; `+` to an adder.
5. Mux tree → MUX2 cells.

Result: 3 × MUX2 cells (a balanced mux tree for 4 inputs).

### Sequential synthesis: FF inference

```verilog
always @(posedge clk) begin
  q <= d;
end
```

Synthesis:
1. Process is clock-sensitive (`posedge clk`).
2. `q` is assigned non-blocking.
3. The non-blocking assigned target becomes a DFF output.
4. The RHS (`d`) is the D input.

Result: `DFF(D=d, CLK=clk, Q=q)`.

With reset:
```verilog
always @(posedge clk or posedge reset) begin
  if (reset) q <= 0;
  else       q <= d;
end
```

Synthesis recognizes the async reset:
1. Sensitivity has both edges.
2. Body has `if (reset)`.
3. Reset value is a constant 0.

Result: `DFF_R(D=d, CLK=clk, R=reset, Q=q)`.

Synchronous reset:
```verilog
always @(posedge clk) begin
  if (reset) q <= 0;
  else       q <= d;
end
```

Sensitivity is just `posedge clk`. The reset is a normal mux:

Result: `DFF(D=mux(reset, 0, d), CLK=clk, Q=q)`.

### FSM extraction

A common idiom:
```verilog
reg [1:0] state, next_state;
parameter S_RED=0, S_GREEN=1, S_YELLOW=2;

always @(posedge clk) state <= next_state;

always @(*) begin
  case (state)
    S_RED:    next_state = S_GREEN;
    S_GREEN:  next_state = S_YELLOW;
    S_YELLOW: next_state = S_RED;
    default:  next_state = S_RED;
  endcase
end
```

Synthesis:
1. Detects: `state` is a register (sequential always block); next-state is combinational.
2. Recognizes the FSM pattern (state register + combinational next-state via case).
3. Records FSM metadata (state count, encoding) for downstream optimization.
4. Encoding choice (binary, one-hot, gray) affected by user attribute or default heuristic.

For binary encoding (default): 2-bit state register; case becomes a mux network.

For one-hot: 3-bit state register; case becomes a priority encoder + gating.

### Operator mapping

| Operator | Gate-level expansion |
|---|---|
| `~a` | NOT |
| `a & b` | AND |
| `a \| b` | OR |
| `a ^ b` | XOR |
| `a + b` (N-bit) | N full adders in a ripple-carry chain (default) or carry-lookahead (if width > threshold or `(* keep_hierarchy *)` requested) |
| `a - b` | adder with `~b + 1` (two's complement) |
| `a * b` | shift-and-add tree (Wallace, Booth — depends on configuration) |
| `a / b` | restoring division |
| `a == b` | XOR + reduction NOR |
| `a < b` | subtractor + sign bit |
| `a << k` (constant k) | wire shuffle (no logic) |
| `a << k` (variable k) | barrel shifter (mux tree) |
| `?:` ternary | MUX2 |
| `{a, b}` concatenation | wire concat (no logic) |
| `{N{a}}` replication | wire replication |
| Reduction `& a`, `\| a`, `^ a` | balanced AND/OR/XOR tree |

Each mapping is a parameterized generator. For example, the adder generator at width N produces:
```
N-bit adder = N × FullAdder cells in a chain
```

### Memory inference

```verilog
reg [7:0] mem [0:255];
always @(posedge clk) begin
  if (we) mem[wa] <= wd;
  rd <= mem[ra];
end
```

Synthesis:
1. `mem` is an array assigned in a clocked process — it's storage.
2. Detects: 1 write port + 1 read port → dual-port-RAM-friendly.
3. For ASIC: emits `MEM_DP(W=8, D=256)` instance; `tech-mapping.md` swaps for an SRAM macro or register file.
4. For FPGA: emits same; `fpga-place-route-bridge.md` maps to BRAM tile.

### Optimization passes

After RTL inference but before projection:
- **Constant folding**: `1'b0 & x` → `1'b0`; `0 + x` → `x`.
- **Dead-code elimination**: outputs nobody reads → remove.
- **Common subexpression elimination**: same `BinaryOp` computed twice → share.
- **Width reduction**: drop unused upper bits of an arithmetic result.
- **Trivial mux removal**: `mux(0, a, b)` → `b`.
- **De Morgan's**: `~(a & b)` → `~a | ~b` (when it reduces gate count).

These produce noticeable but not dramatic gate savings (~10-20%). For deeper optimization (AIG, ABC), see Future Work.

### Latch warnings

Synthesis warns when:
- A combinational `always @(*)` doesn't fully assign all outputs in all branches.
- A clocked process has an `if` with no `else` and the assigned signal appears only in one branch.

The user usually wants to fix the source; the synthesizer infers a D-latch for the unassigned path and proceeds.

## Worked Example 1 — 4-bit Adder

HIR has one ContAssign: `{cout, sum} = a + b + cin`.

Synthesis:
1. Combinational expression: `a + b + cin`.
2. Adder generator produces 4 full-adder modules (one per bit), chained via carry.
3. Each FullAdder: 2 XOR2 + 2 AND2 + 1 OR2 = 5 gates.
4. Total: 4 × 5 = 20 gates.

HNL has 4 instances of `full_adder` plus the carry chain wiring. Same structure as in `gate-netlist-format.md`.

## Worked Example 2 — Sequential counter

```verilog
module counter #(parameter N=8) (input clk, reset, output reg [N-1:0] count);
  always @(posedge clk or posedge reset)
    if (reset) count <= 0;
    else       count <= count + 1;
endmodule
```

Synthesis:
1. Sequential process with async reset.
2. `count` becomes N DFFs with async reset.
3. The `count + 1` is an N-bit adder where one operand is a constant (incrementer); generator simplifies to N half-adders.

For N=8: 8 DFF_R + 8 half-adders ≈ 8 × 1 + 8 × 2 = 24 cells.

## Worked Example 3 — Traffic light FSM

The FSM from §"FSM extraction" produces:
- 2 DFF (state register, with reset to S_RED).
- A mux network for next-state:
  - state==S_RED → S_GREEN
  - state==S_GREEN → S_YELLOW
  - state==S_YELLOW or default → S_RED

≈ 5-7 gates total. Plus output decoding (2-input case to 3-bit one-hot output for the lights).

## Worked Example 4 — 32-bit ALU (mid-scale)

```verilog
module alu(
  input  [31:0] a, b, input [3:0] op, output reg [31:0] y, output reg zero
);
  always @(*) begin
    case (op)
      4'h0: y = a + b;
      4'h1: y = a - b;
      4'h2: y = a & b;
      4'h3: y = a | b;
      4'h4: y = a ^ b;
      4'h5: y = (a < b) ? 1 : 0;
      4'h6: y = a << b[4:0];
      4'h7: y = a >> b[4:0];
      // ... more ops ...
      default: y = 32'h0;
    endcase
    zero = (y == 0);
  end
endmodule
```

Synthesis:
- Adder/subtractor: 32-bit ripple-carry → 192 cells.
- AND/OR/XOR: 32-bit bitwise → 32 cells each.
- Comparator: 32-bit subtractor + sign-bit detect → ~70 cells.
- Barrel shifter (5 stages × 32 muxes): ~160 cells.
- Result mux: 16-input × 32-bit → ~480 cells.
- Zero-detect: 32-input NOR tree → ~10 cells.

Total: ~600 gates. Matches the rough estimate from `gate-netlist-format.md`.

## Public API

```python
from dataclasses import dataclass
from enum import Enum
from typing import Protocol


class FsmEncoding(Enum):
    BINARY    = "binary"
    ONE_HOT   = "one_hot"
    GRAY      = "gray"
    AUTO      = "auto"


@dataclass(frozen=True)
class SynthOptions:
    flatten: bool = False
    optimize: bool = True
    fsm_encoding: FsmEncoding = FsmEncoding.AUTO
    add_mode: str = "ripple_carry"   # "ripple_carry" | "carry_lookahead" | "auto"
    mul_mode: str = "wallace"        # "wallace" | "booth" | "shift_add"
    mem_threshold: int = 64          # below: register file; above: BRAM/SRAM
    keep_hierarchy: bool = True


@dataclass
class SynthReport:
    gate_count: dict[str, int]   # cell_type → count
    total_gates: int
    flop_count: int
    latch_count: int             # warn if > 0 unintentional
    memory_blocks: int
    fsm_count: int
    rejected: list[str]          # synth-incompatible constructs encountered
    warnings: list[str]


class Synthesizer:
    def __init__(self, hir: "HIR", options: SynthOptions = SynthOptions()):
        ...
    
    def run(self) -> tuple["Netlist", SynthReport]:
        ...
    
    def lint(self) -> list[str]:
        """Pre-flight: report all unsynthesizable constructs without producing HNL."""
        ...


# Internal pass interfaces

class Pass(Protocol):
    def run(self, hir: "HIR") -> "HIR": ...

class ConstantFolding(Pass): ...
class DeadCodeElim(Pass): ...
class CommonSubexprElim(Pass): ...
class FsmExtraction(Pass): ...
class OperatorLowering(Pass): ...
class ProcessClassification(Pass): ...
class HnlProjection: 
    def run(self, hir: "HIR") -> "Netlist": ...
```

## Edge Cases

| Scenario | Handling |
|---|---|
| Combinational loop in synthesized HNL | Detected post-synthesis; error. |
| Inferred latch | Warning + emit `DLATCH`. |
| Multiple drivers on same wire (Verilog `always` + `assign`) | Error. |
| Wide arithmetic (e.g., 256-bit add) | Adder generator is unbounded; warn if width > 64. |
| Multiplier > 32 bits | Generator works but slow; warn. |
| Array index that may be out of range | Tristate the load (read 'X'); store ignored. |
| Memory with byte-enable | Reduce to N parallel single-bit memories. |
| Generate-block hierarchy preserve | Per `keep_hierarchy` flag. |
| User attribute `(* keep *)` on a wire | Suppress dead-code-elim of that wire. |
| User attribute `(* mark_debug *)` | Preserve through synthesis to be visible in waveforms. |
| Recursive function | Reject (synthesis can't unroll). |
| `disable` statement | Reject. |
| `event` declaration | Reject. |
| `parameter` types other than integer/logic | Allowed; treated as compile-time. |

## Test Strategy

### Unit (target 95%+)
- Each operator's generator emits the right gate count and structure.
- Process classification: sequential, combinational, latch detected correctly.
- FF inference: posedge / negedge / async reset / sync reset.
- FSM detection: state register + case → records FSM metadata.
- Constant folding: each rule has positive/negative tests.
- Memory inference: detects read/write ports.
- Latch warning fires when expected.

### Integration
- 4-bit adder produces 20-cell HNL; matches reference.
- 32-bit ALU produces ~600-cell HNL; matches reference within 5%.
- ARM1 reference design synthesizes; gate count comparable to `arm1-gatelevel`.
- Equivalence checking by simulation: HIR sim and synthesized HNL sim produce same waveforms on testbench.

### Property
- Idempotence: synthesizing already-structural HIR is a no-op.
- Determinism: same HIR + same options → same HNL.
- Optimization-monotone: `optimize=False` produces ≥ as many cells as `optimize=True`.

## Conformance Matrix

| Standard / construct | Coverage |
|---|---|
| **IEEE 1076.6** (VHDL synthesis) | Full subset; explicit reject on others. |
| **Verilog 2001 synth** (de facto) | Full subset. |
| **Cell types produced** | All from `gate-netlist-format.md` built-ins. |
| FSM encoding hints (attributes) | Recognized: `(* fsm_encoding = "one_hot" *)` etc. |
| `(* keep *)`, `(* mark_debug *)`, `(* synth_off *)` | Recognized. |
| Memory inference | Single-/dual-port; byte-enable; init-from-file (`$readmemh`). |

## Open Questions

1. **Should we have an AIG (And-Inverter Graph) intermediate?** Recommendation: defer. ABC-style optimization is a future spec.
2. **Tech-independent optimizations** (like `yosys -p "opt; opt; opt"`)? Recommendation: light pass for v1; aggressive in future.
3. **Carry-lookahead by default** for adders? Recommendation: ripple by default, CLA when width > 16 (after measurement).
4. **Multi-cycle paths / pipelining annotations**? Defer; future.

## Future Work

- AIG IR for ABC-style optimization.
- Retiming.
- Pipelining (auto-insert pipeline registers under designer guidance).
- Clock gating insertion.
- Multi-cycle path handling.
- Resource sharing (a single multiplier serving multiple sites).
- Mealy/Moore FSM transformation to optimize specific encodings.
- LEC (logic equivalence checking) by SAT/BDD.
