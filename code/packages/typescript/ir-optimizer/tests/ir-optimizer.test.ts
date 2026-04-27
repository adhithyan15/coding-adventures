import { IrOp, IrProgram, imm, lbl, reg } from "@coding-adventures/compiler-ir";
import { describe, expect, it } from "vitest";

import { ConstantFolder, DeadCodeEliminator, IrOptimizer, PeepholeOptimizer } from "../src/index.js";

function makeProgram() {
  const program = new IrProgram("_start");
  program.addInstruction({ opcode: IrOp.LOAD_IMM, operands: [reg(1), imm(5)], id: 1 });
  program.addInstruction({ opcode: IrOp.ADD_IMM, operands: [reg(1), reg(1), imm(3)], id: 2 });
  program.addInstruction({ opcode: IrOp.JUMP, operands: [lbl("done")], id: 3 });
  program.addInstruction({ opcode: IrOp.ADD_IMM, operands: [reg(1), reg(1), imm(1)], id: 4 });
  program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("done")], id: 5 });
  program.addInstruction({ opcode: IrOp.HALT, operands: [], id: 6 });
  return program;
}

describe("optimizer passes", () => {
  it("folds load_imm plus add_imm", () => {
    const program = new IrProgram("_start");
    program.addInstruction({ opcode: IrOp.LOAD_IMM, operands: [reg(1), imm(5)], id: 1 });
    program.addInstruction({ opcode: IrOp.ADD_IMM, operands: [reg(1), reg(1), imm(3)], id: 2 });
    const result = new ConstantFolder().run(program);
    expect(result.instructions[0].opcode).toBe(IrOp.LOAD_IMM);
    expect(result.instructions[0].operands[1]).toEqual(imm(8));
  });

  it("removes code after unconditional jumps", () => {
    const result = new DeadCodeEliminator().run(makeProgram());
    expect(result.instructions.map((instruction) => instruction.id)).toEqual([1, 2, 3, 5, 6]);
  });

  it("merges consecutive add_imm operations", () => {
    const program = new IrProgram("_start");
    program.addInstruction({ opcode: IrOp.ADD_IMM, operands: [reg(2), reg(2), imm(2)], id: 1 });
    program.addInstruction({ opcode: IrOp.ADD_IMM, operands: [reg(2), reg(2), imm(3)], id: 2 });
    const result = new PeepholeOptimizer().run(program);
    expect(result.instructions).toHaveLength(1);
    expect(result.instructions[0].operands[2]).toEqual(imm(5));
  });

  it("runs the default optimization pipeline", () => {
    const result = IrOptimizer.defaultPasses().optimize(makeProgram());
    expect(result.passesRun).toEqual([
      "DeadCodeEliminator",
      "ConstantFolder",
      "PeepholeOptimizer",
    ]);
    expect(result.instructionsEliminated).toBeGreaterThan(0);
  });
});
