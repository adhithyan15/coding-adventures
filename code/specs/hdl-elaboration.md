# HDL Elaboration

## Overview

Elaboration is the bridge between *what the user wrote* (parser AST or DSL trace) and *what the rest of the stack consumes* (HIR — see `hdl-ir.md`). Three transformations happen:

1. **Name resolution** — every `NAME` reference (a port, a signal, a function call) is bound to its definition.
2. **Type binding** — every expression's type is computed and checked.
3. **Generate-style unrolling** — `generate-for`, `generate-if`, `generate-case`, parameterized instantiations are expanded into concrete instances.

After elaboration, HIR is *fully resolved*: no unbound references, no unbound generics, no symbolic types, no `for-generate` placeholders. Synthesis, simulation, and downstream tools see a complete tree where every node knows everything it needs to know.

The elaborator handles **three input front-ends** uniformly:
- Verilog AST (from `verilog-parser` per `F05-verilog-vhdl.md` + `f05-full-ieee-extensions.md`).
- VHDL AST (from `vhdl-parser` similarly).
- Ruby DSL traces (from `ruby-hdl-dsl.md`).

Each has its own elaboration sub-pass; they all merge into a single elaborated HIR tree at the end.

### Generality

Elaboration scales linearly in design size for typical inputs. A 4-bit adder elaborates in microseconds; a 32-bit ALU in ~1 ms; a small RISC-V core in ~10 ms; a 100K-instance design in seconds. The bottleneck for large designs is generate-loop unrolling and parameter-driven module specialization, both of which are addressed by a memoizing module-instance cache.

## Layer Position

```
Verilog AST    VHDL AST     Ruby DSL trace
     │             │              │
     └─────────────┼──────────────┘
                   ▼
        ┌─────────────────────────┐
        │ hdl-elaboration.md       │  ◀── THIS SPEC
        │  (3-pass: collect,       │
        │   bind, unroll)           │
        └─────────────────────────┘
                   │
                   ▼
                  HIR (fully resolved)
                   │
                   ▼
        hardware-vm | synthesis | etc.
```

## Concepts

### Three-pass design

Elaboration is structured as three explicit passes. Each pass walks the input AST/trace once.

**Pass 1 — Collect.** Walk every input file and record:
- Every entity/module declaration.
- Every architecture/body.
- Every package and its contents (functions, types, constants).
- Every component declaration.

This produces a **symbol table** that maps `library.name` to a definition. No bindings happen yet; we just know what exists.

**Pass 2 — Bind.** For each module to be elaborated:
- Bind generic / parameter values from instantiation site.
- Resolve type references to actual types.
- Resolve component instantiations to entity/module references.
- Resolve every name reference in expressions and statements to its scope-correct definition.
- Type-check every expression.

**Pass 3 — Unroll.** Expand `generate` blocks:
- `for-generate`: emit one set of instances per loop iteration.
- `if-generate` / `case-generate`: emit instances from the matching branch only.
- Parameterized modules: specialize per (module, parameter-binding) pair using a memoizing cache.

After Pass 3, the HIR has no unresolved generates and no unbound parameters — every instance refers to a fully-specialized module.

### Symbol table & scopes

Scopes form a tree:
```
Library scope
└── Package scope
    ├── Type / function / constant / signal definitions
    └── Architecture scope (when entity is being elaborated)
        ├── Architecture-local declarations
        ├── Component declarations
        ├── Generate-block scope
        │   └── (per-iteration scope; vars from genvar bound)
        ├── Process scope
        │   ├── Process-local variables
        │   ├── Block scope (begin/end with optional name)
        │   └── ...
        └── Function/procedure scope (when called)
```

Lookup walks outward from the innermost scope. Verilog has fewer scope kinds but the same principle.

### Parameter / generic binding

When a module is instantiated:

```verilog
adder #(.WIDTH(8)) u1 (.a(x), .b(y), .sum(s));
```

