# HDL IR (HIR)

## Overview

This is the keystone of the silicon stack. **HIR** (Hardware IR) is the unified intermediate representation that every hardware description — Verilog, VHDL, Ruby DSL — elaborates *to*, and that every consumer — simulator, synthesizer, waveform writer, FPGA mapper, ASIC backend — operates *on*. Without HIR, every consumer would have to handle three (and eventually more) source languages independently. With HIR, every front-end converges to one shape and every back-end reads one shape; the M×N integration matrix collapses to M+N.

HIR's job is to express **what a digital circuit means** in a form that:

1. **Preserves IEEE semantics faithfully** — full 1076-2008 (VHDL) and 1364-2005 (Verilog) constructs are first-class, including testbench, delay, file I/O, assertions. (The Ruby DSL only generates a synthesizable subset of HIR; that's a property of the DSL, not HIR.)
2. **Normalizes source-language quirks** — `wire` vs `reg` (Verilog) and `signal` vs `variable` (VHDL) are unified to a single `Net` concept; the original kind is preserved as metadata for re-emission.
3. **Is JSON-serializable end-to-end** — every node serializes to a strict JSON schema. This is the convergence contract for polyglot ports: a Python-emitted HIR file must be readable by a Rust consumer, byte-for-byte.
4. **Is strongly typed** — `mypy --strict` over the Python reference implementation. Width-checked nets, direction-checked ports, named processes with explicit sensitivity / continuation points.
5. **Carries provenance** — every node knows which front-end emitted it (`Verilog`, `VHDL`, `RubyDSL`) and where in the source it came from. This makes diagnostics from any back-end actionable.
6. **Distinguishes IR levels** — `HIR.behavioral`, `HIR.structural`, downstream `HNL`. Validators enforce per-level invariants.

### Why one IR

| Approach | Pros | Cons |
|---|---|---|
| **Per-language IRs** (one for VHDL, one for Verilog, one for Ruby DSL) | Each IR is a clean reflection of its source language. | Every back-end has to handle three IRs. Optimizers triplicate. Cross-language reuse impossible. |
| **One unified IR (HIR — this spec)** | Single back-end matrix. Optimizers written once. Cross-language analysis natural. | Must accommodate semantic differences carefully. Provenance metadata required. |

The repo's convergence-and-multi-language stance (per `F05-verilog-vhdl.md`'s shared grammar infrastructure) makes the unified IR the natural choice. Source-language provenance is a property on each node, not a separate IR per source.

### Generality

HIR represents arbitrary digital circuits. The 4-bit adder is the smoke-test example here, but the same IR shape encodes:

- A 32-bit ALU (~600 cells when synthesized).
- A 5-stage RISC-V pipeline (~10K cells; existing `arm1-gatelevel`, `intel4004-gatelevel` reference designs are HIR-target candidates).
- A small SoC with a CPU + memory + UART + GPIO (~50K cells).
- Industrial blocks (millions of cells).

The data-structure shape is constant. Only the cardinality scales.

## Layer Position

```
        Verilog source            VHDL source              Ruby DSL source
              │                        │                         │
              ▼                        ▼                         ▼
       verilog-parser            vhdl-parser           ruby-hdl-dsl elaboration
       AST (per F05)            AST (per F05)          (trace-style; no AST)
              │                        │                         │
              └────────┬───────────────┴─────────────┬───────────┘
                       ▼                             ▼
              ┌──────────────────────────────────────────┐
              │  hdl-elaboration.md                      │
              │  AST + DSL traces → HIR                  │
              └──────────────────────────────────────────┘
                       │
                       ▼
              ┌──────────────────────────────────────────┐
              │  HIR (this spec)                         │
              │  level: behavioral | structural          │
              └──────────────────────────────────────────┘
                       │
       ┌───────────────┼─────────────────┬──────────────┐
       ▼               ▼                 ▼              ▼
hardware-vm.md   synthesis.md       coverage.md   real-fpga-export.md
                       │
                       ▼
                  HNL (gate-netlist-format.md)
```

## Concepts

### What HIR represents

HIR represents a **hierarchy of modules**, where each module:
- Has external **ports** (input/output/inout, with width and type).
- Has internal **nets** (named wires of a given type and width).
- Has **processes** that define behavior over time (encompassing both VHDL `process` and Verilog `always` / `initial`).
- Has **continuous assignments** (VHDL concurrent signal assignments, Verilog `assign`).
- Has **instances** of other modules (hierarchical structure).
- Optionally has **parameters** / **generics** that are bound at elaboration.

### Behavioral vs structural

A module is **structural** when all its content is `instances` and `continuous assignments` — there are no `processes`. Structural modules describe a circuit's connectivity directly.

A module is **behavioral** when it contains `processes` — bodies of sequential statements (assignments, conditionals, loops) executed in response to events. Behavioral modules describe how the circuit *should behave*; synthesis is responsible for inferring the structural circuit from the behavior.

HIR represents both with the same node types. The `level` field on a Module distinguishes them; validators enforce per-level invariants.

```
level=behavioral:  processes ∪ continuous_assigns ∪ instances are all allowed
level=structural:  only continuous_assigns ∪ instances; no processes
```

A separate downstream IR, `HNL` (defined in `gate-netlist-format.md`), is even more restricted — only primitive cells in instances, no continuous assigns. The chain of refinements is: `HIR.behavioral → HIR.structural → HNL`.

### Net, port, signal, variable: one model with provenance

VHDL distinguishes:
- `signal` — global, scheduled (signal assignments are deferred; `<=`).
- `variable` — local to a process, immediate (`:=`).

Verilog distinguishes:
- `wire` — driven by continuous assigns or gate outputs.
- `reg` — assigned in `always` / `initial` blocks (despite the name, not necessarily a register; can synthesize to combinational).
- `tri`, `tri0`, `tri1`, `wand`, `wor` — special net types for tri-state and wired logic.

