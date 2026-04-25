# hdl-ir — Hardware IR

The unified Hardware IR (HIR) for the silicon stack. Every HDL front-end (Verilog parser, VHDL parser, Ruby HDL DSL) elaborates *into* HIR, and every back-end (simulation VM, synthesis, FPGA mapper, ASIC tools) consumes HIR. Without it, every back-end would have to handle three source languages independently. With it, the M×N integration matrix collapses to M+N.

This is the **first implementation layer below the Verilog/VHDL parsers**. It defines the data structures and JSON round-trip; downstream packages (`hdl-elaboration`, `hardware-vm`, `synthesis`) build on top.

See [`code/specs/hdl-ir.md`](../../../specs/hdl-ir.md) for the full design specification, including the IEEE 1364-2005 + 1076-2008 conformance matrix and worked examples.

## Quick start

```python
from hdl_ir import (
    HIR, Module, Port, Direction,
    TyVector, TyLogic,
    ContAssign, BinaryOp, PortRef, Concat,
    Provenance, SourceLang,
)

# A 4-bit adder in HIR
adder4 = Module(
    name="adder4",
    ports=[
        Port("a",   Direction.IN,  TyVector(TyLogic(), 4)),
        Port("b",   Direction.IN,  TyVector(TyLogic(), 4)),
        Port("cin", Direction.IN,  TyLogic()),
        Port("sum", Direction.OUT, TyVector(TyLogic(), 4)),
        Port("cout", Direction.OUT, TyLogic()),
    ],
    cont_assigns=[
        ContAssign(
            target=Concat((PortRef("cout"), PortRef("sum"))),
            rhs=BinaryOp("+", BinaryOp("+", PortRef("a"), PortRef("b")), PortRef("cin")),
        )
    ],
)

hir = HIR(top="adder4", modules={"adder4": adder4})

# Validate it
report = hir.validate()
assert report.ok

# Serialize to JSON
hir.to_json("adder4.json")

# Round-trip
hir2 = HIR.from_json("adder4.json")
assert hir == hir2
```

## What's in this package

| Module | What |
|---|---|
| `hdl_ir.types` | The HIR type system: `TyLogic`, `TyBit`, `TyStdLogic`, `TyVector`, `TyInteger`, `TyEnum`, `TyRecord`, `TyArray`, `TyFile`, `TyTime`, `TyReal`, `TyString` |
| `hdl_ir.expr` | Expressions: `Lit`, `NetRef`, `PortRef`, `VarRef`, `Slice`, `Concat`, `Replication`, `UnaryOp`, `BinaryOp`, `Ternary`, `FunCall`, `SystemCall`, `Attribute` |
| `hdl_ir.stmt` | Statements: `BlockingAssign`, `NonblockingAssign`, `IfStmt`, `CaseStmt`, `ForStmt`, `WhileStmt`, `RepeatStmt`, `ForeverStmt`, `WaitStmt`, `DelayStmt`, `EventStmt`, `AssertStmt`, `ReportStmt`, `DisableStmt`, `ReturnStmt`, `NullStmt`, `ExprStmt` |
| `hdl_ir.module` | Module-level: `Net`, `Variable`, `Port`, `Direction`, `Process`, `ContAssign`, `Instance`, `Module`, `Parameter`, `Library` |
| `hdl_ir.hir` | Top-level `HIR` container with JSON round-trip |
| `hdl_ir.validate` | Validation rules H1–H20 |
| `hdl_ir.provenance` | Source-language provenance tracking |

## Spec compliance

All node types match the design in `code/specs/hdl-ir.md`. The JSON schema is frozen at v0.1.0 — future versions add nodes additively without breaking existing files.

## Testing

```bash
pytest tests/                  # all tests
pytest --cov=hdl_ir            # with coverage (target ≥ 85%)
ruff check src/                # lint
mypy src/                      # type-check (strict)
```

## License

MIT (matching the parent project).
