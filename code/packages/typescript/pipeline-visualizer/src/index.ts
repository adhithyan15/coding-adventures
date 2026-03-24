/**
 * Pipeline Visualizer — Language-agnostic HTML visualization generator.
 * ======================================================================
 *
 * This package reads JSON conforming to the PipelineReport contract
 * and generates self-contained HTML files. Any language that can
 * produce the JSON (Python, Ruby, TypeScript, etc.) can use this
 * visualizer — it has no dependency on other coding-adventures packages.
 *
 * ```
 * Python packages ----> PipelineReport (JSON) --+
 * Ruby packages -----> PipelineReport (JSON) ---+--> Pipeline Visualizer --> report.html
 * TypeScript pkgs ---> PipelineReport (JSON) --+
 * ```
 *
 * ## Quick Start
 *
 * ```typescript
 * import { HTMLRenderer } from "@coding-adventures/pipeline-visualizer";
 *
 * const renderer = new HTMLRenderer();
 *
 * // From a JSON file:
 * const html = renderer.renderFromJson("pipeline-report.json");
 *
 * // From a TypeScript object:
 * const html = renderer.render(report);
 *
 * // Directly to a file:
 * renderer.renderToFile(report, "report.html");
 * ```
 *
 * ## Architecture
 *
 * The renderer uses a *dispatch table* pattern: each pipeline stage
 * (lexer, parser, compiler, etc.) has its own rendering function.
 * The main renderer looks up the function by the stage's `name` field
 * and calls it with the stage data. Unknown stages get a JSON dump.
 *
 * ```
 * PipelineReport.stages[i]
 *        │
 *        ├── stage.name = "lexer"    → renderLexerStage()
 *        ├── stage.name = "parser"   → renderParserStage()
 *        ├── stage.name = "compiler" → renderCompilerStage()
 *        ├── stage.name = "vm"       → renderVMStage()
 *        ├── stage.name = "assembler"→ renderAssemblerStage()
 *        ├── stage.name = "riscv"    → renderHardwareExecutionStage()
 *        ├── stage.name = "arm"      → renderHardwareExecutionStage()
 *        ├── stage.name = "alu"      → renderALUStage()
 *        ├── stage.name = "gates"    → renderGateStage()
 *        └── (unknown)               → renderFallbackStage()
 * ```
 */

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export { HTMLRenderer } from "./renderer.js";

// ---------------------------------------------------------------------------
// Types (re-exported for consumers who want to construct reports)
// ---------------------------------------------------------------------------

export type {
  PipelineReport,
  ReportMetadata,
  StageReport,
  StageData,
  LexerData,
  LexerToken,
  ParserData,
  ASTNode,
  CompilerData,
  BytecodeInstruction,
  VMData,
  VMStep,
  AssemblerData,
  AssemblyLine,
  HardwareExecutionData,
  HardwareStep,
  ALUData,
  ALUOperation,
  GateData,
  GateOperation,
  Gate,
} from "./types.js";

// ---------------------------------------------------------------------------
// Utilities (exported for advanced usage / testing)
// ---------------------------------------------------------------------------

export { escapeHtml } from "./escape.js";
export { getStyles } from "./styles.js";
export {
  renderLexerStage,
  renderParserStage,
  renderCompilerStage,
  renderVMStage,
  renderAssemblerStage,
  renderHardwareExecutionStage,
  renderALUStage,
  renderGateStage,
  renderFallbackStage,
} from "./stage-renderers.js";
