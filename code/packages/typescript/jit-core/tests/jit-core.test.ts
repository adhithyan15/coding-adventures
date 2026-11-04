import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { FunctionTypeStatus, IirFunction, IirInstr, IirModule, Types } from "@coding-adventures/interpreter-ir";
import { VMCore } from "@coding-adventures/vm-core";
import { JITCore } from "../src/index.js";

describe("jit-core", () => {
  it("installs pure VM handlers for typed functions", () => {
    const mod = new IirModule({ name: "jit", functions: [new IirFunction({ name: "main", returnType: Types.U8, typeStatus: FunctionTypeStatus.FullyTyped, instructions: [IirInstr.of("const", { dest: "x", srcs: [42], typeHint: Types.U8 }), IirInstr.of("ret", { srcs: ["x"] })] })] });
    const vm = new VMCore();
    assert.equal(new JITCore(vm).executeWithJit(mod), 42);
    assert.equal(vm.metrics().totalJitHits, 1);
  });
  it("emits artifacts", () => {
    const mod = new IirModule({ name: "emit", functions: [new IirFunction({ name: "main", instructions: [IirInstr.of("ret_void")] })] });
    assert.match(new JITCore(new VMCore()).emit(mod, "wasm").body, /target=wasm/);
  });
});