HIR normalizes all of these to two concepts:

- **`Net`** — anything that connects between processes / instances / assigns. Includes all VHDL signals and Verilog wires/regs. Has `width`, `type`, and `kind` metadata.
- **`Variable`** — process-local storage with immediate semantics. Includes VHDL `variable`. Verilog has no direct equivalent, but `reg` inside an `always_comb` is conceptually similar.

The `kind` field on a Net captures the original source-language flavor:

| Source kind | Meaning | HIR `kind` |
|---|---|---|
| VHDL signal | Default; scheduled assignment | `signal` |
| Verilog wire | Driven by `assign` or gate output | `wire` |
| Verilog reg | Assigned in always/initial; deferred semantics | `reg` |
| Verilog tri | Tristate net | `tri` |
| Verilog wand | Wired-AND | `wand` |
| Verilog wor | Wired-OR | `wor` |
| VHDL signal w/ resolution | std_logic with resolution function | `resolved_signal` |

The simulator (`hardware-vm.md`) consults `kind` for resolution rules; synthesis treats them all as nets.

### Type system

HIR types are richer than HNL types — HIR supports user-defined records, enumerations, arrays, integers, reals, time, strings — to faithfully encode VHDL and Verilog semantics. Synthesis projects all types to bit vectors before producing HNL.

| Type | Description | Source-language origin |
|---|---|---|
| `Logic` | Single-bit 4-state value: `0`, `1`, `X`, `Z` | Verilog default; widely used |
| `Bit` | Single-bit 2-state value: `0`, `1` | VHDL `bit`; mostly historical |
| `StdLogic` | Single-bit 9-state value (IEEE 1164): `U,X,0,1,Z,W,L,H,-` | VHDL `ieee.std_logic_1164` |
| `Vector(T, n)` | n-bit array of T (left-to-right or right-to-left) | Verilog `[N-1:0] x`, VHDL `std_logic_vector(N-1 downto 0)` |
| `Integer(low, high)` | Bounded integer | VHDL `integer range 0 to 255`, Verilog `integer` |
| `Real` | IEEE 754 double | Both languages |
| `Time` | Simulated time | Both (Verilog `time`, VHDL `time`) |
| `String` | Variable-length string | Both |
| `Enum(name, members)` | User-defined enumeration | VHDL `type state is (red, green, yellow)` |
| `Record(name, fields)` | User-defined struct | VHDL `record`; Verilog `struct` (1800 only — out of scope) |
| `Array(T, range)` | Multi-dimensional array | Both |
| `File(T)` | File handle | VHDL textio; Verilog file handles |

Type compatibility rules follow IEEE: Verilog allows implicit conversions between integer and bit-vector; VHDL requires explicit casts.

### Process semantics

A `Process` is a body of sequential code with one of two trigger modes:

- **Sensitivity-list mode** — wakes when any signal in `sensitivity` changes.
- **Wait-mode** — has explicit `wait` statements inside the body. Sensitivity list must be empty in this case.

VHDL allows either; Verilog `always @(...)` is sensitivity-list mode; Verilog `initial` is wait-mode (runs once at time 0).

The process body executes until it suspends (via `wait`, end of body, or `@`). The simulator (`hardware-vm.md`) implements this via continuation-passing-style — every suspend point is a continuation save.

```
Process(
  name="counter_proc",
  sensitivity=[Net("clk").posedge],
  body=[
    IfStmt(
      cond=BinOp("==", Var("reset"), Lit(1)),
      then_branch=[NonblockingAssign(Var("count"), Lit(0))],
      else_branch=[NonblockingAssign(Var("count"), BinOp("+", Var("count"), Lit(1)))]
    )
  ]
)
```

### Continuous assignments

A continuous assignment drives a net from an expression that is re-evaluated whenever its inputs change.

```vhdl
sum <= a + b + cin;     -- VHDL
```

```verilog
assign sum = a + b + cin;   // Verilog
```

In HIR:

```python
ContAssign(target=NetSlice(net="sum", bits=...),
           expr=BinOp("+", BinOp("+", NetRef("a"), NetRef("b")), NetRef("cin")))
```

The simulator evaluates the right side and updates the target whenever any contributing net changes. Synthesis turns continuous assigns into combinational logic.

### Instance hierarchy

Instances reference other modules:

```python
Instance(
  name="u_fa0",
  module="full_adder",
  parameters={},
  connections={
    "a": NetSlice("a", [0]),
    "b": NetSlice("b", [0]),
    "cin": NetSlice("cin", [0]),
    "sum": NetSlice("sum", [0]),
    "cout": NetSlice("c0", [0]),
  }
)
```

Module references resolve at elaboration. Parameter binding (`#(.WIDTH(8))` in Verilog, `generic map` in VHDL) is also resolved at elaboration.

## HIR Data Model

