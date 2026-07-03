import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";
import { WasiStub, WasmRuntime } from "@coding-adventures/wasm-runtime";

import {
  BrainfuckWasmCompiler,
  PackageError,
  compileSource,
  packSource,
  writeWasmFile,
} from "../src/index.js";

class ByteReader {
  private readonly bytes: Uint8Array;
  private offset = 0;

  constructor(text: string) {
    this.bytes = new Uint8Array(Array.from(text, (char) => char.charCodeAt(0) & 0xff));
  }

  read(count: number): Uint8Array {
    const chunk = this.bytes.slice(this.offset, this.offset + count);
    this.offset += chunk.length;
    return chunk;
  }
}

function run(binary: Uint8Array, stdin: string = ""): { result: number[]; output: string[] } {
  const output: string[] = [];
  const reader = new ByteReader(stdin);
  const runtime = new WasmRuntime(new WasiStub({
    stdin: (count) => reader.read(count),
    stdout: (text) => output.push(text),
  }));
  return {
    result: runtime.loadAndRun(binary, "_start", []),
    output,
  };
}

describe("brainfuck-wasm-compiler", () => {
  it("compileSource returns pipeline artifacts", () => {
    const result = compileSource("+.");

    expect(result.rawIr.instructions.length).toBeGreaterThan(0);
    expect(result.optimizedIr.instructions.length).toBeGreaterThan(0);
    expect(result.binary.length).toBeGreaterThan(0);
    expect(result.module.exports.some((entry) => entry.name === "_start")).toBe(true);
    expect(result.filename).toBe("program.bf");
  });

  it("packSource is an alias for compileSource", () => {
    const compiled = compileSource("+.");
    const packed = packSource("+.");

    expect(Array.from(packed.binary)).toEqual(Array.from(compiled.binary));
  });

  it("writeWasmFile writes the output bytes", () => {
    const outputDir = mkdtempSync(join(tmpdir(), "bf-wasm-"));
    const outputPath = join(outputDir, "program.wasm");

    const result = writeWasmFile("+.", outputPath);

    expect(Array.from(readFileSync(outputPath))).toEqual(Array.from(result.binary));
  });

  it("runs compiled output programs in the wasm runtime", () => {
    const result = compileSource(`${"+".repeat(65)}.`);
    const execution = run(result.binary);

    expect(execution.result).toEqual([0]);
    expect(execution.output).toEqual(["A"]);
  });

  it("runs compiled input programs in the wasm runtime", () => {
    const result = compileSource(",.");
    const execution = run(result.binary, "Z");

    expect(execution.result).toEqual([0]);
    expect(execution.output).toEqual(["Z"]);
  });

  it("runs compiled cat programs in the wasm runtime", () => {
    const result = compileSource(",[.,]");
    const execution = run(result.binary, "Hi");

    expect(execution.result).toEqual([0]);
    expect(execution.output).toEqual(["H", "i"]);
  });

  it("honors a custom filename", () => {
    const result = new BrainfuckWasmCompiler({ filename: "hello.bf" }).compileSource("+");
    expect(result.filename).toBe("hello.bf");
  });

  it("compiles with optimization disabled", () => {
    const result = new BrainfuckWasmCompiler({ optimize: false }).compileSource("+.");
    expect(result.binary.length).toBeGreaterThan(0);
    expect(result.optimization).toBeDefined();
  });

  it("PackageError stores stage, message, and cause", () => {
    const cause = new Error("inner");
    const err = new PackageError("stage-name", "test message", cause);
    expect(err.name).toBe("PackageError");
    expect(err.stage).toBe("stage-name");
    expect(err.message).toBe("test message");
    expect(err.cause).toBe(cause);
    expect(err).toBeInstanceOf(Error);
  });

  it("PackageError works without a cause", () => {
    const err = new PackageError("encode", "encoding failed");
    expect(err.stage).toBe("encode");
    expect(err.cause).toBeUndefined();
  });

  it("writeWasmFile wraps filesystem errors in PackageError", () => {
    // Parent directory does not exist → writeFileSync throws ENOENT
    const badPath = join(tmpdir(), "bf-nonexistent-xyz9999abc", "out.wasm");
    expect(() => writeWasmFile("+.", badPath)).toThrow(PackageError);
    try {
      writeWasmFile("+.", badPath);
    } catch (err) {
      expect(err).toBeInstanceOf(PackageError);
      expect((err as PackageError).stage).toBe("write");
    }
  });
});