```vhdl
u1: adder generic map (WIDTH => 8) port map (a => x, b => y, sum => s);
```

The elaborator:
1. Looks up the `adder` module's parameter list.
2. Binds `WIDTH` to the literal `8` (at instantiation time, not at definition time).
3. Specializes the module: every type that depends on `WIDTH` (e.g., `[WIDTH-1:0]`) gets concrete dimensions.
4. Caches the specialized module by `(adder, WIDTH=8)` so the next instantiation with the same binding reuses it.

This is the same idea as C++ template instantiation. Two instances of `adder #(.WIDTH(8))` in the same design share one specialized module; an instance of `adder #(.WIDTH(16))` is a separate specialization.

### Generate-loop unrolling

Verilog:
```verilog
genvar i;
generate
  for (i = 0; i < N; i = i + 1) begin: bit
    full_adder fa (.a(a[i]), .b(b[i]), .cin(c[i]), .sum(s[i]), .cout(c[i+1]));
  end
endgenerate
```

The elaborator:
1. Evaluates the loop bounds (`N` is bound by parameter).
2. For each iteration value of `i`, emits one `Instance` HIR node with `i` substituted into all expressions.
3. The generated instances live in a hierarchical scope named `bit[0]`, `bit[1]`, ..., `bit[N-1]`.

VHDL `for-generate` works the same way; `if-generate` keeps only the matching branch; `case-generate` similarly.

### Type checking

The elaborator computes a type for every expression and verifies:
- Operator operands have compatible types.
- Assignment targets and source types match (or are compatible per language rules — Verilog is permissive, VHDL is strict).
- Function/procedure calls match their declared signatures.
- Port-connection types match.

Type errors are diagnostics with full source locations, traceable through provenance metadata.

### Default values, initialization

If a port or signal has a default initialization expression, the elaborator binds and type-checks it at definition time. Initial values may not depend on signals (only constants and parameters).

### Multi-language elaboration

A VHDL top can instantiate a Verilog component (and vice versa). The elaborator handles this by:
1. Both ASTs feed Pass 1 (symbol table merges across languages — same library/name in both languages is an error unless one is a foreign-language declaration).
2. The component's port list and types must be compatible across the language boundary.
3. After elaboration, the HIR represents the design seamlessly; per-node provenance preserves which language each part came from.

### What the elaborator does NOT do

