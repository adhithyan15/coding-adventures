/**
 * Tests for type definitions — compile-time validation.
 * =======================================================
 *
 * TypeScript interfaces have no runtime representation, so we can't
 * test them the way we test functions. Instead, these tests verify
 * that our test fixtures conform to the type definitions — if the
 * types change in an incompatible way, these tests will fail to
 * compile.
 *
 * We also test that the types work correctly with JSON.parse, since
 * that's how real-world data will arrive.
 */

import { describe, it, expect } from "vitest";
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
} from "../src/types.js";
import { vmReport, riscvReport, minimalReport } from "./fixtures.js";

describe("PipelineReport type", () => {
  it("should have required fields on vmReport", () => {
    const report: PipelineReport = vmReport;
    expect(report.source).toBeDefined();
    expect(report.language).toBeDefined();
    expect(report.target).toBeDefined();
    expect(report.metadata).toBeDefined();
    expect(report.stages).toBeDefined();
    expect(Array.isArray(report.stages)).toBe(true);
  });

  it("should have required metadata fields", () => {
    expect(vmReport.metadata.generated_at).toBeDefined();
    expect(vmReport.metadata.generator_version).toBeDefined();
    expect(vmReport.metadata.packages).toBeDefined();
  });

  it("should survive JSON round-trip", () => {
    const json = JSON.stringify(vmReport);
    const parsed = JSON.parse(json) as PipelineReport;
    expect(parsed.source).toBe(vmReport.source);
    expect(parsed.language).toBe(vmReport.language);
    expect(parsed.stages.length).toBe(vmReport.stages.length);
  });

  it("should work with minimal report", () => {
    const report: PipelineReport = minimalReport;
    expect(report.stages.length).toBe(0);
    expect(report.source).toBe("");
  });
});

describe("StageReport type", () => {
  it("should have required fields", () => {
    const stage: StageReport = vmReport.stages[0];
    expect(stage.name).toBeDefined();
    expect(stage.display_name).toBeDefined();
    expect(stage.input_repr).toBeDefined();
    expect(stage.output_repr).toBeDefined();
    expect(stage.duration_ms).toBeDefined();
    expect(stage.data).toBeDefined();
  });
});

describe("Stage data types", () => {
  it("LexerData should have tokens array", () => {
    const data = vmReport.stages[0].data as LexerData;
    expect(Array.isArray(data.tokens)).toBe(true);
    expect(data.tokens[0].type).toBeDefined();
    expect(data.tokens[0].value).toBeDefined();
    expect(data.tokens[0].line).toBeDefined();
    expect(data.tokens[0].column).toBeDefined();
  });

  it("ParserData should have ast node", () => {
    const data = vmReport.stages[1].data as ParserData;
    expect(data.ast).toBeDefined();
    expect(data.ast.type).toBeDefined();
    expect(Array.isArray(data.ast.children)).toBe(true);
  });

  it("CompilerData should have instructions, constants, names", () => {
    const data = vmReport.stages[2].data as CompilerData;
    expect(Array.isArray(data.instructions)).toBe(true);
    expect(Array.isArray(data.constants)).toBe(true);
    expect(Array.isArray(data.names)).toBe(true);
  });

  it("VMData should have steps", () => {
    const data = vmReport.stages[3].data as VMData;
    expect(Array.isArray(data.steps)).toBe(true);
    expect(data.steps[0].index).toBeDefined();
    expect(data.steps[0].instruction).toBeDefined();
    expect(Array.isArray(data.steps[0].stack_before)).toBe(true);
    expect(Array.isArray(data.steps[0].stack_after)).toBe(true);
  });

  it("AssemblerData should have lines", () => {
    const data = riscvReport.stages[2].data as AssemblerData;
    expect(Array.isArray(data.lines)).toBe(true);
    expect(data.lines[0].address).toBeDefined();
    expect(data.lines[0].assembly).toBeDefined();
    expect(data.lines[0].binary).toBeDefined();
    expect(data.lines[0].encoding).toBeDefined();
  });

  it("HardwareExecutionData should have steps with registers", () => {
    const data = riscvReport.stages[3].data as HardwareExecutionData;
    expect(Array.isArray(data.steps)).toBe(true);
    expect(data.steps[0].registers).toBeDefined();
    expect(data.steps[0].registers_changed).toBeDefined();
  });

  it("ALUData should have operations with flags", () => {
    const data = riscvReport.stages[4].data as ALUData;
    expect(Array.isArray(data.operations)).toBe(true);
    expect(data.operations[0].flags).toBeDefined();
    expect(data.operations[0].flags.zero).toBeDefined();
  });

  it("GateData should have operations with gates", () => {
    const data = riscvReport.stages[5].data as GateData;
    expect(Array.isArray(data.operations)).toBe(true);
    expect(data.operations[0].gates).toBeDefined();
    expect(data.operations[0].gates[0].gate).toBeDefined();
  });
});
