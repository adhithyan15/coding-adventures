import { IrOp, IrProgram, type IrInstruction, type IrProgram as IrProgramType } from "@coding-adventures/compiler-ir";

import type { IrPass } from "../protocol.js";

const UNCONDITIONAL_BRANCHES = new Set([IrOp.JUMP, IrOp.RET, IrOp.HALT]);

function cloneProgram(program: IrProgramType, instructions: IrInstruction[]): IrProgram {
  const next = new IrProgram(program.entryLabel);
  next.version = program.version;
  next.instructions = instructions.map((instruction) => ({
    opcode: instruction.opcode,
    operands: [...instruction.operands],
    id: instruction.id,
  }));
  next.data = [...program.data];
  return next;
}

export class DeadCodeEliminator implements IrPass {
  readonly name = "DeadCodeEliminator";

  run(program: IrProgram): IrProgram {
    const live: IrInstruction[] = [];
    let reachable = true;

    for (const instruction of program.instructions) {
      if (instruction.opcode === IrOp.LABEL) {
        reachable = true;
      }

      if (reachable) {
        live.push(instruction);
      }

      if (UNCONDITIONAL_BRANCHES.has(instruction.opcode)) {
        reachable = false;
      }
    }

    return cloneProgram(program, live);
  }
}