- Static synthesis (e.g., constant folding beyond what's needed for parameter binding) — that's `synthesis.md`.
- Optimization — synthesis.
- Layout, timing, anything else — downstream specs.

## Public API

```python
from dataclasses import dataclass, field
from typing import Protocol


class FrontEnd(Protocol):
    """Common interface for the three front-ends."""
    def collect(self, source: object, symtab: "SymbolTable") -> None: ...
    def bind(self, module_name: str, symtab: "SymbolTable") -> "Module": ...


@dataclass
class SymbolTable:
    libraries: dict[str, "Library"] = field(default_factory=dict)
    packages: dict[str, "Package"] = field(default_factory=dict)
    modules: dict[str, "ModuleDecl"] = field(default_factory=dict)


@dataclass
class Elaborator:
    """Three-pass elaboration."""
    verilog_frontend: FrontEnd
    vhdl_frontend: FrontEnd
    rubydsl_frontend: FrontEnd

    def elaborate(
        self,
        verilog_asts: list = (),
        vhdl_asts: list = (),
        ruby_traces: list = (),
        top: str = "",
    ) -> "HIR":
        symtab = SymbolTable()

        # Pass 1: Collect across all front-ends
        for ast in verilog_asts: self.verilog_frontend.collect(ast, symtab)
        for ast in vhdl_asts:    self.vhdl_frontend.collect(ast, symtab)
        for tr  in ruby_traces:  self.rubydsl_frontend.collect(tr,  symtab)

        # Pass 2: Bind starting from `top`, recursing through instances
        bound: dict[str, "Module"] = {}
        self._bind_module(top, {}, symtab, bound)

        # Pass 3: Unroll (folded into bind via lazy instantiation)

        return HIR(top=top, modules=bound)

    def _bind_module(
        self, name: str, params: dict, symtab: SymbolTable,
        bound: dict[str, "Module"]
    ) -> None:
        spec_key = self._specialization_key(name, params)
        if spec_key in bound:
            return  # memoized

        decl = symtab.modules[name]
        ctx = BindContext(symtab=symtab, params=params)
        module = ctx.bind(decl)
        bound[spec_key] = module

        for inst in module.instances:
            inst_params = self._resolve_params(inst.parameters, ctx)
            self._bind_module(inst.module, inst_params, symtab, bound)


@dataclass
class BindContext:
    symtab: SymbolTable
    params: dict[str, "Expr"]
    scopes: list[dict[str, object]] = field(default_factory=list)

    def lookup(self, name: str) -> object:
        for scope in reversed(self.scopes):
            if name in scope:
                return scope[name]
        if name in self.params:
            return self.params[name]
        if name in self.symtab.modules:
            return self.symtab.modules[name]
        raise NameError(f"undefined: {name}")
```

## Worked Examples

### Example 1 — Parameterized adder

```verilog
module adder #(parameter N=4) (input [N-1:0] a, b, output [N:0] sum);
  assign sum = a + b;
endmodule

module top;
  wire [3:0] x4, y4;
  wire [4:0] s4;
  adder #(.N(4)) u4 (.a(x4), .b(y4), .sum(s4));
  
  wire [7:0] x8, y8;
  wire [8:0] s8;
  adder #(.N(8)) u8 (.a(x8), .b(y8), .sum(s8));
endmodule
```

After elaboration:
- `top` is in HIR as one Module.
- Two specializations of `adder` exist: `adder<N=4>` and `adder<N=8>`. Each is a Module with concrete bit widths in its ports.
- `top.instances` contains two Instance nodes pointing at the two specializations.

### Example 2 — Generate-for unrolling

```verilog
module ripple_adder #(parameter N=8) (
  input [N-1:0] a, b, input cin,
  output [N-1:0] sum, output cout
);
  wire [N:0] c;
  assign c[0] = cin;
  assign cout = c[N];
  
  genvar i;
  generate
    for (i = 0; i < N; i = i + 1) begin: bit
      full_adder fa (.a(a[i]), .b(b[i]), .cin(c[i]),
                     .sum(sum[i]), .cout(c[i+1]));
    end
  endgenerate
endmodule
```

After elaboration with N=8: 8 `Instance` nodes named `bit[0].fa`, `bit[1].fa`, ..., `bit[7].fa`, each connected to bit-i of `a`, `b`, `sum` and `c[i]`, `c[i+1]`.

### Example 3 — Mixed-language instantiation

VHDL top `top.vhd` instantiates Verilog component `adder.v`:

```vhdl
architecture rtl of top is
  component adder
    generic (N : integer := 4);
    port (a, b : in std_logic_vector(N-1 downto 0);
          sum  : out std_logic_vector(N downto 0));
  end component;
begin
  u: adder generic map (N => 8) port map (a => x, b => y, sum => s);
end architecture;
```

The elaborator:
1. Collect pass picks up `adder` (Verilog) and `top` (VHDL).
2. Bind pass hits `top.u: adder` instantiation.
3. Looks up `adder` in symtab; finds the Verilog declaration.
4. Verifies port types match (Verilog `[N-1:0]` ↔ VHDL `std_logic_vector(N-1 downto 0)` — compatible).
5. Specializes Verilog `adder` with N=8.
6. The HIR instance points at the specialized Verilog adder module.

## VHDL-Specific Wrinkles

- **Configurations**: a `configuration` declaration overrides default component-binding; the elaborator consults it to choose which architecture of `adder` (e.g., `adder(behavioral)` vs `adder(structural)`) to instantiate.
- **Default binding**: if no configuration, VHDL's default-binding rules apply — same library, same name, last architecture analyzed.
- **Direct entity instantiation**: VHDL-93 onward allows `u: entity work.adder(rtl) port map ...`, bypassing component declarations.
- **Generics with discrete or scalar types**: parameter values can be integers, booleans, enums.
- **Resolution functions** for `std_logic`: when multiple drivers exist on a `std_logic` signal, the resolution function from `ieee.std_logic_1164` combines them. The elaborator records the resolution function name in `Net.attributes` for the simulator.

## Verilog-Specific Wrinkles

- **`defparam`**: deprecated; the elaborator handles it but warns.
- **Hierarchical references**: `top.u1.x` is bound at elaboration; recorded as a hierarchical NetRef in HIR.
- **Implicit nets** (Verilog default): if a signal is referenced without declaration, an implicit `wire` is created. (`` `default_nettype none `` disables this.)
- **Interface arrays** (Verilog 2001 `[3:0]` on instance names): unrolled to N separate instances.

## Edge Cases

| Scenario | Handling |
|---|---|
| Cyclic module instantiation (M instantiates M) | Detected at bind; rejected (HIR rule H20). |
| Recursive parameter expressions (parameter X = X + 1) | Detected; rejected. |
| Forward reference to module declared later in file | Allowed; collect pass runs to completion before bind. |
| Parameter binding to unevaluated expression | Evaluate at bind time; if not a constant, error. |
| Generic without default and not bound at instantiation | Error. |
| Generate-loop with non-constant bounds | Error (loop bounds must be parameter or literal). |
| Generate-loop with very large bounds (N=10000) | Allowed; emit warning if > 1000. |
| Library + use clause referring to non-existent library | Error. |
| Multiple architectures of one entity, no configuration | Use last-analyzed (VHDL rule); warn. |
| Component declaration mismatch with entity | Error. |
| Default-port binding (Verilog `.x()` empty) | Bind to "Z" or "open"; HIR records as such. |
| VHDL `open` association on output | Allowed; output is unconnected; warn if used elsewhere. |
| Type mismatch on port (Verilog wire to VHDL std_logic) | Compatible if widths match; warn on potential signedness issue. |

## Test Strategy

### Unit (target 95%+)
- Symbol-table collection: every module/entity/package found.
- Name resolution: lookup correctly walks scope tree.
- Parameter binding: literal, expression, and parameter-of-parameter cases.
- Type-check: positive + negative.
- Generate-for unroll: N iterations produce N instances.
- Generate-if and generate-case branch selection.
- Memoization: same `(module, params)` hashed instantiation.

### Integration
- Adder, ALU, FSM, RISC-V toy core — full elaboration pipeline.
- Mixed-language design (VHDL top + Verilog leaves).
- Re-elaboration after parameter change produces correct delta.
- Real-world testbench (300+ lines) elaborates without warnings.

### Property
- Idempotence: elaborating a design twice produces identical HIR.
- Determinism: parallel-elaboration of independent sub-trees produces identical results.
- Round-trip: HIR → emit-Verilog/VHDL → re-elaborate → equivalent HIR.

## Open Questions

1. **Generate unroll vs HIR generate node**: should HIR retain `Generate` blocks for debugging? Recommendation: optional flag; default unroll for downstream simplicity.
2. **Lazy elaboration**: only elaborate modules reachable from `top`? Recommendation: yes; tree-shake unused modules.
3. **Pre-elaboration constant evaluation**: how aggressive? Recommendation: only what's needed to bind parameters; let synthesis do the rest.

## Future Work

- Incremental re-elaboration (only re-elab parts that changed).
- Parallel elaboration of independent module specializations.
- Cross-package dependency tracking for IDE jump-to-definition.
- Configuration UI for selecting alternate architectures interactively.
