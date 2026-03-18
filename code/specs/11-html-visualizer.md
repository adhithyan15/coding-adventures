# 11 — HTML Visualizer

## Overview

The HTML visualizer generates a self-contained HTML file that shows every stage of the computing stack — from source code to gate operations. It opens in any browser, requires no server, and can be shared with anyone.

The architecture is split into two parts:
1. **JSON data contract** — a standardized format that any language implementation (Python, Ruby, TypeScript) can produce
2. **HTML renderer** — reads the JSON and generates beautiful, static HTML with embedded CSS and SVG

This design means the renderer never needs to change when a new language implementation is added. It only reads JSON.

## Architecture: Pluggability

```
Python packages ──→ PipelineReport (JSON) ──┐
Ruby packages ────→ PipelineReport (JSON) ──┼──→ HTML Renderer ──→ report.html
TypeScript pkgs ──→ PipelineReport (JSON) ──┘
```

The **JSON data contract** is the boundary. Any language that can produce a JSON file conforming to the contract can generate the same HTML visualization.

## JSON Data Contract

### PipelineReport (root object)

```json
{
  "source": "x = 1 + 2",
  "language": "python",
  "target": "vm",
  "metadata": {
    "generated_at": "2026-03-18T12:00:00Z",
    "generator_version": "0.1.0",
    "packages": {
      "lexer": "0.1.0",
      "parser": "0.1.0"
    }
  },
  "stages": [ ... ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| source | string | The source code that was compiled |
| language | string | Which language implementation produced this ("python", "ruby", "typescript") |
| target | string | Execution target ("vm", "riscv", "arm") |
| metadata | object | Timestamp, versions, any extra context |
| stages | array | Ordered list of StageReport objects |

### StageReport (one per pipeline stage)

```json
{
  "name": "lexer",
  "display_name": "Tokenization",
  "input_repr": "x = 1 + 2",
  "output_repr": "6 tokens",
  "duration_ms": 0.12,
  "data": { ... }
}
```

| Field | Type | Description |
|-------|------|-------------|
| name | string | Machine identifier (used for rendering dispatch) |
| display_name | string | Human-readable title for the HTML section |
| input_repr | string | Short description of what this stage received |
| output_repr | string | Short description of what this stage produced |
| duration_ms | float | How long this stage took |
| data | object | Stage-specific structured data (see below) |

### Stage-specific data schemas

#### Lexer (`name: "lexer"`)
```json
{
  "tokens": [
    {"type": "NAME", "value": "x", "line": 1, "column": 1},
    {"type": "EQUALS", "value": "=", "line": 1, "column": 3},
    {"type": "NUMBER", "value": "1", "line": 1, "column": 5},
    {"type": "PLUS", "value": "+", "line": 1, "column": 7},
    {"type": "NUMBER", "value": "2", "line": 1, "column": 9},
    {"type": "EOF", "value": "", "line": 1, "column": 10}
  ]
}
```

#### Parser (`name: "parser"`)
```json
{
  "ast": {
    "type": "Assignment",
    "children": [
      {"type": "Name", "value": "x", "children": []},
      {"type": "BinaryOp", "value": "+", "children": [
        {"type": "Number", "value": "1", "children": []},
        {"type": "Number", "value": "2", "children": []}
      ]}
    ]
  }
}
```

The AST is a recursive tree of nodes. Each node has `type`, optional `value`, and `children`. The renderer draws this as an SVG tree diagram.

#### Bytecode Compiler (`name: "compiler"`)
```json
{
  "instructions": [
    {"index": 0, "opcode": "LOAD_CONST", "arg": "1", "stack_effect": "→ 1"},
    {"index": 1, "opcode": "LOAD_CONST", "arg": "2", "stack_effect": "→ 2"},
    {"index": 2, "opcode": "ADD", "arg": null, "stack_effect": "1, 2 → 3"},
    {"index": 3, "opcode": "STORE_NAME", "arg": "x", "stack_effect": "3 →"},
    {"index": 4, "opcode": "HALT", "arg": null, "stack_effect": ""}
  ],
  "constants": [1, 2],
  "names": ["x"]
}
```

#### VM Execution (`name: "vm"`)
```json
{
  "steps": [
    {"index": 0, "instruction": "LOAD_CONST 1", "stack_before": [], "stack_after": [1], "variables": {}},
    {"index": 1, "instruction": "LOAD_CONST 2", "stack_before": [1], "stack_after": [1, 2], "variables": {}},
    {"index": 2, "instruction": "ADD", "stack_before": [1, 2], "stack_after": [3], "variables": {}},
    {"index": 3, "instruction": "STORE_NAME x", "stack_before": [3], "stack_after": [], "variables": {"x": 3}},
    {"index": 4, "instruction": "HALT", "stack_before": [], "stack_after": [], "variables": {"x": 3}}
  ]
}
```

#### Assembler (`name: "assembler"`)
```json
{
  "lines": [
    {"address": 0, "assembly": "addi x1, x0, 1", "binary": "0x00100093", "encoding": {"imm": "000000000001", "rs1": "00000", "funct3": "000", "rd": "00001", "opcode": "0010011"}},
    {"address": 4, "assembly": "addi x2, x0, 2", "binary": "0x00200113", "encoding": {"imm": "000000000010", "rs1": "00000", "funct3": "000", "rd": "00010", "opcode": "0010011"}},
    {"address": 8, "assembly": "add x3, x1, x2", "binary": "0x002081B3", "encoding": {"funct7": "0000000", "rs2": "00010", "rs1": "00001", "funct3": "000", "rd": "00011", "opcode": "0110011"}},
    {"address": 12, "assembly": "ecall", "binary": "0x00000073", "encoding": {}}
  ]
}
```

The `encoding` field allows the renderer to show a color-coded binary breakdown of each instruction.

#### RISC-V / ARM Execution (`name: "riscv"` or `name: "arm"`)
```json
{
  "steps": [
    {"address": 0, "instruction": "addi x1, x0, 1", "registers_changed": {"x1": 1}, "registers": {"x0": 0, "x1": 1, "x2": 0, "x3": 0}},
    {"address": 4, "instruction": "addi x2, x0, 2", "registers_changed": {"x2": 2}, "registers": {"x0": 0, "x1": 1, "x2": 2, "x3": 0}},
    {"address": 8, "instruction": "add x3, x1, x2", "registers_changed": {"x3": 3}, "registers": {"x0": 0, "x1": 1, "x2": 2, "x3": 3}},
    {"address": 12, "instruction": "ecall", "registers_changed": {}, "registers": {"x0": 0, "x1": 1, "x2": 2, "x3": 3}}
  ]
}
```

#### ALU Operations (`name: "alu"`)
```json
{
  "operations": [
    {"op": "ADD", "a": 1, "b": 2, "result": 3, "bits_a": "00000001", "bits_b": "00000010", "bits_result": "00000011", "flags": {"zero": false, "carry": false, "negative": false, "overflow": false}}
  ]
}
```

#### Gate Operations (`name: "gates"`)
```json
{
  "operations": [
    {"description": "Full adder bit 0", "gates": [
      {"gate": "XOR", "inputs": [1, 0], "output": 1, "label": "A0 XOR B0"},
      {"gate": "AND", "inputs": [1, 0], "output": 0, "label": "A0 AND B0"},
      {"gate": "XOR", "inputs": [1, 0], "output": 1, "label": "Sum0 = partial XOR carry_in"},
      {"gate": "AND", "inputs": [1, 0], "output": 0, "label": "partial AND carry_in"},
      {"gate": "OR", "inputs": [0, 0], "output": 0, "label": "Carry0"}
    ]},
    {"description": "Full adder bit 1", "gates": [...]}
  ]
}
```

## HTML Output Structure

The generated HTML file has these sections, top to bottom:

```html
<header>
  <h1>Computing Stack Report</h1>
  <code class="source">x = 1 + 2</code>
  <p>Implementation: Python | Target: VM | Generated: 2026-03-18</p>
