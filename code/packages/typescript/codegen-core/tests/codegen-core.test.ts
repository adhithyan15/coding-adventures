import { describe, expect, it } from "vitest";
import { IirFunction, IirInstr, IirModule, Types } from "@coding-adventures/interpreter-ir";
import { BackendRegistry, TextBackend } from "../src/index.js";

describe("codegen-core", () => {
  it("emits text artifacts", () => {
    const mod = new IirModule({ name: "sample", language: "test", functions: [new IirFunction({ name: "main", returnType: Types.U8, instructions: [IirInstr.of("const", { dest: "x", srcs: [42], typeHint: Types.U8 }), IirInstr.of("ret", { srcs: ["x"] })] })] });
    const registry = BackendRegistry.default();
    const wasm = registry.compile(mod, "wasm");
    expect(registry.targets()).toEqual(["pure_vm", "jvm", "clr", "wasm"]);
    expect(wasm.body).toContain("target=wasm");
    expect(wasm.body).toContain(".function main -> u8");
  });
  it("registers custom backends", () => {
    const registry = new BackendRegistry();
    registry.register(new TextBackend("custom"));
    expect(registry.fetch("custom").target).toBe("custom");
    expect(() => registry.fetch("missing")).toThrow(/unknown backend target/);
  });
});
