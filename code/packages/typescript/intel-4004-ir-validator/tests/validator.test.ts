import { IrOp, IrProgram, imm, lbl, reg } from "@coding-adventures/compiler-ir";
import { describe, expect, it } from "vitest";

import { IrValidator } from "../src/index.js";

describe("intel-4004-ir-validator", () => {
  it("accepts a small feasible program", () => {
    const program = new IrProgram("_start");
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    program.addInstruction({ opcode: IrOp.LOAD_IMM, operands: [reg(0), imm(0)], id: 1 });
    program.addInstruction({ opcode: IrOp.HALT, operands: [], id: 2 });
    expect(new IrValidator().validate(program)).toEqual([]);
  });

  it("rejects recursive call graphs", () => {
    const program = new IrProgram("_start");
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_fn_a")], id: -1 });
    program.addInstruction({ opcode: IrOp.CALL, operands: [lbl("_fn_b")], id: 1 });
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_fn_b")], id: -1 });
    program.addInstruction({ opcode: IrOp.CALL, operands: [lbl("_fn_a")], id: 2 });
    const errors = new IrValidator().validate(program);
    expect(errors.some((error) => error.rule === "call_depth")).toBe(true);
  });

  it("rejects excessive static RAM", () => {
    const program = new IrProgram("_start");
    program.data = [{ label: "huge", size: 200, init: 0 }];
    const errors = new IrValidator().validate(program);
    expect(errors.some((error) => error.rule === "static_ram")).toBe(true);
  });
});
