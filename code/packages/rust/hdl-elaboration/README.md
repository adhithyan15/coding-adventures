# hdl-elaboration

Three-pass Verilog HDL elaborator that converts Verilog source into a
Hardware Intermediate Representation (HIR) suitable for simulation,
synthesis, and analysis.

## What it does

```
Verilog source → [parse] → GrammarASTNode → [elaborate] → Hir
```

The elaborator runs three passes:

| Pass | Name    | What it does |
|------|---------|--------------|
| 1    | Collect | Walk the AST, register every `module_declaration` |
| 2    | Bind    | Elaborate ports and continuous assignments per module |
| 3    | Unroll  | Determine top module, build final `Hir` |

### v0.1.0 scope

Structural Verilog (ANSI 2001 style):

- Module declarations with port lists
- Port direction: `input`, `output`, `inout`
- Port types: scalar (`1`-bit) and vector (`[msb:lsb]`)
- ANSI port direction inheritance (`input a, b` → both inputs)
- Continuous assignments (`assign lhs = rhs;`)
- Full expression grammar: binary, ternary, unary, primary, concatenation,
  replication

Behavioral constructs (`always`, `initial`, functions, tasks) are v0.2.0.

## Usage

```rust
use hdl_elaboration::elaborate_verilog;

let src = r#"
  module adder(input [3:0] a, input [3:0] b, output [4:0] sum);
    assign sum = a + b;
  endmodule
"#;

let hir = elaborate_verilog(src).unwrap();

// Top module
assert_eq!(hir.top, "adder");

// Ports
let m = &hir.modules["adder"];
assert_eq!(m.ports.len(), 3);

// Continuous assignments
assert_eq!(m.cont_assigns.len(), 1);
```

### Explicit top module

```rust
use hdl_elaboration::elaborate_verilog_with_top;

let src = "module a; endmodule module b; endmodule";
let hir = elaborate_verilog_with_top(src, "b").unwrap();
assert_eq!(hir.top, "b");
```

## How it fits in the stack

```
verilog-lexer          tokenize source
    ↓
verilog-parser         parse tokens → GrammarASTNode
    ↓
hdl-elaboration  ←─── you are here
    ↓
hdl-ir                 Hir, Module, Port, ContAssign, Expr
    ↓
hardware-vm            event-driven combinational simulator
    ↓
vcd-writer             IEEE 1364-2005 VCD waveform output
```

## Error handling

`ElaborationError` variants:

| Variant | When |
|---------|------|
| `ParseError(String)` | Source fails to lex or parse |
| `TopModuleNotFound(String)` | Named top module not in source |
| `InvalidModule(String)` | Module declaration is malformed |
| `InvalidPort(String)` | Port declaration is malformed |
| `InvalidExpr(String)` | Expression cannot be elaborated |

## License

MIT
