/**
 * Tests for the HTMLRenderer class — the main public API.
 * =========================================================
 *
 * These tests verify the full rendering pipeline: PipelineReport
 * in, self-contained HTML out. We test:
 *
 * - Valid HTML structure (doctype, head, body)
 * - Correct sections for different pipeline types
 * - Edge cases (empty source, XSS, unknown stages)
 * - File I/O (renderFromJson, renderToFile)
 * - JSON round-trip (report -> JSON -> file -> renderFromJson)
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { writeFileSync, readFileSync, existsSync, unlinkSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { HTMLRenderer } from "../src/renderer.js";
import {
  vmReport,
  riscvReport,
  minimalReport,
  singleTokenReport,
  xssReport,
  unknownStageReport,
  deepAstReport,
} from "./fixtures.js";

// ===========================================================================
// HTML Structure
// ===========================================================================

describe("HTMLRenderer — HTML structure", () => {
  const renderer = new HTMLRenderer();

  it("should produce valid HTML5 document", () => {
    const html = renderer.render(vmReport);
    expect(html).toContain("<!DOCTYPE html>");
    expect(html).toContain("<html lang=\"en\">");
    expect(html).toContain("<head>");
    expect(html).toContain("</head>");
    expect(html).toContain("<body>");
    expect(html).toContain("</body>");
    expect(html).toContain("</html>");
  });

  it("should include charset and viewport meta tags", () => {
    const html = renderer.render(vmReport);
    expect(html).toContain('charset="UTF-8"');
    expect(html).toContain('name="viewport"');
  });

  it("should include embedded CSS in a style tag", () => {
    const html = renderer.render(vmReport);
    expect(html).toContain("<style>");
    expect(html).toContain("</style>");
    // Check for some CSS rules we know should be there
    expect(html).toContain("font-family");
    expect(html).toContain("background-color");
  });

  it("should include page title with source code", () => {
    const html = renderer.render(vmReport);
    expect(html).toContain("<title>Computing Stack Report");
    expect(html).toContain("x = 1 + 2");
  });

  it("should include header with source code", () => {
    const html = renderer.render(vmReport);
    expect(html).toContain("<header>");
    expect(html).toContain("Computing Stack Report");
    expect(html).toContain("x = 1 + 2");
  });

  it("should include metadata in header", () => {
    const html = renderer.render(vmReport);
    expect(html).toContain("python");
    expect(html).toContain("vm");
    expect(html).toContain("2026-03-18");
  });

  it("should include footer with version", () => {
    const html = renderer.render(vmReport);
    expect(html).toContain("<footer>");
    expect(html).toContain("0.1.0");
  });
});

// ===========================================================================
// VM-only Pipeline
// ===========================================================================

describe("HTMLRenderer — VM pipeline", () => {
  const renderer = new HTMLRenderer();

  it("should include lexer section", () => {
    const html = renderer.render(vmReport);
    expect(html).toContain('id="lexer"');
    expect(html).toContain("Tokenization");
  });

  it("should include parser section", () => {
    const html = renderer.render(vmReport);
    expect(html).toContain('id="parser"');
    expect(html).toContain("Parsing");
  });

  it("should include compiler section", () => {
    const html = renderer.render(vmReport);
    expect(html).toContain('id="compiler"');
    expect(html).toContain("Bytecode Compilation");
  });

  it("should include VM section", () => {
    const html = renderer.render(vmReport);
    expect(html).toContain('id="vm"');
    expect(html).toContain("VM Execution");
  });

  it("should NOT include hardware sections", () => {
    const html = renderer.render(vmReport);
    expect(html).not.toContain('id="assembler"');
    expect(html).not.toContain('id="riscv"');
    expect(html).not.toContain('id="alu"');
    expect(html).not.toContain('id="gates"');
  });

  it("should show stage meta info (input, output, duration)", () => {
    const html = renderer.render(vmReport);
    expect(html).toContain("stage-meta");
    expect(html).toContain("6 tokens");
    expect(html).toContain("0.12ms");
  });
});

// ===========================================================================
// RISC-V Pipeline
// ===========================================================================

describe("HTMLRenderer — RISC-V pipeline", () => {
  const renderer = new HTMLRenderer();

  it("should include lexer and parser sections", () => {
    const html = renderer.render(riscvReport);
    expect(html).toContain('id="lexer"');
    expect(html).toContain('id="parser"');
  });

  it("should include assembler section", () => {
    const html = renderer.render(riscvReport);
    expect(html).toContain('id="assembler"');
    expect(html).toContain("Assembly");
  });

  it("should include RISC-V execution section", () => {
    const html = renderer.render(riscvReport);
    expect(html).toContain('id="riscv"');
    expect(html).toContain("RISC-V Execution");
  });

  it("should include ALU section", () => {
    const html = renderer.render(riscvReport);
    expect(html).toContain('id="alu"');
    expect(html).toContain("ALU Operations");
  });

  it("should include gates section", () => {
    const html = renderer.render(riscvReport);
    expect(html).toContain('id="gates"');
    expect(html).toContain("Gate Operations");
  });

  it("should NOT include VM or compiler sections", () => {
    const html = renderer.render(riscvReport);
    expect(html).not.toContain('id="vm"');
    expect(html).not.toContain('id="compiler"');
  });
});

// ===========================================================================
// Edge Cases
// ===========================================================================

describe("HTMLRenderer — edge cases", () => {
  const renderer = new HTMLRenderer();

  it("should handle minimal report with no stages", () => {
    const html = renderer.render(minimalReport);
    expect(html).toContain("<!DOCTYPE html>");
    expect(html).toContain("<header>");
    expect(html).toContain("<footer>");
    // No sections
    expect(html).not.toContain("<section");
  });

  it("should handle single-token report", () => {
    const html = renderer.render(singleTokenReport);
    expect(html).toContain("NUMBER");
    expect(html).toContain("42");
  });

  it("should escape HTML in source code (XSS prevention)", () => {
    const html = renderer.render(xssReport);
    // The script tag should be escaped, not rendered as HTML
    expect(html).toContain("&lt;script&gt;");
    expect(html).not.toContain("<script>alert");
  });

  it("should handle unknown stage types with fallback", () => {
    const html = renderer.render(unknownStageReport);
    expect(html).toContain("Quantum Simulation");
    // Fallback renders as JSON
    expect(html).toContain("qubits");
    expect(html).toContain("H");
  });

  it("should handle deeply nested AST", () => {
    const html = renderer.render(deepAstReport);
    expect(html).toContain("<svg");
    // All four levels should be present
    expect(html).toContain("BinaryOp(+)");
    expect(html).toContain("Number(1)");
    expect(html).toContain("Number(4)");
  });
});

// ===========================================================================
// File I/O
// ===========================================================================

describe("HTMLRenderer — file I/O", () => {
  const renderer = new HTMLRenderer();
  let tempDir: string;

  beforeEach(() => {
    tempDir = join(tmpdir(), `html-renderer-test-${Date.now()}`);
    mkdirSync(tempDir, { recursive: true });
  });

  afterEach(() => {
    // Clean up temp files
    try {
      const jsonFile = join(tempDir, "test-report.json");
      const htmlFile = join(tempDir, "test-output.html");
      if (existsSync(jsonFile)) unlinkSync(jsonFile);
      if (existsSync(htmlFile)) unlinkSync(htmlFile);
    } catch {
      // Ignore cleanup errors
    }
  });

  it("should render from a JSON file", () => {
    const jsonPath = join(tempDir, "test-report.json");
    writeFileSync(jsonPath, JSON.stringify(vmReport), "utf-8");

    const html = renderer.renderFromJson(jsonPath);
    expect(html).toContain("<!DOCTYPE html>");
    expect(html).toContain("x = 1 + 2");
    expect(html).toContain("Tokenization");
  });

  it("should render directly to an HTML file", () => {
    const outputPath = join(tempDir, "test-output.html");
    renderer.renderToFile(vmReport, outputPath);

    expect(existsSync(outputPath)).toBe(true);
    const content = readFileSync(outputPath, "utf-8");
    expect(content).toContain("<!DOCTYPE html>");
    expect(content).toContain("x = 1 + 2");
  });

  it("should round-trip: report -> JSON -> file -> renderFromJson", () => {
    // Render the report to HTML directly
    const directHtml = renderer.render(vmReport);

    // Save report as JSON, then render from JSON
    const jsonPath = join(tempDir, "test-report.json");
    writeFileSync(jsonPath, JSON.stringify(vmReport), "utf-8");
    const fromJsonHtml = renderer.renderFromJson(jsonPath);

    // Both should produce identical HTML
    expect(fromJsonHtml).toBe(directHtml);
  });
});

// ===========================================================================
// Theme (future-proofing)
// ===========================================================================

describe("HTMLRenderer — theme", () => {
  it("should accept a theme parameter", () => {
    const renderer = new HTMLRenderer("dark");
    const html = renderer.render(minimalReport);
    expect(html).toContain("<!DOCTYPE html>");
  });

  it("should default to dark theme", () => {
    const renderer = new HTMLRenderer();
    const html = renderer.render(minimalReport);
    // Dark theme uses #1e1e2e background
    expect(html).toContain("#1e1e2e");
  });
});