```python
from dataclasses import dataclass, field
from enum import Enum
from typing import Literal, Union


# ═══════════════════════════════════════════════════════════════
# Source-language provenance
# ═══════════════════════════════════════════════════════════════

class SourceLang(Enum):
    VERILOG  = "verilog"
    VHDL     = "vhdl"
    RUBY_DSL = "ruby_dsl"
    UNKNOWN  = "unknown"      # for hand-built test fixtures


@dataclass(frozen=True)
class SourceLocation:
    file: str
    line: int
    column: int


@dataclass(frozen=True)
class Provenance:
    """Where a node came from."""
    lang: SourceLang
    location: SourceLocation | None = None


# ═══════════════════════════════════════════════════════════════
# Types
# ═══════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class TyLogic:        pass
@dataclass(frozen=True)
class TyBit:          pass
@dataclass(frozen=True)
class TyStdLogic:     pass
@dataclass(frozen=True)
class TyReal:         pass
@dataclass(frozen=True)
class TyTime:         pass
@dataclass(frozen=True)
class TyString:       pass

@dataclass(frozen=True)
class TyVector:
    elem: "Ty"
    width: int
    msb_first: bool = True   # True: [N-1:0]; False: [0:N-1]

@dataclass(frozen=True)
class TyInteger:
    low: int
    high: int

@dataclass(frozen=True)
class TyEnum:
    name: str
    members: tuple[str, ...]

@dataclass(frozen=True)
class TyRecord:
    name: str
    fields: tuple[tuple[str, "Ty"], ...]

@dataclass(frozen=True)
class TyArray:
    elem: "Ty"
    low: int
    high: int

@dataclass(frozen=True)
class TyFile:
    elem: "Ty"

Ty = Union[TyLogic, TyBit, TyStdLogic, TyReal, TyTime, TyString,
           TyVector, TyInteger, TyEnum, TyRecord, TyArray, TyFile]


def width(t: Ty) -> int:
    """Bit width of a type after synthesis projection. Used by HNL boundaries."""
    if isinstance(t, (TyLogic, TyBit, TyStdLogic)): return 1
    if isinstance(t, TyVector): return t.width * width(t.elem)
    if isinstance(t, TyInteger): return max(1, (t.high - t.low + 1).bit_length())
    if isinstance(t, TyEnum):    return max(1, (len(t.members) - 1).bit_length())
    if isinstance(t, TyRecord):  return sum(width(ft) for _, ft in t.fields)
    if isinstance(t, TyArray):   return (t.high - t.low + 1) * width(t.elem)
    raise ValueError(f"width() not defined for {t!r} (type may be unsynthesizable)")
```

