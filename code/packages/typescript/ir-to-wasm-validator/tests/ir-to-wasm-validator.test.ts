import { describe, expect, it } from "vitest";
import { IrProgram, IrOp, imm, lbl } from "@coding-adventures/compiler-ir";

import { validate } from "../src/index.js";

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
});
