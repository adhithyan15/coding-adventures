import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  NibWasmCompiler,
  PackageError,
  compileSource,
  packSource,
  writeWasmFile,
} from "../src/index.js";

describe("nib-wasm-compiler", () => {
  it("compileSource returns pipeline artifacts", () => {
    const result = compileSource("fn answer() -> u4 { return 7; }");

    expect(result.typedAst).toBeTruthy();
    expect(result.rawIr.instructions.length).toBeGreaterThan(0);
    expect(result.optimizedIr.instructions.length).toBeGreaterThan(0);
    expect(result.binary.length).toBeGreaterThan(0);
    expect(result.module.exports.some((entry) => entry.name === "answer")).toBe(true);
  });

  it("packSource is an alias for compileSource", () => {
    const compiled = compileSource("fn answer() -> u4 { return 7; }");
    const packed = packSource("fn answer() -> u4 { return 7; }");

    expect(Array.from(packed.binary)).toEqual(Array.from(compiled.binary));
  });

  it("writeWasmFile writes the output bytes", () => {
    const outputDir = mkdtempSync(join(tmpdir(), "nib-wasm-"));
    const outputPath = join(outputDir, "program.wasm");

    const result = writeWasmFile("fn answer() -> u4 { return 7; }", outputPath);

    expect(Array.from(readFileSync(outputPath))).toEqual(Array.from(result.binary));
  });

  it("exports compiled functions in the wasm module", () => {
    const result = compileSource("fn answer() -> u4 { return 7; }");

    expect(result.module.exports.some((entry) => entry.name === "answer")).toBe(true);
    expect(result.validatedModule.funcTypes.length).toBeGreaterThan(0);
  });

  it("exports compiled entrypoints in the wasm module", () => {
    const source = "fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() -> u4 { return add(3, 4); }";
    const result = new NibWasmCompiler().compileSource(source);

    expect(result.module.exports.some((entry) => entry.name === "_start")).toBe(true);
    expect(result.module.exports.some((entry) => entry.name === "main")).toBe(true);
  });

  it("raises package errors with stage metadata for type failures", () => {
    expect(() => compileSource("fn main() { let flag: bool = 1; }")).toThrow(PackageError);
    try {
      compileSource("fn main() { let flag: bool = 1; }");
    } catch (error) {
      expect(error).toBeInstanceOf(PackageError);
      expect((error as PackageError).stage).toBe("type-check");
    }
  });
});