```python
# ═══════════════════════════════════════════════════════════════
# Nets, variables, ports
# ═══════════════════════════════════════════════════════════════

class NetKind(Enum):
    SIGNAL          = "signal"
    WIRE            = "wire"
    REG             = "reg"
    TRI             = "tri"
    WAND            = "wand"
    WOR             = "wor"
    SUPPLY0         = "supply0"
    SUPPLY1         = "supply1"
    RESOLVED_SIGNAL = "resolved_signal"


@dataclass(frozen=True)
class Net:
    name: str
    type: Ty
    kind: NetKind = NetKind.SIGNAL
    initial: "Expr | None" = None
    provenance: Provenance | None = None


@dataclass(frozen=True)
class Variable:
    """Process-local storage with immediate-update semantics."""
    name: str
    type: Ty
    initial: "Expr | None" = None
    provenance: Provenance | None = None


class Direction(Enum):
    IN    = "in"
    OUT   = "out"
    INOUT = "inout"
    BUFFER = "buffer"   # VHDL buffer mode


@dataclass(frozen=True)
class Port:
    name: str
    direction: Direction
    type: Ty
    default: "Expr | None" = None
    provenance: Provenance | None = None


# ═══════════════════════════════════════════════════════════════
# Expressions
# ═══════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class Lit:
    value: int | bool | float | str | tuple[int, ...]   # tuple for vector literal
    type: Ty
    provenance: Provenance | None = None

@dataclass(frozen=True)
class NetRef:
    name: str
    provenance: Provenance | None = None

@dataclass(frozen=True)
class VarRef:
    name: str
    provenance: Provenance | None = None

@dataclass(frozen=True)
class PortRef:
    name: str
    provenance: Provenance | None = None

@dataclass(frozen=True)
class Slice:
    """Bit/range select on an Expr."""
    base: "Expr"
    msb: int
    lsb: int
    provenance: Provenance | None = None

@dataclass(frozen=True)
class Concat:
    parts: tuple["Expr", ...]
    provenance: Provenance | None = None

@dataclass(frozen=True)
class Replication:
    count: "Expr"
    body: "Expr"
    provenance: Provenance | None = None

@dataclass(frozen=True)
class UnaryOp:
    op: str   # 'NOT', 'NEG', 'AND_RED', 'OR_RED', 'XOR_RED', 'PLUS', 'MINUS'
    operand: "Expr"
    provenance: Provenance | None = None

@dataclass(frozen=True)
class BinaryOp:
    op: str   # '+', '-', '*', '/', '%', 'AND', 'OR', 'XOR', '<<', '>>',
              # '<', '<=', '>', '>=', '==', '!=', '===', '!==', '&&', '||'
    lhs: "Expr"
    rhs: "Expr"
    provenance: Provenance | None = None

@dataclass(frozen=True)
class Ternary:
    cond: "Expr"
    then_expr: "Expr"
    else_expr: "Expr"
    provenance: Provenance | None = None

@dataclass(frozen=True)
class FunCall:
    name: str
    args: tuple["Expr", ...]
    provenance: Provenance | None = None

@dataclass(frozen=True)
class SystemCall:
    """$display, $time, $random, etc."""
    name: str
    args: tuple["Expr", ...]
    provenance: Provenance | None = None

@dataclass(frozen=True)
class Attribute:
    """signal'event, signal'last_value, etc."""
    base: "Expr"
    name: str
    args: tuple["Expr", ...] = ()
    provenance: Provenance | None = None

Expr = Union[Lit, NetRef, VarRef, PortRef, Slice, Concat, Replication,
             UnaryOp, BinaryOp, Ternary, FunCall, SystemCall, Attribute]


# ═══════════════════════════════════════════════════════════════
# Statements
# ═══════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class BlockingAssign:
    """Verilog =; VHDL := for variables."""
    target: Expr     # NetRef / VarRef / Slice / Concat
    rhs: Expr
    delay: "Expr | None" = None
    provenance: Provenance | None = None

@dataclass(frozen=True)
class NonblockingAssign:
    """Verilog <= ; VHDL <= for signals."""
    target: Expr
    rhs: Expr
    delay: "Expr | None" = None
    provenance: Provenance | None = None

@dataclass(frozen=True)
class IfStmt:
    cond: Expr
    then_branch: tuple["Stmt", ...]
    else_branch: tuple["Stmt", ...] = ()
    provenance: Provenance | None = None

@dataclass(frozen=True)
class CaseStmt:
    expr: Expr
    items: tuple["CaseItem", ...]
    default: tuple["Stmt", ...] | None = None
    kind: str = "case"   # 'case', 'casex', 'casez', 'casez_priority', etc.
    provenance: Provenance | None = None

@dataclass(frozen=True)
class CaseItem:
    choices: tuple[Expr, ...]
    body: tuple["Stmt", ...]

@dataclass(frozen=True)
class ForStmt:
    init: "Stmt"
    cond: Expr
    step: "Stmt"
    body: tuple["Stmt", ...]
    provenance: Provenance | None = None

@dataclass(frozen=True)
class WhileStmt:
    cond: Expr
    body: tuple["Stmt", ...]
    provenance: Provenance | None = None

@dataclass(frozen=True)
class RepeatStmt:
    count: Expr
    body: tuple["Stmt", ...]
    provenance: Provenance | None = None

@dataclass(frozen=True)
class ForeverStmt:
    body: tuple["Stmt", ...]
    provenance: Provenance | None = None

@dataclass(frozen=True)
class WaitStmt:
    on: tuple[Expr, ...]    # signals; empty = no sensitivity
    until: Expr | None      # condition
    for_: Expr | None       # timeout
    provenance: Provenance | None = None

@dataclass(frozen=True)
class DelayStmt:
    """#10 alone; used for procedural delays."""
    amount: Expr
    body: tuple["Stmt", ...]
    provenance: Provenance | None = None

@dataclass(frozen=True)
class EventStmt:
    """@(posedge clk) — wait for an event."""
    events: tuple["Event", ...]
    body: tuple["Stmt", ...]
    provenance: Provenance | None = None

@dataclass(frozen=True)
class Event:
    edge: str   # 'posedge', 'negedge', 'change'
    expr: Expr

@dataclass(frozen=True)
class AssertStmt:
    cond: Expr
    message: Expr | None = None
    severity: str = "error"   # 'note', 'warning', 'error', 'failure'
    provenance: Provenance | None = None

@dataclass(frozen=True)
class ReportStmt:
    message: Expr
    severity: str = "note"
    provenance: Provenance | None = None

@dataclass(frozen=True)
class DisableStmt:
    target: str   # name of named block
    provenance: Provenance | None = None

@dataclass(frozen=True)
class ReturnStmt:
    value: Expr | None
    provenance: Provenance | None = None

@dataclass(frozen=True)
class NullStmt:
    provenance: Provenance | None = None

@dataclass(frozen=True)
class ExprStmt:
    """A bare expression used for side effect (function call etc.)"""
    expr: Expr
    provenance: Provenance | None = None

Stmt = Union[BlockingAssign, NonblockingAssign, IfStmt, CaseStmt,
             ForStmt, WhileStmt, RepeatStmt, ForeverStmt,
             WaitStmt, DelayStmt, EventStmt, AssertStmt, ReportStmt,
             DisableStmt, ReturnStmt, NullStmt, ExprStmt]


# ═══════════════════════════════════════════════════════════════
# Processes, instances, continuous assignments
# ═══════════════════════════════════════════════════════════════

class ProcessKind(Enum):
    ALWAYS    = "always"      # Verilog always
    INITIAL   = "initial"     # Verilog initial
    PROCESS   = "process"     # VHDL process
    ALWAYS_FF = "always_ff"   # SV; future
    ALWAYS_COMB = "always_comb" # SV; future


@dataclass(frozen=True)
class Process:
    name: str | None       # named processes are common in VHDL
    kind: ProcessKind
    sensitivity: tuple["SensitivityItem", ...]   # empty if wait-mode
    variables: tuple[Variable, ...]              # process-local
    body: tuple[Stmt, ...]
    provenance: Provenance | None = None


@dataclass(frozen=True)
class SensitivityItem:
    edge: str    # 'posedge', 'negedge', 'change' (any change)
    expr: Expr


@dataclass(frozen=True)
class ContAssign:
    target: Expr
    rhs: Expr
    delay: Expr | None = None
    provenance: Provenance | None = None


@dataclass(frozen=True)
class Instance:
    name: str
    module: str                         # name of the module being instantiated
    parameters: dict[str, Expr]         # generic / parameter bindings
    connections: dict[str, Expr]        # port name → expression (typically NetRef or Slice)
    provenance: Provenance | None = None


# ═══════════════════════════════════════════════════════════════
# Modules
# ═══════════════════════════════════════════════════════════════

class Level(Enum):
    BEHAVIORAL = "behavioral"
    STRUCTURAL = "structural"


@dataclass
class Module:
    name: str
    ports: list[Port]
    parameters: list["Parameter"]      # generic / parameter declarations
    nets: list[Net]
    instances: list[Instance]
    cont_assigns: list[ContAssign]
    processes: list[Process]
    level: Level = Level.BEHAVIORAL
    attributes: dict[str, Expr] = field(default_factory=dict)
    provenance: Provenance | None = None


@dataclass(frozen=True)
class Parameter:
    name: str
    type: Ty
    default: Expr
    provenance: Provenance | None = None


@dataclass
class Library:
    """A collection of related modules; corresponds to a VHDL library."""
    name: str
    modules: dict[str, Module] = field(default_factory=dict)


@dataclass
class HIR:
    """Top-level container."""
    top: str
    modules: dict[str, Module]
    libraries: dict[str, Library] = field(default_factory=dict)
    provenance: Provenance | None = None
    version: str = "0.1.0"

    def validate(self) -> "ValidationReport":
        ...

    def to_json(self, path: str) -> None:
        ...

    @classmethod
    def from_json(cls, path: str) -> "HIR":
        ...

    def stats(self) -> "HIRStats":
        ...
```

