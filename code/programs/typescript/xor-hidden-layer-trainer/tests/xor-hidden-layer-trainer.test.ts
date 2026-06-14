import { describe, expect, it } from "vitest";
import {
  XOR_INPUTS,
  formatLinearFailure,
  formatTrace,
  formatXorRun,
  runLinearFailureDemo,
  runXorDemo,
} from "../src/index.js";

describe("xor hidden-layer trainer", () => {
  it("learns the XOR truth table", () => {
    const result = runXorDemo({ epochs: 12000, logEvery: 4000 });
    const rounded = result.rows.map(row => row.rounded);

    expect(rounded).toEqual([0, 1, 1, 0]);
    expect(result.rows[0]!.prediction).toBeLessThan(0.15);
    expect(result.rows[1]!.prediction).toBeGreaterThan(0.85);
    expect(result.rows[2]!.prediction).toBeGreaterThan(0.85);
    expect(result.rows[3]!.prediction).toBeLessThan(0.15);
  });

  it("reports one hidden activation vector per XOR row", () => {
    const result = runXorDemo({ epochs: 100, logEvery: 100 });

    expect(result.rows).toHaveLength(XOR_INPUTS.length);
    expect(result.rows.every(row => row.hidden.length === 2)).toBe(true);
  });

  it("runs the canonical XOR graph through bytecode and matrix instructions", () => {
    const result = runXorDemo({ epochs: 100, logEvery: 100 });

    expect(result.vmRows.map(row => row.rounded)).toEqual([0, 1, 1, 0]);
    expect(result.bytecodeInstructionCount).toBeGreaterThan(0);
    expect(result.matrixInstructionCount).toBeGreaterThan(0);
    expect(result.matrixInstructionCount).toBeLessThan(result.bytecodeInstructionCount);
  });

  it("formats checkpoints and hidden activations", () => {
    const text = formatXorRun(runXorDemo({ epochs: 100, logEvery: 100 }));

    expect(text).toContain("epoch");
    expect(text).toContain("hidden=[");
  });

  it("demonstrates that a no-hidden-layer model cannot solve XOR", () => {
    const result = runLinearFailureDemo(1000, 1);
    const rounded = result.rows.map(row => row.rounded);

    expect(rounded).not.toEqual([0, 1, 1, 0]);
    expect(result.finalLoss).toBeGreaterThan(0.2);
  });

  it("formats a neuron-level trace", () => {
    const result = runXorDemo({ epochs: 100, logEvery: 100 });
    const text = formatTrace(result.trace);

    expect(text).toContain("Trace for XOR row");
    expect(text).toContain("hidden[");
    expect(text).toContain("output[");
    expect(text).toContain("delta=");
  });
});
