import { mkdtempSync, realpathSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  NibJvmCompiler,
  PackageError,
  compileSource,
  packSource,
  writeClassFile,
} from "../src/index.js";

describe("nib-jvm-compiler", () => {
  it("returns the expected pipeline artifacts", () => {
    const result = compileSource("fn main() -> u4 { return 7; }");
    expect(result.rawIr).toBeDefined();
    expect(result.optimizedIr).toBeDefined();
    expect(result.classBytes.length).toBeGreaterThan(0);
    expect(result.className).toBe("NibProgram");
    expect(result.parsedClass.thisClassName).toBe("NibProgram");
    expect(result.parsedClass.findMethod("_start", "()I")).not.toBeNull();
  });

  it("aliases packSource to compileSource", () => {
    const compiled = compileSource("fn main() -> u4 { return 7; }");
    const packed = packSource("fn main() -> u4 { return 7; }");
    expect(packed.classBytes).toEqual(compiled.classBytes);
    expect(packed.className).toBe(compiled.className);
  });

  it("writes class files into classpath layout", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-nib-jvm-"));
    try {
      const result = writeClassFile("fn main() -> u4 { return 7; }", tempdir);
      expect(result.classFilePath).toBe(join(realpathSync.native(tempdir), "NibProgram.class"));
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });

  it("honors custom class name and wrapper policy", () => {
    const result = new NibJvmCompiler({
      className: "demo.CustomNib",
      emitMainWrapper: false,
    }).compileSource("fn main() -> u4 { return 7; }");
    expect(result.className).toBe("demo.CustomNib");
    expect(result.parsedClass.thisClassName).toBe("demo/CustomNib");
    expect(result.parsedClass.findMethod("main", "([Ljava/lang/String;)V")).toBeNull();
  });

  it("raises type-check errors with stage labels", () => {
    expect(() => compileSource("fn main() { let x: bool = 1 +% 2; }")).toThrow(PackageError);
    try {
      compileSource("fn main() { let x: bool = 1 +% 2; }");
    } catch (error) {
      expect((error as PackageError).stage).toBe("type-check");
    }
  });

  it("raises parse errors with stage labels", () => {
    expect(() => compileSource("fn main(")).toThrow(PackageError);
    try {
      compileSource("fn main(");
    } catch (error) {
      expect((error as PackageError).stage).toBe("parse");
    }
  });
});
