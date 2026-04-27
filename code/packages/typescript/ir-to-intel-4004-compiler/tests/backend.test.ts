import { IrOp, IrProgram, imm, lbl, reg } from "@coding-adventures/compiler-ir";
import { describe, expect, it } from "vitest";

import { IrToIntel4004Compiler } from "../src/index.js";

describe("ir-to-intel-4004-compiler", () => {
  it("emits assembly for a small program", () => {
    const program = new IrProgram("_start");
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    program.addInstruction({ opcode: IrOp.LOAD_IMM, operands: [reg(1), imm(5)], id: 1 });
    program.addInstruction({ opcode: IrOp.HALT, operands: [], id: 2 });

    const assembly = new IrToIntel4004Compiler().compile(program);
    expect(assembly).toContain("ORG 0x000");
    expect(assembly).toContain("_start:");
    expect(assembly).toContain("LDM 5");
    expect(assembly).toContain("HLT");
  });

  it("fails validation before codegen", () => {
    const program = new IrProgram("_start");
    program.addInstruction({ opcode: IrOp.LOAD_WORD, operands: [reg(1), reg(2), reg(3)], id: 1 });
    expect(() => new IrToIntel4004Compiler().compile(program)).toThrow(/no_word_ops/);
  });
});
