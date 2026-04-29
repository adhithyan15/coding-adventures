import { describe, expect, it } from "vitest";
import { FunctionTypeStatus, IirFunction, IirInstr, IirModule, Types } from "@coding-adventures/interpreter-ir";
import { FrameOverflowError, VMCore, VMError, VMFrame } from "../src/index.js";

describe("vm-core", () => {
  it("executes arithmetic and output", () => {
    const fn = new IirFunction({ name: "main", returnType: Types.U8, typeStatus: FunctionTypeStatus.FullyTyped, instructions: [
      IirInstr.of("const", { dest: "a", srcs: [250], typeHint: Types.U8 }),
      IirInstr.of("const", { dest: "b", srcs: [10], typeHint: Types.U8 }),
      IirInstr.of("add", { dest: "sum", srcs: ["a", "b"], typeHint: Types.U8 }),
      IirInstr.of("io_out", { srcs: [65] }),
      IirInstr.of("ret", { srcs: ["sum"] }),
    ] });
    const vm = new VMCore({ u8Wrap: true });
    expect(vm.execute(new IirModule({ name: "m", functions: [fn] }))).toBe(4);
    expect(vm.output).toBe("A");
  });

  it("records branches, coverage, calls, memory, and traces", () => {
    const helper = new IirFunction({ name: "helper", params: [{ name: "x", type: Types.U8 }], returnType: Types.U8, instructions: [
      IirInstr.of("cast", { dest: "ok", srcs: ["x"], typeHint: Types.Bool }),
      IirInstr.of("type_assert", { srcs: ["ok"], typeHint: Types.Bool }),
      IirInstr.of("ret", { srcs: ["x"] }),
    ] });
    const main = new IirFunction({ name: "main", instructions: [
      IirInstr.of("io_in", { dest: "cell", typeHint: Types.U8 }),
      IirInstr.of("const", { dest: "ptr", srcs: [2], typeHint: Types.U32 }),
      IirInstr.of("store_mem", { srcs: ["ptr", "cell"], typeHint: Types.U8 }),
      IirInstr.of("load_mem", { dest: "loaded", srcs: ["ptr"], typeHint: Types.U8 }),
      IirInstr.of("jmp_if_false", { srcs: [false, "skip"] }),
      IirInstr.of("label", { srcs: ["skip"] }),
      IirInstr.of("call", { dest: "result", srcs: ["helper", "loaded"], typeHint: Types.U8 }),
      IirInstr.of("ret", { srcs: ["result"] }),
    ] });
    const vm = new VMCore({ input: "B" });
    vm.enableCoverage();
    const traced = vm.executeTraced(new IirModule({ name: "m", functions: [main, helper] }));
    expect(traced.result).toBe(66);
    expect(vm.memory.get(2)).toBe(66);
    expect(vm.branchProfile("main", 4)?.takenCount).toBe(1);
    expect(vm.coverageData().get("main")).toContain(7);
    expect(vm.hotFunctions()).toEqual(["main", "helper"]);
    vm.resetCoverage();
    vm.resetMetrics();
    expect(vm.metrics().totalInstructionsExecuted).toBe(0);
  });

  it("supports loops, JIT hooks, frames, and errors", () => {
    const frameFn = new IirFunction({ name: "f", params: [{ name: "x", type: Types.U8 }] });
    const frame = new VMFrame(frameFn, [7]);
    frame.storeSlot("s", 8);
    expect(frame.loadSlot("s")).toBe(8);
    expect(frame.resolve(["x", 1])).toEqual([7, 1]);

    const loop = new IirFunction({ name: "main", instructions: [
      IirInstr.of("const", { dest: "i", srcs: [0], typeHint: Types.U8 }),
      IirInstr.of("label", { srcs: ["loop"] }),
      IirInstr.of("add", { dest: "i", srcs: ["i", 1], typeHint: Types.U8 }),
      IirInstr.of("cmp_lt", { dest: "more", srcs: ["i", 2], typeHint: Types.Bool }),
      IirInstr.of("jmp_if_true", { srcs: ["more", "loop"] }),
      IirInstr.of("ret", { srcs: ["i"] }),
    ] });
    const vm = new VMCore({ maxFrames: 1 });
    expect(vm.execute(new IirModule({ name: "loop", functions: [loop] }))).toBe(2);
    expect(vm.loopIterations("main", "loop")).toBe(1);
    vm.registerJitHandler("main", () => 99);
    expect(vm.execute(new IirModule({ name: "jit", functions: [loop] }))).toBe(99);
    vm.unregisterJitHandler("main");
    const recursive = new IirFunction({ name: "main", instructions: [IirInstr.of("call", { dest: "x", srcs: ["main"] })] });
    expect(() => vm.execute(new IirModule({ name: "r", functions: [recursive] }))).toThrow(FrameOverflowError);
    expect(() => vm.builtins.call("missing", [])).toThrow(VMError);
  });
});