## JSON Schema

```json
{
  "format": "HIR",
  "version": "0.1.0",
  "top": "adder4",
  "modules": {
    "adder4": {
      "level": "behavioral",
      "provenance": { "lang": "verilog", "location": { "file": "adder4.v", "line": 1, "column": 0 } },
      "ports": [
        { "name": "a",   "direction": "in",  "type": { "kind": "vector", "elem": { "kind": "logic" }, "width": 4 } },
        { "name": "b",   "direction": "in",  "type": { "kind": "vector", "elem": { "kind": "logic" }, "width": 4 } },
        { "name": "cin", "direction": "in",  "type": { "kind": "logic" } },
        { "name": "sum", "direction": "out", "type": { "kind": "vector", "elem": { "kind": "logic" }, "width": 4 } },
        { "name": "cout","direction": "out", "type": { "kind": "logic" } }
      ],
      "parameters": [],
      "nets": [
        { "name": "c0", "type": {"kind":"logic"}, "kind": "wire" },
        { "name": "c1", "type": {"kind":"logic"}, "kind": "wire" },
        { "name": "c2", "type": {"kind":"logic"}, "kind": "wire" }
      ],
      "instances": [
        {
          "name": "u_fa0", "module": "full_adder", "parameters": {},
          "connections": {
            "a":    { "kind": "slice", "base": {"kind":"port_ref", "name":"a"}, "msb": 0, "lsb": 0 },
            "b":    { "kind": "slice", "base": {"kind":"port_ref", "name":"b"}, "msb": 0, "lsb": 0 },
            "cin":  { "kind": "port_ref", "name": "cin" },
            "sum":  { "kind": "slice", "base": {"kind":"port_ref", "name":"sum"}, "msb": 0, "lsb": 0 },
            "cout": { "kind": "net_ref", "name": "c0" }
          }
        }
      ],
      "cont_assigns": [],
      "processes": []
    }
  }
}
```

Schema rules:
- Every `Expr` JSON object has a `kind` discriminator (`lit`, `net_ref`, `port_ref`, `slice`, `concat`, `binop`, etc.).
- Every `Stmt` has a `kind` discriminator.
- Every `Ty` has a `kind` discriminator.
- Names follow Verilog identifier rules.
- Bit widths are explicit; types fully specified.
- The schema is **frozen** at v0.1.0; future versions add nodes additively.

## IEEE Construct Coverage

This is the keystone matrix: every IEEE 1076-2008 / 1364-2005 construct mapped to its HIR representation.

### Verilog (IEEE 1364-2005)

| Verilog construct | HIR mapping |
|---|---|
| `module ... endmodule` | `Module` |
| `input/output/inout port` | `Port(direction)` |
| `wire`, `reg`, `tri`, etc. | `Net(kind=...)` |
| `parameter` | `Parameter` |
| `localparam` | `Parameter` (immutable, no override) |
| `assign x = y` | `ContAssign` |
| `always @(...)` | `Process(kind=ALWAYS, sensitivity=...)` |
| `initial begin ... end` | `Process(kind=INITIAL, sensitivity=())` |
| `if/else` | `IfStmt` |
| `case`, `casex`, `casez` | `CaseStmt(kind=...)` |
| `for/while/repeat/forever` | `ForStmt` / `WhileStmt` / `RepeatStmt` / `ForeverStmt` |
| `=` (blocking) | `BlockingAssign` |
| `<=` (non-blocking) | `NonblockingAssign` |
| `#10` delay | `DelayStmt(amount=Lit(10))` |
| `@(posedge clk)` | `EventStmt(events=[Event("posedge", NetRef("clk"))])` |
| `wait (cond) stmt` | `WaitStmt(until=cond, body=stmt)` |
| `disable foo` | `DisableStmt("foo")` |
| `force`/`release` | `ForceStmt`, `ReleaseStmt` (additional Stmt nodes) |
| `function` declaration | `FunctionDecl` (Module-level addition) |
| `task` declaration | `TaskDecl` |
| `generate for/if/case` | unrolled at elaboration; produce instances |
| `genvar` | elaboration variable; not in HIR after elaboration |
| `$display, $monitor, $finish` | `SystemCall` inside `ExprStmt` |
| `$readmemh, $readmemb` | `SystemCall` (special-cased in simulator for memory init) |
| `$time, $stime, $realtime` | `SystemCall` (in expressions) |
| primitive (UDP) | `Module(level=structural, attributes={"udp": True})` with truth-table attribute |
| `(* attr *)` | `Module/Instance/.attributes[name] = value` |
| `specify` block | `Module.attributes["specify"] = SpecifyData(...)` |
| `config` declaration | top-level `ConfigDecl`; resolved at elaboration |
| Strength specifiers | optional metadata on `ContAssign` and primitive instances |
| `===, !==` (case equality) | `BinaryOp("===", ...)`; preserved (4-state semantics) |

### VHDL (IEEE 1076-2008)

