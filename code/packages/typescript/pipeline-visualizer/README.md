# Pipeline Visualizer

**Language-agnostic HTML visualization generator** for the computing stack.

## What this package does

Reads JSON conforming to the **PipelineReport** contract and generates self-contained HTML files. Any language that can produce the JSON (Python, Ruby, TypeScript, etc.) can use this renderer — it has no dependency on other coding-adventures packages.

## Where it fits

```
Logic Gates -> Arithmetic -> CPU -> ARM -> Assembler -> Lexer -> Parser -> Compiler -> VM
                                                                                       |
                                                                                Pipeline Runner
                                                                                       |
                                                                                JSON Report
                                                                                       |
                                                                             +---------+---------+
                                                                             |                   |
                                                                        Python impl        Ruby / TS impl
                                                                             |                   |
                                                                             +---------+---------+
                                                                                       |
                                                                              [HTML Renderer]
                                                                                       |
                                                                              Static HTML file
```

The renderer is **pluggable**: any language can emit the PipelineReport JSON, and this single renderer turns it into a visual HTML page.

## Installation

```bash
npm install @coding-adventures/html-renderer
```

## Usage

```typescript
import { HTMLRenderer } from "@coding-adventures/html-renderer";

const renderer = new HTMLRenderer();

// From a JSON file
const html = renderer.renderFromJson("pipeline-report.json");

// From a TypeScript object
const report = {
  source: "x = 1 + 2",
  language: "python",
  target: "vm",
  metadata: {
    generated_at: "2026-03-18T12:00:00Z",
    generator_version: "0.1.0",
    packages: { lexer: "0.1.0", parser: "0.1.0" },
  },
  stages: [
    /* ... */
  ],
};
const htmlString = renderer.render(report);

// Directly to a file
renderer.renderToFile(report, "report.html");
```

## Supported Stage Types

| Stage Name | Display | Visualization |
|-----------|---------|---------------|
| `lexer` | Token stream | Colored token badges |
| `parser` | AST | SVG tree diagram |
| `compiler` | Bytecode | Instruction table with stack effects |
| `vm` | VM execution | Step-by-step trace with stack visualization |
| `assembler` | Assembly | Instruction listing with binary encoding |
| `riscv` | RISC-V execution | Register state trace with change highlights |
| `arm` | ARM execution | Register state trace with change highlights |
| `alu` | ALU operations | Bit-level arithmetic with CPU flags |
| `gates` | Gate operations | Gate-level circuit trace |

Unknown stage types are rendered as formatted JSON (fallback).

## Architecture

The renderer uses a **dispatch table** pattern:

```
PipelineReport.stages[i]
       |
       +-- stage.name = "lexer"     -> renderLexerStage()
       +-- stage.name = "parser"    -> renderParserStage()
       +-- stage.name = "compiler"  -> renderCompilerStage()
       +-- stage.name = "vm"        -> renderVMStage()
       +-- stage.name = "assembler" -> renderAssemblerStage()
       +-- stage.name = "riscv"     -> renderHardwareExecutionStage()
       +-- stage.name = "arm"       -> renderHardwareExecutionStage()
       +-- stage.name = "alu"       -> renderALUStage()
       +-- stage.name = "gates"     -> renderGateStage()
       +-- (unknown)                -> renderFallbackStage()
```

Generated HTML is **self-contained**: CSS is embedded in a `<style>` tag, SVG graphics are inline, and there are no external dependencies. You can email the file, put it on a USB drive, or open it offline.

## Spec

See [11-html-visualizer.md](../../../specs/11-html-visualizer.md) for the full specification.
