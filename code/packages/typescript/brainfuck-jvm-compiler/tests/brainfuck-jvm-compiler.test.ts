import { mkdtempSync, realpathSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  BrainfuckJvmCompiler,
  PackageError,
  compileSource,
  packSource,
  writeClassFile,
} from "../src/index.js";

describe("brainfuck-jvm-compiler", () => {
  it("returns the expected pipeline artifacts", () => {
    const result = compileSource("+.");
    expect(result.rawIr).toBeDefined();
    expect(result.optimizedIr).toBeDefined();
    expect(result.classBytes.length).toBeGreaterThan(0);
    expect(result.className).toBe("BrainfuckProgram");
    expect(result.parsedClass.thisClassName).toBe("BrainfuckProgram");
    expect(result.parsedClass.findMethod("_start", "()I")).not.toBeNull();
  });

  it("aliases packSource to compileSource", () => {
    const compiled = compileSource("+.");
    const packed = packSource("+.");
    expect(packed.classBytes).toEqual(compiled.classBytes);
    expect(packed.className).toBe(compiled.className);
  });

  it("writes class files into classpath layout", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-bf-jvm-"));
    try {
      const result = writeClassFile("+.", tempdir);
      expect(result.classFilePath).toBe(join(realpathSync.native(tempdir), "BrainfuckProgram.class"));
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });

  it("honors custom filename and class name", () => {
    const result = new BrainfuckJvmCompiler({
      filename: "hello.bf",
      className: "demo.HelloBrainfuck",
    }).compileSource("+");
    expect(result.filename).toBe("hello.bf");
    expect(result.className).toBe("demo.HelloBrainfuck");
    expect(result.parsedClass.thisClassName).toBe("demo/HelloBrainfuck");
  });

  it("raises stage-labeled lower-jvm errors", () => {
    expect(() => compileSource("+", { className: ".BadName" })).toThrow(PackageError);
    try {
      compileSource("+", { className: ".BadName" });
    } catch (error) {
      expect((error as PackageError).stage).toBe("lower-jvm");
    }
  });
});