| VHDL construct | HIR mapping |
|---|---|
| `entity ... end entity` | `Module` (entity portion) |
| `architecture ... end` | `Module` body (one Module per entity-architecture pair, or merged) |
| `port (... )` | `Module.ports` |
| `generic (... )` | `Module.parameters` |
| `signal x : T;` | `Net(kind=SIGNAL)` |
| `variable x : T;` (in process) | `Variable` |
| `constant x : T := v;` | `Parameter` (immutable) |
| `x <= e;` (concurrent) | `ContAssign` |
| `x <= e;` (sequential) | `NonblockingAssign` |
| `x := e;` | `BlockingAssign` |
| `process(a, b)` | `Process(kind=PROCESS, sensitivity=[change(a), change(b)])` |
| `process; ... wait until ...; end process;` | `Process(sensitivity=())` with `WaitStmt` in body |
| `if ... then ... elsif ... else ... end if;` | nested `IfStmt` |
| `case ... when ... =>` | `CaseStmt` |
| `for i in 0 to N-1 loop` | `ForStmt` |
| `while ... loop` | `WhileStmt` |
| `loop ... end loop;` | `ForeverStmt` |
| `wait`, `wait on`, `wait until`, `wait for` | `WaitStmt` (combinations supported) |
| `assert cond report msg severity sev;` | `AssertStmt` |
| `report msg severity sev;` | `ReportStmt` |
| `null;` | `NullStmt` |
| `return [expr];` | `ReturnStmt` |
| `next [label];`, `exit [label];` | additional Stmt nodes (ExitStmt, NextStmt) |
| `function f(...) return T is ... end f;` | `FunctionDecl` |
| `procedure p(...) is ... end p;` | `ProcedureDecl` |
| `for ... generate`, `if ... generate`, `case ... generate` | unrolled at elaboration |
| `component ... end component;` | reference to a Module signature |
| `instance: comp port map (...)` | `Instance` |
| `attribute name : type;` | `Module/.../.attributes[name]` |
| `attribute name of e : ent_class is val;` | sets attribute on referenced entity |
| `record` type | `TyRecord` |
| `array` type | `TyArray` |
| `file` declaration | `FileDecl` (Module-level), `TyFile` |
| `textio.read`, `textio.write` | `FunCall` with a registered I/O signature |
| `library work; use ieee.numeric_std.all;` | resolved at elaboration; not in HIR |
| `package ... package body ...` | resolved at elaboration; constants/types/functions injected |
| `configuration` | `ConfigDecl`; resolved at elaboration |
| `block ... end block;` | `BlockStmt` (additional Stmt) |
| `guarded` signal assignment | `ContAssign` with `guard` attribute |
| Predefined signal attributes: `'event`, `'last_value`, `'stable`, ... | `Attribute` expression |

### Constructs deliberately not represented in HIR

| Construct | Why not |
|---|---|
| Verilog `defparam` | Deprecated in IEEE 1364; resolved at elaboration if encountered, warned. |
| VHDL aliases | Resolved at elaboration to the aliased entity. |
| VHDL `use` clauses | Elaboration-only. |
| SystemVerilog (1800) constructs | Out of scope per master plan. |
| AMS / Verilog-AMS | Out of scope. |
| PSL assertions | Parsed parse-skip; future spec. |

## Validation Rules

| ID | Rule | Severity | Level |
|---|---|---|---|
| H1 | Every Module name is unique within HIR | Error | All |
| H2 | Top module exists | Error | All |
| H3 | Every Instance.module resolves to a Module | Error | All |
| H4 | Every Instance.connections key is an actual port of the target module | Error | All |
| H5 | Connection types compatible (width-checked) | Error | All |
| H6 | Every NetRef/PortRef/VarRef resolves | Error | All |
| H7 | Every Net has at most one driver per bit (unless multi-driver kind: tri/wand/wor or all drivers are inside a process) | Error | All |
| H8 | Every continuous assign target is non-overlapping with other drivers | Error | All |
| H9 | Process sensitivity OR wait-mode (mutually exclusive) | Error | All |
| H10 | No combinational loop without a sequential cell on the cycle | Error | Structural+ |
| H11 | Every parameter has a default value or is bound at elaboration | Error | All |
| H12 | Every Module marked structural has no `processes` | Error | Structural |
| H13 | Every type in a port/net has a known synthesizable width | Warning | Behavioral; Error in Structural |
| H14 | Initial values respect type bounds | Error | All |
| H15 | Function/procedure bodies terminate (no infinite recursion in static analysis sense) | Warning | Behavioral |
| H16 | Verilog blocking-assignment to non-`reg` net | Error | Behavioral |
| H17 | VHDL signal assignment from non-`signal` target | Error | Behavioral |
| H18 | `wait` statement only inside a process with empty sensitivity list | Error | Behavioral |
| H19 | `disable` target is a valid named block in scope | Error | Behavioral |
| H20 | Module not self-instantiating (transitively) | Error | All |

## Worked Example 1 — 4-bit Adder (HIR for Verilog source)

Verilog source:

```verilog
module adder4(input [3:0] a, b, input cin,
              output [3:0] sum, output cout);
  assign {cout, sum} = a + b + cin;
endmodule
```

HIR (compact):

```python
HIR(
  top="adder4",
  modules={
    "adder4": Module(
      name="adder4",
      ports=[
        Port("a",   Direction.IN,  TyVector(TyLogic(), 4)),
        Port("b",   Direction.IN,  TyVector(TyLogic(), 4)),
        Port("cin", Direction.IN,  TyLogic()),
        Port("sum", Direction.OUT, TyVector(TyLogic(), 4)),
        Port("cout",Direction.OUT, TyLogic()),
      ],
      parameters=[],
      nets=[],
      instances=[],
      cont_assigns=[
        ContAssign(
          target=Concat(parts=(PortRef("cout"), PortRef("sum"))),
          rhs=BinaryOp("+",
                       BinaryOp("+", PortRef("a"), PortRef("b")),
                       PortRef("cin"))
        )
      ],
      processes=[],
      level=Level.BEHAVIORAL,
      provenance=Provenance(SourceLang.VERILOG, SourceLocation("adder4.v", 1, 0)),
    )
  }
)
```

## Worked Example 2 — 4-bit Adder (HIR for VHDL source)

VHDL source:

