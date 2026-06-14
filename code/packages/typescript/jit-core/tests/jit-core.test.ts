import { describe, expect, it } from "vitest";
import { FunctionTypeStatus, IirFunction, IirInstr, IirModule, Types } from "@coding-adventures/interpreter-ir";
import { VMCore } from "@coding-adventures/vm-core";
import { JITCore } from "../src/index.js";

describe("jit-core", () => {
  it("installs pure VM handlers for typed functions", () => {
    const mod = new IirModule({ name: "jit", functions: [new IirFunction({ name: "main", returnType: Types.U8, typeStatus: FunctionTypeStatus.FullyTyped, instructions: [IirInstr.of("const", { dest: "x", srcs: [42], typeHint: Types.U8 }), IirInstr.of("ret", { srcs: ["x"] })] })] });
    const vm = new VMCore();
    expect(new JITCore(vm).executeWithJit(mod)).toBe(42);
    expect(vm.metrics().totalJitHits).toBe(1);
  });
  it("emits artifacts", () => {
    const mod = new IirModule({ name: "emit", functions: [new IirFunction({ name: "main", instructions: [IirInstr.of("ret_void")] })] });
    expect(new JITCore(new VMCore()).emit(mod, "wasm").body).toContain("target=wasm");
  });
});
