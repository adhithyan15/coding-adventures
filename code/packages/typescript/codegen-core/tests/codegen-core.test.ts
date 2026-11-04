import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { IirFunction, IirInstr, IirModule, Types } from "@coding-adventures/interpreter-ir";
import { BackendRegistry, TextBackend } from "../src/index.js";

describe("codegen-core", () => {
  it("emits text artifacts", () => {
    const mod = new IirModule({ name: "sample", language: "test", functions: [new IirFunction({ name: "main", returnType: Types.U8, instructions: [IirInstr.of("const", { dest: "x", srcs: [42], typeHint: Types.U8 }), IirInstr.of("ret", { srcs: ["x"] })] })] });
    const registry = BackendRegistry.default();
    const wasm = registry.compile(mod, "wasm");
    assert.deepEqual(registry.targets(), ["pure_vm", "jvm", "clr", "wasm"]);
    assert.match(wasm.body, /target=wasm/);
    assert.match(wasm.body, /\.function main -> u8/);
  });
  it("registers custom backends", () => {
    const registry = new BackendRegistry();
    registry.register(new TextBackend("custom"));
    assert.equal(registry.fetch("custom").target, "custom");
    assert.throws(() => registry.fetch("missing"), /unknown backend target/);
  });
});