```vhdl
library ieee; use ieee.numeric_std.all;
entity adder4 is
  port (a, b : in  std_logic_vector(3 downto 0);
        cin  : in  std_logic;
        sum  : out std_logic_vector(3 downto 0);
        cout : out std_logic);
end entity adder4;

architecture rtl of adder4 is begin
  process(a, b, cin)
    variable tmp : unsigned(4 downto 0);
  begin
    tmp := unsigned('0' & a) + unsigned('0' & b) + unsigned'(0 => cin, others => '0');
    sum  <= std_logic_vector(tmp(3 downto 0));
    cout <= tmp(4);
  end process;
end architecture;
```

HIR (compact):

```python
HIR(
  top="adder4",
  modules={
    "adder4": Module(
      name="adder4",
      ports=[
        Port("a",   Direction.IN,  TyVector(TyStdLogic(), 4)),
        Port("b",   Direction.IN,  TyVector(TyStdLogic(), 4)),
        Port("cin", Direction.IN,  TyStdLogic()),
        Port("sum", Direction.OUT, TyVector(TyStdLogic(), 4)),
        Port("cout",Direction.OUT, TyStdLogic()),
      ],
      parameters=[],
      nets=[],
      instances=[],
      cont_assigns=[],
      processes=[
        Process(
          name=None,
          kind=ProcessKind.PROCESS,
          sensitivity=(SensitivityItem("change", PortRef("a")),
                       SensitivityItem("change", PortRef("b")),
                       SensitivityItem("change", PortRef("cin"))),
          variables=(Variable("tmp", TyVector(TyStdLogic(), 5)),),
          body=(
            BlockingAssign(VarRef("tmp"),
              BinaryOp("+",
                BinaryOp("+",
                  Concat((Lit(0, TyStdLogic()), PortRef("a"))),
                  Concat((Lit(0, TyStdLogic()), PortRef("b")))),
                Concat((Lit(0, TyStdLogic()), Concat((Lit(0, TyStdLogic()),
                                                      Concat((Lit(0, TyStdLogic()),
                                                              Concat((Lit(0, TyStdLogic()),
                                                                      PortRef("cin")))))))))
              )
            ),
            NonblockingAssign(PortRef("sum"),  Slice(VarRef("tmp"), 3, 0)),
            NonblockingAssign(PortRef("cout"), Slice(VarRef("tmp"), 4, 4)),
          ),
          provenance=Provenance(SourceLang.VHDL, SourceLocation("adder4.vhd", 9, 2)),
        )
      ],
      level=Level.BEHAVIORAL,
      provenance=Provenance(SourceLang.VHDL, SourceLocation("adder4.vhd", 2, 0)),
    )
  }
)
```

Both Verilog and VHDL versions converge to the same `Module` shape with different `Process`/`ContAssign` choices reflecting source-language idiom. Synthesis produces the same HNL from either.

## Worked Example 3 — 4-bit Adder (HIR for Ruby DSL source)

Ruby DSL source:

```ruby
class Adder4 < Module
  io = Bundle.new(
    a:    Input(UInt(4)),
    b:    Input(UInt(4)),
    cin:  Input(UInt(1)),
    sum:  Output(UInt(4)),
    cout: Output(UInt(1)),
  )
  full = io.a +& io.b + io.cin   # +& is Chisel-style "expand width" add
  io.sum  := full[3, 0]
  io.cout := full[4]
end
```

The DSL elaborator traces this and emits the same `Module` shape as the Verilog version, with `provenance.lang = SourceLang.RUBY_DSL`.

## Worked Example 4 — Sequential design (FSM) — generality demonstration

A 4-state FSM (Red → Green → Yellow → Red). HIR has a `Process` with sensitivity `[posedge(clk), posedge(reset)]`, an `IfStmt` inside for reset handling, a `CaseStmt` for state transitions:

```python
Process(
  name="fsm_proc",
  kind=ProcessKind.PROCESS,
  sensitivity=(SensitivityItem("posedge", NetRef("clk")),
               SensitivityItem("posedge", NetRef("reset"))),
  variables=(),
  body=(
    IfStmt(
      cond=BinaryOp("==", NetRef("reset"), Lit(1, TyLogic())),
      then_branch=(NonblockingAssign(NetRef("state"), Lit("Red", TyEnum("color", ("Red","Green","Yellow")))),),
      else_branch=(
        CaseStmt(
          expr=NetRef("state"),
          items=(
            CaseItem((Lit("Red",   ...),), (NonblockingAssign(NetRef("state"), Lit("Green",  ...)),)),
            CaseItem((Lit("Green", ...),), (NonblockingAssign(NetRef("state"), Lit("Yellow", ...)),)),
            CaseItem((Lit("Yellow",...),), (NonblockingAssign(NetRef("state"), Lit("Red",    ...)),)),
          ),
          default=None,
        ),
      )
    ),
  )
)
```

## Worked Example 5 — Mid-scale design (32-bit ALU)

The same `Module` shape encodes a 32-bit ALU with hundreds of internal nets and dozens of instances. The HIR doesn't grow architecturally; only `nets`, `instances`, `cont_assigns`, `processes` lists grow longer. After synthesis, the HNL has ~600 cells. After tech mapping, the HNL has ~600 standard cells. After place-and-route, the GDSII is a few KB.

The `arm1-gatelevel` reference design (existing in repo) is an HIR target that demonstrates HIR's capacity for industrial-scale designs.

## Edge Cases

