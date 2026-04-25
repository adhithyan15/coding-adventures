# hdl-elaboration

Bridges parser AST to HIR. Takes Verilog (or VHDL or Ruby DSL traces) AST output, produces fully-resolved HIR ready for downstream consumers (`hardware-vm`, `synthesis`, etc.).

This is the *second* implementation layer beneath the Verilog/VHDL parsers (the first is `hdl-ir`).

See [`code/specs/hdl-elaboration.md`](../../../specs/hdl-elaboration.md) for the design.

## Quick start

```python
from hdl_elaboration import elaborate_verilog

source = """
module adder4(input [3:0] a, b, input cin,
              output [3:0] sum, output cout);
  assign {cout, sum} = a + b + cin;
endmodule
"""

hir = elaborate_verilog(source, top="adder4")

print(hir.modules["adder4"].name)               # adder4
print(len(hir.modules["adder4"].ports))         # 5
print(hir.modules["adder4"].cont_assigns[0])    # ContAssign(...)

# It validates clean.
report = hir.validate()
assert report.ok
```

## What's in v0.1.0

- **Verilog elaboration** for the synthesizable subset relevant to the canonical 4-bit adder smoke test:
  - Modules with parameters, ports (input/output/inout, with bit-vector ranges).
  - Continuous assignments.
  - Expressions: literals, name references, binary ops (`+`, `-`, `&`, `|`, `^`), concatenation `{...}`, slices `x[h:l]`, parentheses.
  - Hierarchical instances with parameter binding and port connections.
- **Three-pass design** per `hdl-elaboration.md`:
  - Pass 1 (Collect): build symbol table from input ASTs.
  - Pass 2 (Bind): name resolution + parameter binding + type tagging.
  - Pass 3 (Unroll): generate-for / for-loop unrolling.
- **Provenance**: every HIR node carries `Provenance(SourceLang.VERILOG, source_location)` so downstream diagnostics resolve back to source.

## Out of scope (v0.1.0; planned for 0.2.0)

- VHDL elaboration (the existing `vhdl-parser` is supported via the same framework, but this release focuses on Verilog).
- Behavioral processes (`always @(...) begin ... end`) — combinational logic only for now.
- Generate-for unrolling beyond simple cases.
- Configurations.
- Mixed-language designs.

## Testing

```bash
pytest tests/                  # all tests
ruff check src/                # lint
```

## License

MIT.
