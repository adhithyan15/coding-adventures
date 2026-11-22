# coding_adventures_verilog_parser

Pure Dart grammar-driven parser for Verilog HDL (IEEE 1364-1995, IEEE 1364-2001, and IEEE 1364-2005).

## Overview

`coding_adventures_verilog_parser` provides recursive descent grammar parsing for Verilog source text into an Abstract Syntax Tree (AST), supporting multiple standard editions.

### Key Capabilities

- **Multi-Edition Grammar Support**: Precompiled grammar specifications for IEEE 1364-1995, 2001, and 2005.
- **Combined Nesting Depth Defense**: Combined limit (64) across delimiters (`()`, `[]`, `{}`), structural block keywords (`begin`/`end`, `case`/`endcase`, `module`/`endmodule`, `function`/`endfunction`, `task`/`endtask`, `generate`/`endgenerate`), and unary prefix operator chains.
- **Token Ceiling Protection**: Bounded token stream evaluation (`maxTokenCount = 65,536`).
- **Standing Rule #20 Compliance**: 100% centralized localizable diagnostic catalog in `messages.dart`, sealed exceptions with closed constructors, and strict payload-blindness.

---

## 5-Class Security Resilience Showcase

| Class | Vulnerability Vector | Defense Mechanism | Invariant Guaranteed |
| :--- | :--- | :--- | :--- |
| **Class 1: Bounded Memory & Input Envelope** | Denial of Service via unbounded token streams (CWE-400) | Enforces `maxTokenCount = 65,536` before parser tree evaluation | Prevents heap exhaustion from malicious token streams |
| **Class 2: Uncontrolled Recursion & Stack Exhaustion** | Stack overflow via deeply nested delimiters, block structures, or operator chains (CWE-674) | Single combined nesting depth counter (`maxNestingDepth = 64`) tracking delimiters, blocks, and prefix operators with VM stack catch backstop | Guarantees bounded call depth within safe VM stack limits |
| **Class 3: AST Projection & Node Integrity** | Incomplete AST structure or corrupted parent-child relations | Complete grammar validation against canonical AST fixture projections | Guarantees deterministic AST node hierarchy |
| **Class 4: Closed Diagnostic Catalog & Payload Blindness** | Sensitive source code exfiltration via error messages (CWE-209) | Sealed `VerilogParserException` with closed constructors and static messages from `messages.dart` | Eliminates payload echoing in error messages |
| **Class 5: Adversarial Grammar & Fuzz Resilience** | Parser stalls or uncontrolled failure modes on invalid inputs | Bounded grammar-driven backtracking parser resilient against mutated adversarial streams | Predictable syntax error detection without infinite loops |

---

## Usage

```dart
import 'package:coding_adventures_verilog_parser/verilog_parser.dart';

void main() {
  const source = '''
module counter(input wire clk, output reg [7:0] q);
  always @(posedge clk) begin
    q <= q + 1;
  end
endmodule
''';

  final ast = parseVerilog(source);
  print('Parsed module: ${ast.ruleName}');
}
```

---

## Verification & Testing

Validate formatting, static analysis, conformance, and test coverage:

```bash
sh BUILD
```