| Scenario | Handling |
|---|---|
| Verilog `parameter` overridden by `defparam` | Resolved at elaboration; `defparam` warned. |
| VHDL `port map` with `=>` named association vs positional | Both parse; HIR uses dict (keyed). Order recorded for diagnostics. |
| Verilog `wire [3:0] x` vs `wire [0:3] x` | Recorded via `TyVector.msb_first`. |
| VHDL `signal x : std_logic_vector(3 downto 0)` vs `(0 to 3)` | Same as above. |
| Generate-for unrolling | Done at elaboration; HIR sees concrete instances. |
| `genvar i; for(i=0;i<N;i=i+1) begin ... end` | Unrolled. |
| Recursive functions | Supported; depth-limited at elaboration. |
| Self-referential parameters (`parameter X = X+1`) | Detected at elaboration; error. |
| Unbound port (Verilog `.x()`) | Modeled as port mapping to `Lit("Z", TyLogic())`. |
| VHDL `open` association | Same — port is unconnected; modeled as `Lit("Z", ...)`. |
| Mixed-language design (VHDL top instantiates Verilog component) | Allowed in HIR; modules' provenance differs. Elaborator must reconcile. |
| Module with same name in different libraries (VHDL) | HIR.libraries resolves; modules in `HIR.modules` use library-prefixed names. |
| Empty process (no statements) | Allowed; documented "no-op" warning. |
| `wait` followed by no body | Modeled as `WaitStmt(...)` followed by no statements; the wait is the entire body. |
| Verilog `casex`/`casez` with don't-care patterns | Preserved via `CaseStmt.kind`. |
| VHDL `case` with `=>` and `|` (alternatives) | `CaseItem.choices` is a tuple of alternatives. |
| Hierarchical signal reference (Verilog `top.u1.x`) | Modeled as nested NetRef chain; resolved at elaboration. |
| Forward references to modules | Allowed; resolved at elaboration end-of-pass. |

## Test Strategy

### Unit (target 95%+)
- Round-trip: `HIR → JSON → HIR` is identity (modulo dict ordering normalization).
- Construct each Stmt/Expr node type with valid arguments; round-trip through JSON.
- Validation rules H1–H20: positive + negative test each.
- Net-driver conflict (H7): two `assign` to same bit → error.
- Combinational loop detection (H10): two combinational processes that read each other → error.
- Cycle in module instantiation graph (H20): module M instantiates M → error.
- Width mismatch (H5): `wire [3:0] x; assign x = 1'b0;` → error.

### Property
- For random valid HIR documents: `validate()` returns ok.
- For random valid HIR documents with single mutation: validation flags it.
- The same conceptual circuit emitted from VHDL and Verilog elaborators produces semantically equivalent HIR (after normalization).

### Integration
- Verilog `adder4.v` parses → AST → elaborates → HIR. Validate: ok. Simulate: matches reference.
- VHDL `adder4.vhd` parses → AST → elaborates → HIR. Validate: ok. Simulate: matches reference. Same outputs as Verilog version on the same stimulus.
- Ruby DSL `Adder4` elaborates → HIR. Same as above.
- The full suite of testbenches in `arm1-simulator`, `intel4004-simulator` parses through the HIR pipeline.
- IEEE conformance: parse + elaborate test vectors from public IEEE 1076-2008 / 1364-2005 test suites where available.

## Conformance Matrix

| Construct class | IEEE 1364-2005 coverage | IEEE 1076-2008 coverage |
|---|---|---|
| Module / Entity | Full | Full |
| Ports / Generics | Full | Full |
| Signal / Wire / Reg / Variable | Full | Full |
| Continuous / Concurrent assignment | Full | Full |
| Always / Process | Full (always, initial) | Full (process) |
| Sensitivity / wait | Full | Full |
| Sequential statements (if/case/loops) | Full | Full |
| Tasks / Functions / Procedures | Full | Full |
| Generate / Generate-for/if/case | Full (for/if) | Full (for/if/case) |
| Hierarchy / Instances | Full | Full |
| Parameters / Generics binding | Full | Full |
| Attributes | Full | Full |
| Configurations | Full | Full |
| Specify / SDF back-annotation | Parse + annotate; semantics in hardware-vm | N/A in VHDL |
| UDPs | Full | N/A |
| File I/O / textio | Full | Full |
| Assert / Report / Severity | Full (`$assert` + immediate) | Full |
| Records / Aggregates | N/A in classic Verilog | Full |
| Predefined attributes (`'event`, etc.) | N/A | Full |
| PSL embedded | Out of scope | Out of scope |

## Open Questions

1. **Should HIR distinguish between Verilog `wire`/`reg` past elaboration?**
   The `kind` field captures it. Synthesis ignores `kind`; simulator consults it for resolution. Recommendation: keep but allow `kind=normalized` in post-synthesis HIR.

2. **Hierarchy preservation vs flattening**.
   HIR preserves hierarchy. Synthesis takes a flat HIR if `flatten=True`. Recommended default: preserve.

3. **Should records be flattened to bit-vectors in HIR or only at synthesis?**
   Recommendation: keep as records in HIR (they aid debug). Synthesis projects to bits when producing HNL.

4. **Functions with side effects**.
   VHDL allows `impure` functions; Verilog functions are pure. HIR represents both via `FunctionDecl.is_pure`. Synthesizer warns on impure functions.

5. **Time type representation**.
   VHDL `time` is a unit-bearing scalar; Verilog `time` is unsigned. Recommendation: `TyTime` is the abstract type; concrete unit is metadata.

6. **Mixed-language elaboration**.
   When a VHDL top instantiates a Verilog cell, both ASTs feed elaboration. Recommendation: `hdl-elaboration.md` orchestrates; HIR is language-agnostic.

7. **Backwriting (HIR → Verilog/VHDL)**.
   Provenance metadata determines the target language. If HIR has no original-language metadata (e.g., synthesized from Ruby DSL), `real-fpga-export.md` defaults to Verilog.

## Future Work

- AIG (And-Inverter Graph) sub-IR for ABC-style optimization.
- SystemVerilog (1800) extensions: interfaces, classes, assertions.
- VHDL-2019 features: interfaces, generics on packages.
- Mixed-signal nodes for AMS bridging to `spice-engine.md`.
- Streaming HIR readers for designs > 100K modules.
- Provenance-preserving optimization passes (so optimized HIR can still attribute back to source).
- HIR → MLIR bridge for advanced optimization on a more general IR substrate.
