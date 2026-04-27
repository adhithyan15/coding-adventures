import {
  IrOp,
  IrProgram,
  type IrInstruction,
  imm,
  reg,
} from "@coding-adventures/compiler-ir";

import type { IrPass } from "../protocol.js";

const MAX_ITERATIONS = 10;

function isRegisterOperand(operand: IrInstruction["operands"][number] | undefined): operand is ReturnType<typeof reg> {
  return operand?.kind === "register";
}

function isImmediateOperand(operand: IrInstruction["operands"][number] | undefined): operand is ReturnType<typeof imm> {
  return operand?.kind === "immediate";
}

function cloneProgram(program: IrProgram, instructions: IrInstruction[]): IrProgram {
  const next = new IrProgram(program.entryLabel);
  next.version = program.version;
  next.instructions = instructions;
  next.data = [...program.data];
  return next;
}

export class PeepholeOptimizer implements IrPass {
  readonly name = "PeepholeOptimizer";

  run(program: IrProgram): IrProgram {
    let instructions = [...program.instructions];
    for (let iteration = 0; iteration < MAX_ITERATIONS; iteration += 1) {
      const next = this.applyPatterns(instructions);
      if (next.length === instructions.length) {
        instructions = next;
        break;
      }
      instructions = next;
    }
    return cloneProgram(program, instructions);
  }

  private applyPatterns(instructions: IrInstruction[]): IrInstruction[] {
    const out: IrInstruction[] = [];
    for (let index = 0; index < instructions.length; ) {
      if (index + 1 < instructions.length) {
        const merged = this.tryMerge(instructions[index], instructions[index + 1]);
        if (merged) {
          out.push(merged);
          index += 2;
          continue;
        }
      }
      out.push({
        opcode: instructions[index].opcode,
        operands: [...instructions[index].operands],
        id: instructions[index].id,
      });
      index += 1;
    }
    return out;
  }

  private tryMerge(current: IrInstruction, next: IrInstruction): IrInstruction | null {
    if (
      current.opcode === IrOp.ADD_IMM &&
      next.opcode === IrOp.ADD_IMM &&
      current.operands.length === 3 &&
      next.operands.length === 3
    ) {
      const cDest = current.operands[0];
      const cSrc = current.operands[1];
      const cImm = current.operands[2];
      const nDest = next.operands[0];
      const nSrc = next.operands[1];
      const nImm = next.operands[2];
      if (
        isRegisterOperand(cDest) &&
        isRegisterOperand(cSrc) &&
        isImmediateOperand(cImm) &&
        isRegisterOperand(nDest) &&
        isRegisterOperand(nSrc) &&
        isImmediateOperand(nImm) &&
        cDest.index === cSrc.index &&
        nDest.index === nSrc.index &&
        cDest.index === nDest.index
      ) {
        return {
          opcode: IrOp.ADD_IMM,
          operands: [reg(cDest.index), reg(cSrc.index), imm(cImm.value + nImm.value)],
          id: current.id,
        };
      }
    }

    if (
      next.opcode === IrOp.AND_IMM &&
      next.operands.length === 3 &&
      isRegisterOperand(next.operands[0]) &&
      isRegisterOperand(next.operands[1]) &&
      isImmediateOperand(next.operands[2]) &&
      next.operands[0].index === next.operands[1].index &&
      next.operands[2].value === 255
    ) {
      const target = next.operands[0];
      if (
        (current.opcode === IrOp.ADD_IMM || current.opcode === IrOp.LOAD_IMM) &&
        current.operands.length >= 2 &&
        isRegisterOperand(current.operands[0]) &&
        current.operands[0].index === target.index
      ) {
        const immediate = current.operands[current.operands.length - 1];
        if (isImmediateOperand(immediate) && immediate.value >= 0 && immediate.value <= 255) {
          return {
            opcode: current.opcode,
            operands: [...current.operands],
            id: current.id,
          };
        }
      }
    }

    if (
      current.opcode === IrOp.LOAD_IMM &&
      next.opcode === IrOp.ADD_IMM &&
      current.operands.length === 2 &&
      next.operands.length === 3
    ) {
      const cDest = current.operands[0];
      const cImm = current.operands[1];
      const nDest = next.operands[0];
      const nSrc = next.operands[1];
      const nImm = next.operands[2];
      if (
        isRegisterOperand(cDest) &&
        isImmediateOperand(cImm) &&
        isRegisterOperand(nDest) &&
        isRegisterOperand(nSrc) &&
        isImmediateOperand(nImm) &&
        cImm.value === 0 &&
        cDest.index === nDest.index &&
        nDest.index === nSrc.index
      ) {
        return {
          opcode: IrOp.LOAD_IMM,
          operands: [reg(cDest.index), imm(nImm.value)],
          id: current.id,
        };
      }
    }

    return null;
  }
}
