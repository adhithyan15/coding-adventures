import { describe, expect, it } from "vitest";
import { IrProgram, IrOp, imm, lbl } from "@coding-adventures/compiler-ir";

import { validate, VERSION, WasmIrValidator } from "../src/index.js";

describe("validate", () => {
  it("reports lowering errors for unsupported syscalls", () => {
    const program = new IrProgram("_start");
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    program.addInstruction({ opcode: IrOp.SYSCALL, operands: [imm(999)], id: 1 });

    const errors = validate(program);

    expect(errors).toEqual([
      {
        rule: "lowering",
        message: "unsupported SYSCALL number(s): 999",
      },
    ]);
  });

  it("returns no errors for a well-formed program", () => {
    const program = new IrProgram("_start");
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    program.addInstruction({ opcode: IrOp.HALT, operands: [], id: 1 });
    expect(validate(program)).toEqual([]);
  });

  it("exposes a VERSION string", () => {
    expect(typeof VERSION).toBe("string");
    expect(VERSION).toMatch(/^\d+\.\d+\.\d+/);
  });

  it("WasmIrValidator can be instantiated directly", () => {
    const v = new WasmIrValidator();
    const program = new IrProgram("_start");
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    program.addInstruction({ opcode: IrOp.HALT, operands: [], id: 1 });
    expect(v.validate(program)).toEqual([]);
  });
});
