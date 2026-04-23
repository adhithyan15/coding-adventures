import {
  IrOp,
  IrProgram,
  type IrInstruction,
  type IrOperand,
  imm,
  reg,
} from "@coding-adventures/compiler-ir";

import type { IrPass } from "../protocol.js";

const FOLDABLE_IMM_OPS = new Set([IrOp.ADD_IMM, IrOp.AND_IMM]);
const WRITES_TO_DEST = new Set([
  IrOp.LOAD_IMM,
  IrOp.LOAD_ADDR,
  IrOp.LOAD_BYTE,
  IrOp.LOAD_WORD,
  IrOp.ADD,
  IrOp.ADD_IMM,
  IrOp.SUB,
  IrOp.AND,
  IrOp.AND_IMM,
  IrOp.CMP_EQ,
  IrOp.CMP_NE,
  IrOp.CMP_LT,
  IrOp.CMP_GT,
]);

function cloneInstruction(
  opcode: IrOp,
  operands: readonly IrOperand[],
  id: number,
): IrInstruction {
  return {
    opcode,
    operands: [...operands],
    id,
  };
}

function cloneProgram(program: IrProgram, instructions: IrInstruction[]): IrProgram {
  const next = new IrProgram(program.entryLabel);
  next.version = program.version;
  next.instructions = instructions;
  next.data = [...program.data];
  return next;
}

function isRegisterOperand(operand: IrOperand | undefined): operand is ReturnType<typeof reg> {
  return operand?.kind === "register";
}

function isImmediateOperand(operand: IrOperand | undefined): operand is ReturnType<typeof imm> {
  return operand?.kind === "immediate";
}

export class ConstantFolder implements IrPass {
  readonly name = "ConstantFolder";

  run(program: IrProgram): IrProgram {
    const pendingLoad = new Map<number, number>();
    const out: IrInstruction[] = [];

    for (const instruction of program.instructions) {
      if (instruction.opcode === IrOp.LOAD_IMM) {
        const dest = instruction.operands[0];
        const immediate = instruction.operands[1];
        if (isRegisterOperand(dest) && isImmediateOperand(immediate)) {
          pendingLoad.set(dest.index, immediate.value);
        }
        out.push(cloneInstruction(instruction.opcode, instruction.operands, instruction.id));
        continue;
      }

      if (FOLDABLE_IMM_OPS.has(instruction.opcode)) {
        const dest = instruction.operands[0];
        const src = instruction.operands[1];
        const immediate = instruction.operands[2];

        if (
          isRegisterOperand(dest) &&
          isRegisterOperand(src) &&
          isImmediateOperand(immediate) &&
          dest.index === src.index &&
          pendingLoad.has(dest.index)
        ) {
          const base = pendingLoad.get(dest.index) ?? 0;
          const newValue =
            instruction.opcode === IrOp.ADD_IMM ? base + immediate.value : (base & immediate.value);

          for (let index = out.length - 1; index >= 0; index -= 1) {
            const previous = out[index];
            const previousDest = previous.operands[0];
            if (
              previous.opcode === IrOp.LOAD_IMM &&
              isRegisterOperand(previousDest) &&
              previousDest.index === dest.index
            ) {
              out[index] = {
                opcode: IrOp.LOAD_IMM,
                operands: [reg(dest.index), imm(newValue)],
                id: previous.id,
              };
              break;
            }
          }

          pendingLoad.set(dest.index, newValue);
          continue;
        }
      }

      if (WRITES_TO_DEST.has(instruction.opcode)) {
        const dest = instruction.operands[0];
        if (isRegisterOperand(dest)) {
          pendingLoad.delete(dest.index);
        }
      }

      out.push(cloneInstruction(instruction.opcode, instruction.operands, instruction.id));
    }

    return cloneProgram(program, out);
  }
}
