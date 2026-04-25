# synthesis

HIR -> HNL synthesis. Takes a behavioral HIR (combinational continuous assignments) and produces a generic-cell-level HNL netlist of AND2/OR2/XOR2/NOT/etc. cells.

See [`code/specs/synthesis.md`](../../../specs/synthesis.md).

## Quick start

```python
from hdl_ir import HIR, Module, Port, Direction, ContAssign, BinaryOp, PortRef, Concat, TyVector, TyLogic
from synthesis import synthesize

# Build (or elaborate) an HIR
adder = Module(
    name="adder4",
    ports=[
        Port("a", Direction.IN, TyVector(TyLogic(), 4)),
        Port("b", Direction.IN, TyVector(TyLogic(), 4)),
        Port("cin", Direction.IN, TyLogic()),
        Port("sum", Direction.OUT, TyVector(TyLogic(), 4)),
        Port("cout", Direction.OUT, TyLogic()),
    ],
    cont_assigns=[
        ContAssign(
            target=Concat((PortRef("cout"), PortRef("sum"))),
            rhs=BinaryOp("+",
                BinaryOp("+", PortRef("a"), PortRef("b")),
                PortRef("cin"),
            ),
        ),
    ],
)
hir = HIR(top="adder4", modules={"adder4": adder})

# Synthesize
hnl = synthesize(hir)
print(hnl.stats())
# NetlistStats(cell_counts={'AND2': 8, 'OR2': 4, 'XOR2': 8, 'BUF': N, 'CONST_0': M, ...},
#              total_cells=~30, total_nets=~20)

hnl.to_json("adder4.hnl.json")
```

## v0.1.0 scope

- Combinational ContAssigns synthesized to generic gates.
- Operator lowering:
  - `&`/`AND` -> N parallel AND2 cells (bit-blasted).
  - `|`/`OR` -> N parallel OR2 cells.
  - `^`/`XOR` -> N parallel XOR2 cells.
  - `NAND`, `NOR` likewise.
  - `+` (addition) -> ripple-carry chain of full-adders (built from XOR2 + AND2 + OR2).
  - Unary NOT -> N parallel NOT cells.
  - Reduction AND/OR/XOR -> balanced tree.
- Concat lvalue handling: rhs split bit-wise across target parts.
- Constants -> CONST_0/CONST_1 cells.

## Out of scope (v0.2.0)

- Sequential always blocks (FF inference).
- FSM extraction.
- Subtraction, multiplication, division, shifts, comparison.
- Optimization passes (constant folding, dead-code elimination, common-subexpression).
- Latch inference and warning.

## Testing

```bash
pytest tests/
ruff check src/
```

MIT.
