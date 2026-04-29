import { describe, expect, it } from "vitest";
import { FunctionTypeStatus, IirFunction, IirInstr, IirModule, SlotKind, SlotState, Types } from "../src/index.js";

describe("interpreter-ir", () => {
  it("records feedback", () => {
    const slot = new SlotState();
    const instr = IirInstr.of("add", { dest: "r0", srcs: ["a", "b"], typeHint: Types.U8 });
    instr.recordObservation("u8", slot).recordObservation("bool", slot);
    expect(instr.typed).toBe(true);
    expect(instr.polymorphic).toBe(true);
    expect(slot.kind).toBe(SlotKind.Polymorphic);
  });

  it("infers function type status", () => {
    const fn = new IirFunction({
      name: "add",
      params: [{ name: "a", type: Types.U8 }],
      returnType: Types.U8,
      instructions: [IirInstr.of("add", { dest: "r", srcs: ["a", 1], typeHint: Types.U8 })],
    });
    expect(fn.typeStatus).toBe(FunctionTypeStatus.FullyTyped);
    expect(fn.paramNames()).toEqual(["a"]);
    expect(fn.paramTypes()).toEqual(["u8"]);
  });

  it("validates branch labels", () => {
    const ok = new IirModule({
      name: "ok",
      functions: [new IirFunction({ name: "main", instructions: [IirInstr.of("label", { srcs: ["x"] }), IirInstr.of("jmp", { srcs: ["x"] })] })],
    });
    expect(() => ok.validate()).not.toThrow();
    const bad = new IirModule({ name: "bad", functions: [new IirFunction({ name: "main", instructions: [IirInstr.of("jmp", { srcs: ["missing"] })] })] });
    expect(() => bad.validate()).toThrow(/undefined label missing/);
  });
});
