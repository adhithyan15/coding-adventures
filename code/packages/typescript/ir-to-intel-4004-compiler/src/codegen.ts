import { IrOp, type IrInstruction, type IrImmediate, type IrLabel, type IrProgram, type IrRegister } from "@coding-adventures/compiler-ir";

const INDENT = "    ";
const VREG_TO_PREG = new Map<number, string>(Array.from({ length: 13 }, (_, index) => [index, `R${index}`]));

function preg(index: number): string {
  return VREG_TO_PREG.get(index) ?? `R${index}`;
}

function pair(index: number): string {
  return `P${Math.floor(index / 2)}`;
}

function isRegisterOperand(operand: unknown): operand is IrRegister {
  return Boolean(operand) && typeof operand === "object" && (operand as IrRegister).kind === "register";
}

function isImmediateOperand(operand: unknown): operand is IrImmediate {
  return Boolean(operand) && typeof operand === "object" && (operand as IrImmediate).kind === "immediate";
}

function isLabelOperand(operand: unknown): operand is IrLabel {
  return Boolean(operand) && typeof operand === "object" && (operand as IrLabel).kind === "label";
}

export class CodeGenerator {
  generate(program: IrProgram): string {
    const lines: string[] = [`${INDENT}ORG 0x000`];
    for (const instruction of program.instructions) {
      lines.push(...this.emit(instruction));
    }
    return `${lines.join("\n")}\n`;
  }

  private emit(instruction: IrInstruction): string[] {
    switch (instruction.opcode) {
      case IrOp.LABEL:
        return this.emitLabel(instruction.operands);
      case IrOp.LOAD_IMM:
        return this.emitLoadImm(instruction.operands);
      case IrOp.LOAD_ADDR:
        return this.emitLoadAddr(instruction.operands);
      case IrOp.LOAD_BYTE:
        return this.emitLoadByte(instruction.operands);
      case IrOp.STORE_BYTE:
        return this.emitStoreByte(instruction.operands);
      case IrOp.ADD:
        return this.emitThreeRegister(instruction.operands, "ADD");
      case IrOp.SUB:
        return this.emitThreeRegister(instruction.operands, "SUB");
      case IrOp.AND:
        return this.emitThreeRegister(instruction.operands, "AND");
      case IrOp.ADD_IMM:
        return this.emitAddImm(instruction.operands);
      case IrOp.AND_IMM:
        return this.emitAndImm(instruction.operands);
      case IrOp.CMP_EQ:
        return this.emitCmpEq(instruction.operands);
      case IrOp.CMP_LT:
        return this.emitCmpLt(instruction.operands);
      case IrOp.CMP_NE:
      case IrOp.CMP_GT:
        return [`${INDENT}; ${IrOp[instruction.opcode]} - no direct 4004 equivalent`];
      case IrOp.JUMP:
        return isLabelOperand(instruction.operands[0]) ? [`${INDENT}JUN ${instruction.operands[0].name}`] : [];
      case IrOp.BRANCH_Z:
        return this.emitBranch(instruction.operands, "0x4");
      case IrOp.BRANCH_NZ:
        return this.emitBranch(instruction.operands, "0xC");
      case IrOp.CALL:
        return isLabelOperand(instruction.operands[0]) ? [`${INDENT}JMS ${instruction.operands[0].name}`] : [];
      case IrOp.RET:
        return [`${INDENT}BBL 0`];
      case IrOp.HALT:
        return [`${INDENT}HLT`];
      case IrOp.NOP:
        return [`${INDENT}NOP`];
      case IrOp.COMMENT:
        return isLabelOperand(instruction.operands[0]) ? [`${INDENT}; ${instruction.operands[0].name}`] : [`${INDENT};`];
      case IrOp.SYSCALL:
        return [`${INDENT}; syscall not supported on 4004`];
      default:
        return [`${INDENT}; unsupported opcode ${IrOp[instruction.opcode]}`];
    }
  }

  private emitLabel(operands: readonly unknown[]): string[] {
    return isLabelOperand(operands[0]) ? [`${operands[0].name}:`] : [];
  }

  private emitLoadImm(operands: readonly unknown[]): string[] {
    if (!isRegisterOperand(operands[0]) || !isImmediateOperand(operands[1])) {
      return [`${INDENT}; LOAD_IMM: invalid operands`];
    }
    const registerName = preg(operands[0].index);
    const registerPair = pair(operands[0].index);
    if (operands[1].value <= 15) {
      return [`${INDENT}LDM ${operands[1].value}`, `${INDENT}XCH ${registerName}`];
    }
    return [`${INDENT}FIM ${registerPair}, ${operands[1].value}`];
  }

  private emitLoadAddr(operands: readonly unknown[]): string[] {
    if (!isRegisterOperand(operands[0]) || !isLabelOperand(operands[1])) {
      return [`${INDENT}; LOAD_ADDR: invalid operands`];
    }
    return [`${INDENT}FIM ${pair(operands[0].index)}, ${operands[1].name}`];
  }