</header>

<section id="tokens">         <!-- Color-coded token list -->
<section id="ast">             <!-- SVG tree diagram -->
<section id="bytecode">        <!-- Instruction table with stack effects -->
<section id="vm-execution">    <!-- Step-by-step execution table -->
<section id="assembly">        <!-- Assembly listing with binary breakdown -->
<section id="machine-code">    <!-- Color-coded bit field diagrams -->
<section id="hw-execution">    <!-- Register state per instruction -->
<section id="alu">             <!-- ALU operation details -->
<section id="gates">           <!-- Gate-level trace -->
<footer>
  <p>Generated by coding-adventures HTML Visualizer v0.1.0</p>
</footer>
```

Each section only appears if the corresponding stage exists in the report. This means:
- VM-only reports show: tokens → AST → bytecode → VM execution
- RISC-V reports show: tokens → AST → assembly → machine code → RISC-V execution → ALU → gates

## Visual Design

- **Color scheme:** Dark background, syntax-highlighting colors (like a code editor)
- **Tokens:** Each token is a colored badge (NAME=blue, NUMBER=green, OP=red, etc.)
- **AST:** SVG tree with rounded rectangles for nodes, lines connecting parent to children
- **Bytecode/Assembly:** Monospace table with alternating row colors
- **Execution tables:** Stack states shown as vertical stacks of boxes
- **Binary encoding:** Each bit field in a different color with labels above
- **Gates:** Circuit-style layout with inputs on left, gate symbol, output on right

## Public API

```python
# In the html-renderer package
class HTMLRenderer:
    def __init__(self, theme: str = "dark") -> None: ...

    def render(self, report: PipelineReport) -> str: ...
        # Returns complete HTML string

    def render_from_json(self, json_path: str) -> str: ...
        # Load JSON file, render to HTML string

    def render_to_file(self, report: PipelineReport, output_path: str) -> None: ...
        # Render and write to file

# In the pipeline-visualizer program
def main():
    report = Pipeline.run("x = 1 + 2", target="vm")
    renderer = HTMLRenderer()
    renderer.render_to_file(report, "pipeline-report.html")
```

## Test Strategy

- Render a report with all stages: verify valid HTML (no unclosed tags)
- Render a VM-only report: verify hardware sections are absent
- Render a RISC-V report: verify VM section is absent, hardware sections present
- Verify JSON round-trip: PipelineReport → JSON → load → render produces same HTML
- Verify SVG tree generation for known ASTs
- Test with edge cases: empty source, single token, deeply nested AST
- Visual inspection: open generated HTML in browser, verify it looks correct

## Future Extensions

- **Themes:** Light mode, high contrast, print-friendly
- **Multiple programs:** Render several programs in one report for comparison
- **Diff mode:** Two implementations side by side (Python vs Ruby)
- **PDF export:** From the HTML (browser's print-to-PDF works)
- **Animations:** Optional lightweight JS for step-through animation
