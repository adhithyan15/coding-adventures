import { mkdtemp, readFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { Intel4004Simulator } from "@coding-adventures/intel4004-simulator";
import { describe, expect, it } from "vitest";

import { NibCompiler, compileSource, decodeHex, writeHexFile } from "../src/index.js";

const SAMPLE = `
fn add(a: u4, b: u4) -> u4 {
    return a +% b;
}

fn main() {
    let result: u4 = add(3, 4);
}
`;

describe("nib-compiler", () => {
  it("compiles nib source to hex and preserves intermediate artifacts", () => {
    const result = compileSource(SAMPLE);
    expect(result.hexText).toContain(":");
    expect(result.assembly).toContain("_fn_add:");
    expect(result.binary.length).toBeGreaterThan(0);
    expect(result.optimizedIr.instructions.length).toBeGreaterThan(0);
  });

  it("round-trips the generated intel hex", () => {
    const result = compileSource(SAMPLE);
    const decoded = decodeHex(result.hexText);
    expect(decoded.origin).toBe(0);
    expect(Array.from(decoded.binary)).toEqual(Array.from(result.binary));
  });

  it("runs the compiled program in the intel 4004 simulator", () => {
    const result = compileSource(SAMPLE);
    const decoded = decodeHex(result.hexText);
    const simulator = new Intel4004Simulator();
    const traces = simulator.run(decoded.binary);
    expect(traces.length).toBeGreaterThan(0);
    expect(simulator.halted).toBe(true);
  });

  it("writes hex output to disk", async () => {
    const directory = await mkdtemp(join(tmpdir(), "nib-compiler-"));
    try {
      const outputPath = join(directory, "program.hex");
      const result = await writeHexFile(SAMPLE, outputPath);
      const text = await readFile(outputPath, "utf8");
      expect(text).toBe(result.hexText);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  it("supports disabling ir optimization", () => {
    const optimized = new NibCompiler({ optimizeIr: true }).compileSource(SAMPLE);
    const unoptimized = new NibCompiler({ optimizeIr: false }).compileSource(SAMPLE);
    expect(unoptimized.rawIr.instructions.length).toBeGreaterThanOrEqual(
      optimized.optimizedIr.instructions.length,
    );
  });
});
