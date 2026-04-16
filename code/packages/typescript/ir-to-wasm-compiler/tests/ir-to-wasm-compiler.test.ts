import { describe, expect, it } from "vitest";
import { parseBrainfuck } from "@coding-adventures/brainfuck";
import { compile, releaseConfig } from "@coding-adventures/brainfuck-ir-compiler";

import { IrToWasmCompiler } from "../src/index.js";

describe("IrToWasmCompiler", () => {
  it("lowers brainfuck IR into a wasm module with memory and wasi imports", () => {
    const ast = parseBrainfuck(",.");
    const { program } = compile(ast, "echo.bf", releaseConfig());

    const module = new IrToWasmCompiler().compile(program);

    expect(module.memories).toHaveLength(1);
    expect(module.exports.some((entry) => entry.name === "memory")).toBe(true);
    expect(module.exports.some((entry) => entry.name === "_start")).toBe(true);
    expect(module.imports.map((entry) => entry.name)).toEqual(["fd_write", "fd_read"]);
  });
});