  private emitLoadByte(operands: readonly unknown[]): string[] {
    if (!isRegisterOperand(operands[0]) || !isRegisterOperand(operands[1])) {
      return [`${INDENT}; LOAD_BYTE: invalid operands`];
    }
    return [
      `${INDENT}SRC ${pair(operands[1].index)}`,
      `${INDENT}RDM`,
      `${INDENT}XCH ${preg(operands[0].index)}`,
    ];
  }

  private emitStoreByte(operands: readonly unknown[]): string[] {
    if (!isRegisterOperand(operands[0]) || !isRegisterOperand(operands[1])) {
      return [`${INDENT}; STORE_BYTE: invalid operands`];
    }
    return [
      `${INDENT}LD ${preg(operands[0].index)}`,
      `${INDENT}SRC ${pair(operands[1].index)}`,
      `${INDENT}WRM`,
    ];
  }

  private emitThreeRegister(operands: readonly unknown[], mnemonic: string): string[] {
    if (!isRegisterOperand(operands[0]) || !isRegisterOperand(operands[1]) || !isRegisterOperand(operands[2])) {
      return [`${INDENT}; ${mnemonic}: invalid operands`];
    }
    const target = preg(operands[0].index);
    const left = preg(operands[1].index);
    const right = preg(operands[2].index);
    return [`${INDENT}LD ${left}`, `${INDENT}${mnemonic} ${right}`, `${INDENT}XCH ${target}`];
  }

  private emitAddImm(operands: readonly unknown[]): string[] {
    if (!isRegisterOperand(operands[0]) || !isRegisterOperand(operands[1]) || !isImmediateOperand(operands[2])) {
      return [`${INDENT}; ADD_IMM: invalid operands`];
    }
    const destination = preg(operands[0].index);
    const source = preg(operands[1].index);
    const immediate = operands[2].value;
    if (immediate === 0) {
      return [`${INDENT}LD ${source}`, `${INDENT}XCH ${destination}`];
    }
    if (immediate <= 15) {
      const scratch = operands[1].index === 1 ? "R14" : "R1";
      return [
        `${INDENT}LDM ${immediate}`,
        `${INDENT}XCH ${scratch}`,
        `${INDENT}LD ${source}`,
        `${INDENT}ADD ${scratch}`,
        `${INDENT}XCH ${destination}`,
      ];
    }
    return [
      `${INDENT}FIM P7, ${immediate}`,
      `${INDENT}LD ${source}`,
      `${INDENT}ADD R14`,
      `${INDENT}XCH ${destination}`,
    ];
  }

  private emitAndImm(operands: readonly unknown[]): string[] {
    if (!isRegisterOperand(operands[0]) || !isRegisterOperand(operands[1]) || !isImmediateOperand(operands[2])) {
      return [`${INDENT}; AND_IMM: invalid operands`];
    }
    const mask = operands[2].value;
    if (mask === 255) {
      return [`${INDENT}; AND_IMM 255 is a no-op on 4004 (8-bit pair)`];
    }
    if (mask === 15) {
      return [`${INDENT}; AND_IMM 15 is a no-op on 4004 (4-bit register)`];
    }
    return [`${INDENT}; AND_IMM ${mask} is unsupported on 4004`];
  }

  private emitCmpLt(operands: readonly unknown[]): string[] {
    if (!isRegisterOperand(operands[0]) || !isRegisterOperand(operands[1]) || !isRegisterOperand(operands[2])) {
      return [`${INDENT}; CMP_LT: invalid operands`];
    }
    return [
      `${INDENT}LD ${preg(operands[1].index)}`,
      `${INDENT}SUB ${preg(operands[2].index)}`,
      `${INDENT}TCS`,
      `${INDENT}XCH ${preg(operands[0].index)}`,
    ];
  }

  private emitCmpEq(operands: readonly unknown[]): string[] {
    if (!isRegisterOperand(operands[0]) || !isRegisterOperand(operands[1]) || !isRegisterOperand(operands[2])) {
      return [`${INDENT}; CMP_EQ: invalid operands`];
    }
    return [
      `${INDENT}LD ${preg(operands[1].index)}`,
      `${INDENT}SUB ${preg(operands[2].index)}`,
      `${INDENT}CMA`,
      `${INDENT}IAC`,
      `${INDENT}XCH ${preg(operands[0].index)}`,
    ];
  }

  private emitBranch(operands: readonly unknown[], condition: string): string[] {
    if (!isRegisterOperand(operands[0]) || !isLabelOperand(operands[1])) {
      return [`${INDENT}; BRANCH: invalid operands`];
    }
    return [
      `${INDENT}LD ${preg(operands[0].index)}`,
      `${INDENT}JCN ${condition}, ${operands[1].name}`,
    ];
  }
}
