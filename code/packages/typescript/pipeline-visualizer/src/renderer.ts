/**
 * HTMLRenderer — The main orchestrator that produces self-contained HTML.
 * ========================================================================
 *
 * This is the public face of the html-renderer package. It takes a
 * PipelineReport (either as a TypeScript object or a JSON file path)
 * and produces a complete, self-contained HTML file.
 *
 * "Self-contained" means the HTML file has *everything* it needs to
 * display correctly: CSS is embedded in a `<style>` tag, SVG graphics
 * are inline, and there are no external dependencies. You can email
 * the file to someone, put it on a USB drive, or open it offline —
 * it just works.
 *
 * The rendering pipeline looks like this:
 *
 * ```
 * PipelineReport
 *       │
 *       ├── renderHeader()     → <header> with source code and metadata
 *       │
 *       ├── For each stage in report.stages:
 *       │     ├── Look up renderer by stage.name
 *       │     ├── Call renderXxxStage(stage.data)
 *       │     └── Wrap in <section> with display_name and meta info
 *       │
 *       ├── renderFooter()     → <footer> with generator info
 *       │
 *       └── Wrap everything in HTML boilerplate with embedded CSS
 * ```
 *
 * Design decisions:
 *
 * 1. **No template engine** — We generate HTML by string concatenation.
 *    For a fixed-layout report like this, a template engine would add
 *    a dependency without much benefit. The HTML structure is defined
 *    right here in code, making it easy to see exactly what gets output.
 *
 * 2. **Stage dispatch by name** — The renderer uses the stage's `name`
 *    field to pick the right rendering function. Unknown stages get a
 *    JSON dump fallback rather than being silently dropped.
 *
 * 3. **Sections only for present stages** — If a VM-only pipeline
 *    report doesn't include assembler/hardware stages, those sections
 *    simply don't appear in the HTML. No empty placeholders.
 */

import { readFileSync, writeFileSync } from "node:fs";
import { escapeHtml } from "./escape.js";
import { getStyles } from "./styles.js";
import {
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
import type {
  PipelineReport,
  StageReport,
  LexerData,
  ParserData,
  CompilerData,
  VMData,
  AssemblerData,
  HardwareExecutionData,
  ALUData,
  GateData,
} from "./types.js";

// ===========================================================================
// Stage Renderer Registry
// ===========================================================================

/**
 * A registry mapping stage names to their rendering functions.
 *
 * This is the "dispatch table" pattern: instead of a big if/else
 * chain, we store functions in a map and look them up by key.
 * This makes it easy to add new stage types — just add a new
 * entry to the map.
 *
 * The functions accept `unknown` data (since each stage has a
 * different data shape) and return an HTML string. Each function
 * internally casts the data to the expected type.
 */
type StageRendererFn = (data: Record<string, unknown>) => string;

const STAGE_RENDERERS: Record<string, StageRendererFn> = {
  lexer: (data) => renderLexerStage(data as unknown as LexerData),
  parser: (data) => renderParserStage(data as unknown as ParserData),
  compiler: (data) => renderCompilerStage(data as unknown as CompilerData),
  vm: (data) => renderVMStage(data as unknown as VMData),
  assembler: (data) => renderAssemblerStage(data as unknown as AssemblerData),
  riscv: (data) =>
    renderHardwareExecutionStage(data as unknown as HardwareExecutionData),
  arm: (data) =>
    renderHardwareExecutionStage(data as unknown as HardwareExecutionData),
  alu: (data) => renderALUStage(data as unknown as ALUData),
  gates: (data) => renderGateStage(data as unknown as GateData),
};

// ===========================================================================
// HTMLRenderer Class
// ===========================================================================

/**
 * The main renderer class.
 *
 * Usage:
 * ```typescript
 * const renderer = new HTMLRenderer();
 *
 * // From a TypeScript object:
 * const html = renderer.render(report);
 *
 * // From a JSON file:
 * const html = renderer.renderFromJson("pipeline-report.json");
 *
 * // Directly to a file:
 * renderer.renderToFile(report, "report.html");
 * ```
 *
 * The `theme` parameter is reserved for future use. Currently only
 * "dark" is supported.
 */
export class HTMLRenderer {
  private readonly theme: string;

  constructor(theme: string = "dark") {
    this.theme = theme;
  }

  // =========================================================================
  // Public API
  // =========================================================================

  /**
   * Render a PipelineReport to a complete HTML string.
   *
   * This is the main entry point. It takes a report object and
   * returns a self-contained HTML document as a string.
   */
  render(report: PipelineReport): string {
    const header = this.renderHeader(report);
    const stages = report.stages
      .map((stage) => this.renderStage(stage))
      .join("\n");
    const footer = this.renderFooter(report);
    const styles = getStyles();

    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Computing Stack Report — ${escapeHtml(report.source)}</title>
  <style>${styles}</style>
</head>
<body>
  ${header}
  ${stages}
  ${footer}
</body>
</html>`;
  }

  /**
   * Load a JSON file and render it.
   *
   * Convenience method for when you have a JSON file on disk
   * rather than an already-parsed object.
   */
  renderFromJson(jsonPath: string): string {
    const raw = readFileSync(jsonPath, "utf-8");
    const report = JSON.parse(raw) as PipelineReport;
    return this.render(report);
  }

  /**
   * Render a report and write the HTML directly to a file.
   *
   * Convenience method that combines render() with writing to disk.
   */
  renderToFile(report: PipelineReport, outputPath: string): void {
    const html = this.render(report);
    writeFileSync(outputPath, html, "utf-8");
  }

  // =========================================================================
  // Private Rendering Methods
  // =========================================================================

  /**
   * Render the page header with source code and metadata.
   *
   * The header shows:
   * - The report title
   * - The source code in a code block
   * - Metadata: language, target, and generation timestamp
   */
  private renderHeader(report: PipelineReport): string {
    const date = report.metadata.generated_at.split("T")[0] || "";

    return `<header>
    <h1>Computing Stack Report</h1>
    <pre class="source-code"><code>${escapeHtml(report.source)}</code></pre>
    <p class="meta-info">
      Implementation: ${escapeHtml(report.language)} |
      Target: ${escapeHtml(report.target)} |
      Generated: ${escapeHtml(date)}
    </p>
  </header>`;
  }

  /**
   * Render a single pipeline stage as an HTML section.
   *
   * Each stage gets:
   * - A section heading with the display name
   * - Meta info showing input/output and duration
   * - The stage-specific visualization (dispatched by name)
   *
   * Unknown stage types get a JSON fallback display.
   */
  private renderStage(stage: StageReport): string {
    const renderFn = STAGE_RENDERERS[stage.name] ?? renderFallbackStage;
    const content = renderFn(stage.data as Record<string, unknown>);

    return `<section id="${escapeHtml(stage.name)}">
    <h2>${escapeHtml(stage.display_name)}</h2>
    <div class="stage-meta">
      <span>Input: ${escapeHtml(stage.input_repr)}</span>
      <span>Output: ${escapeHtml(stage.output_repr)}</span>
      <span>Duration: ${stage.duration_ms.toFixed(2)}ms</span>
    </div>
    ${content}
  </section>`;
  }

  /**
   * Render the page footer.
   */
  private renderFooter(report: PipelineReport): string {
    return `<footer>
    <p>Generated by coding-adventures HTML Visualizer v${escapeHtml(report.metadata.generator_version)}</p>
  </footer>`;
  }
}
