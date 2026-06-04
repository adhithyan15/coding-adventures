import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";
import type { ASTNode } from "@coding-adventures/parser";

import {
  NibWasmCompiler,
  PackageError,
  compileSource,
  extractSignatures,
  packSource,
  writeWasmFile,
} from "../src/index.js";

function astNode(ruleName: string, children: unknown[] = []): ASTNode {
  return { ruleName, children } as unknown as ASTNode;
}

function tok(type: string, value: string) {
  return { type, value, line: 1, column: 1 };
}

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

  it("compiles with optimization disabled", () => {
    const result = new NibWasmCompiler({ optimize: false }).compileSource("fn answer() -> u4 { return 7; }");
    expect(result.binary.length).toBeGreaterThan(0);
    expect(result.optimization).toBeDefined();
  });

  it("writeWasmFile wraps filesystem errors in PackageError", () => {
    const source = "fn answer() -> u4 { return 7; }";
    // Create a regular file, then try to use it as a directory component → ENOTDIR
    const tmpDir = mkdtempSync(join(tmpdir(), "nib-test-"));
    const blockerFile = join(tmpDir, "blocker");
    writeFileSync(blockerFile, "");
    const badPath = join(blockerFile, "sub", "out.wasm");

    expect(() => new NibWasmCompiler().writeWasmFile(source, badPath)).toThrow(PackageError);
    try {
      new NibWasmCompiler().writeWasmFile(source, badPath);
    } catch (err) {
      expect(err).toBeInstanceOf(PackageError);
      expect((err as PackageError).stage).toBe("write");
    }
  });

  it("extractSignatures returns only _start for an empty program", () => {
    const program = astNode("program", []);
    const sigs = extractSignatures(program);
    expect(sigs).toHaveLength(1);
    expect(sigs[0].exportName).toBe("_start");
    expect(sigs[0].paramCount).toBe(0);
  });

  it("extractSignatures skips top_decl with no inner decl", () => {
    // unwrapTopDecl returns null when top_decl has no ASTNode children
    const topDecl = astNode("top_decl", []);
    const program = astNode("program", [topDecl]);
    const sigs = extractSignatures(program);
    expect(sigs).toHaveLength(1);
  });

  it("extractSignatures skips top-level declarations that are not fn_decl", () => {
    const nonFn = astNode("type_decl", []);
    const topDecl = astNode("top_decl", [nonFn]);
    const program = astNode("program", [topDecl]);
    const sigs = extractSignatures(program);
    expect(sigs).toHaveLength(1);
  });

  it("extractSignatures skips fn_decl with no NAME token in its subtree", () => {
    // fn_decl whose only children are non-NAME tokens → firstName returns null
    const fnDecl = astNode("fn_decl", [tok("LPAREN", "("), tok("RPAREN", ")")]);
    const topDecl = astNode("top_decl", [fnDecl]);
    const program = astNode("program", [topDecl]);
    const sigs = extractSignatures(program);
    expect(sigs).toHaveLength(1);
  });

  it("extractSignatures resolves a NAME token nested one level deep in fn_decl", () => {
    // emptyNode (ASTNode with no children) causes firstName to recurse but return null,
    // then the NAME token is found on the next iteration
    const emptyNode = astNode("empty", []);
    const nameTok = tok("NAME", "myFunc");
    const fnDecl = astNode("fn_decl", [emptyNode, nameTok]);
    const topDecl = astNode("top_decl", [fnDecl]);
    const program = astNode("program", [topDecl]);
    const sigs = extractSignatures(program);
    expect(sigs).toHaveLength(2);
    expect(sigs[1].exportName).toBe("myFunc");
    expect(sigs[1].paramCount).toBe(0);
  });
});
