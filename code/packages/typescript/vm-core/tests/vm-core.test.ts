import { describe, expect, it } from "vitest";
import { FunctionTypeStatus, IirFunction, IirInstr, IirModule, Types } from "@coding-adventures/interpreter-ir";
import {
  BranchStats,
  BuiltinRegistry,
  FrameOverflowError,
  UnknownOpcodeError,
  VMCore,
  VMError,
  VMFrame,
  VMInterrupt,
  VMMetrics,
} from "../src/index.js";

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

  it("covers error constructors, registry helpers, and miscellaneous accessors", () => {
    // Error class constructors set .name correctly.
    expect(new VMError("boom").name).toBe("VMError");
    expect(new UnknownOpcodeError("xyz").name).toBe("UnknownOpcodeError");
    expect(new FrameOverflowError(4).name).toBe("FrameOverflowError");
    expect(new VMInterrupt().name).toBe("VMInterrupt");

    // Default builtins fire from the registry.
    const reg = new BuiltinRegistry();
    expect(reg.call("noop", [])).toBeNull();
    expect(reg.call("assert_eq", [1, 1])).toBeNull();
    expect(() => reg.call("assert_eq", [1, 2])).toThrow(VMError);
    expect(reg.names()).toEqual(expect.arrayContaining(["noop", "assert_eq"]));
    expect(reg.entries().some(([name]) => name === "noop")).toBe(true);

    // Empty registry (registerDefaults=false) exposes the missing-builtin path.
    const empty = new BuiltinRegistry(false);
    expect(() => empty.call("noop", [])).toThrow(VMError);

    // VMFrame.resolve handles undefined.
    expect(new VMFrame(new IirFunction({ name: "f" })).resolve(undefined)).toBeNull();

    // Metrics + BranchStats accessor surface.
    const stats = new BranchStats();
    stats.record(true); stats.record(false);
    expect(stats.takenCount).toBe(1);
    expect(stats.notTakenCount).toBe(1);
    expect(new VMMetrics().totalInstructionsExecuted).toBe(0);

    // VMCore one-line public methods (registerBuiltin, register/unregisterJitHandler, disableCoverage, interrupt).
    const vm = new VMCore();
    vm.registerBuiltin("hi", () => "hi");
    vm.registerJitHandler("g", () => 1);
    vm.unregisterJitHandler("g");
    vm.enableCoverage(); vm.disableCoverage();
    vm.interrupt();
    expect(vm.coverageData().size).toBe(0);

    // "no module loaded" / "unknown function" errors fire from invokeFunction.
    // VMCore tracks module via execute(); call execute on an empty module to
    // surface unknown-function.
    const empty2 = new IirModule({ name: "m", functions: [new IirFunction({ name: "main" })] });
    expect(() => new VMCore().execute(empty2, "nope")).toThrow(VMError);
  });

  it("dispatches every opcode + binary/cast/runtime-type/toNumber path", () => {
    const vm = new VMCore({ input: "Z" });
    // Build a function that touches: neg, not, jmp_if_false (taken+fall-through),
    // call_builtin, load_reg, store_reg, is_null, safepoint, ret_void, ret with
    // the full binaryOp menu (sub/mul/div/mod/and/or/xor/shl/shr/cmp_*) and the
    // full cast menu (U8/U16/U32/Bool/Str/Nil).
    const ops = ["sub", "mul", "div", "mod", "and", "or", "xor", "shl", "shr",
      "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge"];
    const binInstrs = ops.flatMap((op, i) => [
      IirInstr.of("const", { dest: "x", srcs: [4], typeHint: Types.U8 }),
      IirInstr.of("const", { dest: "y", srcs: [2], typeHint: Types.U8 }),
      IirInstr.of(op, { dest: `r${i}`, srcs: ["x", "y"], typeHint: Types.U8 }),
    ]);
    const casts = [Types.U8, Types.U16, Types.U32, Types.Bool, Types.Str, Types.Nil];
    const castInstrs = casts.flatMap((t, i) => [
      IirInstr.of("const", { dest: "c", srcs: [300], typeHint: Types.U16 }),
      IirInstr.of("cast", { dest: `c${i}`, srcs: ["c"], typeHint: t }),
    ]);
    const main = new IirFunction({ name: "main", instructions: [
      IirInstr.of("const", { dest: "n", srcs: [3], typeHint: Types.U8 }),
      IirInstr.of("neg", { dest: "ng", srcs: ["n"], typeHint: Types.U8 }),
      IirInstr.of("not", { dest: "nt", srcs: ["n"], typeHint: Types.U8 }),
      ...binInstrs,
      ...castInstrs,
      // jmp_if_false (taken) jumps past a noop builtin call; fall-through tested in earlier test.
      IirInstr.of("jmp_if_false", { srcs: [false, "after"] }),
      IirInstr.of("call_builtin", { dest: "_", srcs: ["noop"] }),
      IirInstr.of("label", { srcs: ["after"] }),
      IirInstr.of("load_reg", { dest: "lr", srcs: ["n"], typeHint: Types.U8 }),
      IirInstr.of("store_reg", { srcs: ["lr", "n"] }),       // dest = null branch
      IirInstr.of("store_reg", { dest: "lr2", srcs: ["n"] }), // dest != null branch
      IirInstr.of("is_null", { dest: "isn", srcs: ["n"], typeHint: Types.Bool }),
      IirInstr.of("safepoint", { srcs: [] }),
      // Exercise toNumber's boolean+string branches via cast.
      IirInstr.of("const", { dest: "tb", srcs: [true], typeHint: Types.Bool }),
      IirInstr.of("cast", { dest: "tbn", srcs: ["tb"], typeHint: Types.U8 }),
      IirInstr.of("const", { dest: "ts", srcs: ["42"], typeHint: Types.Str }),
      IirInstr.of("cast", { dest: "tsn", srcs: ["ts"], typeHint: Types.U8 }),
      // type_assert + runtimeType branches.
      IirInstr.of("type_assert", { srcs: ["isn"], typeHint: Types.Bool }),
      IirInstr.of("ret_void", { srcs: [] }),
    ] });
    vm.execute(new IirModule({ name: "m", functions: [main] }));

    // Type-assert mismatch path.
    const bad = new IirFunction({ name: "main", instructions: [
      IirInstr.of("const", { dest: "n", srcs: [3], typeHint: Types.U8 }),
      IirInstr.of("type_assert", { srcs: ["n"], typeHint: Types.Bool }),
      IirInstr.of("ret_void", { srcs: [] }),
    ] });
    expect(() => new VMCore().execute(new IirModule({ name: "bad", functions: [bad] }))).toThrow(VMError);

    // toNumber failure path.
    const obj = new IirFunction({ name: "main", instructions: [
      IirInstr.of("const", { dest: "x", srcs: [{ k: 1 }], typeHint: Types.Any }),
      IirInstr.of("neg", { dest: "y", srcs: ["x"], typeHint: Types.U8 }),
      IirInstr.of("ret_void", { srcs: [] }),
    ] });
    expect(() => new VMCore().execute(new IirModule({ name: "obj", functions: [obj] }))).toThrow(VMError);

    // Unknown opcode triggers default branch in dispatch + binaryOp.
    const unk = new IirFunction({ name: "main", instructions: [
      IirInstr.of("__nope__" as never, { dest: "x" }),
    ] });
    expect(() => new VMCore().execute(new IirModule({ name: "u", functions: [unk] }))).toThrow(UnknownOpcodeError);

    // jumpTarget undefined-label path — IirModule.validate() catches the
    // forward reference before runtime, so any error type is acceptable here;
    // the assertion just exercises the validation guard.
    const jl = new IirFunction({ name: "main", instructions: [
      IirInstr.of("jmp", { srcs: ["missing"] }),
    ] });
    expect(() => new VMCore().execute(new IirModule({ name: "j", functions: [jl] }))).toThrow();
  });
});
