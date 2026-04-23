import { IrOp, type IrInstruction, type IrLabel, type IrProgram, type IrRegister } from "@coding-adventures/compiler-ir";

const MAX_RAM_BYTES = 160;
const MAX_CALL_DEPTH = 2;
const MAX_VIRTUAL_REGISTERS = 12;
const MIN_LOAD_IMM = 0;
const MAX_LOAD_IMM = 255;

export class IrValidationError extends Error {
  constructor(
    readonly rule: string,
    readonly message: string,
  ) {
    super(message);
  }

  override toString(): string {
    return `[${this.rule}] ${this.message}`;
  }
}

function isLabelOperand(operand: unknown): operand is IrLabel {
  return Boolean(operand) && typeof operand === "object" && (operand as IrLabel).kind === "label";
}

function isRegisterOperand(operand: unknown): operand is IrRegister {
  return Boolean(operand) && typeof operand === "object" && (operand as IrRegister).kind === "register";
}

export class IrValidator {
  validate(program: IrProgram): IrValidationError[] {
    return [
      ...this.checkNoWordOps(program),
      ...this.checkStaticRam(program),
      ...this.checkCallDepth(program),
      ...this.checkRegisterCount(program),
      ...this.checkOperandRange(program),
    ];
  }

  private checkNoWordOps(program: IrProgram): IrValidationError[] {
    const errors: IrValidationError[] = [];
    let sawLoadWord = false;
    let sawStoreWord = false;

    for (const instruction of program.instructions) {
      if (instruction.opcode === IrOp.LOAD_WORD && !sawLoadWord) {
        errors.push(
          new IrValidationError(
            "no_word_ops",
            "LOAD_WORD is not supported on Intel 4004. Replace it with byte-sized accesses.",
          ),
        );
        sawLoadWord = true;
      } else if (instruction.opcode === IrOp.STORE_WORD && !sawStoreWord) {
        errors.push(
          new IrValidationError(
            "no_word_ops",
            "STORE_WORD is not supported on Intel 4004. Replace it with byte-sized accesses.",
          ),
        );
        sawStoreWord = true;
      }
    }

    return errors;
  }

  private checkStaticRam(program: IrProgram): IrValidationError[] {
    const total = program.data.reduce((sum, decl) => sum + decl.size, 0);
    if (total <= MAX_RAM_BYTES) {
      return [];
    }
    return [
      new IrValidationError(
        "static_ram",
        `Static RAM usage ${total} bytes exceeds the Intel 4004 limit of ${MAX_RAM_BYTES} bytes.`,
      ),
    ];
  }

  private checkCallDepth(program: IrProgram): IrValidationError[] {
    const callGraph = new Map<string, string[]>();
    let currentLabel: string | null = null;

    for (const instruction of program.instructions) {
      if (instruction.opcode === IrOp.LABEL && isLabelOperand(instruction.operands[0])) {
        currentLabel = instruction.operands[0].name;
        if (!callGraph.has(currentLabel)) {
          callGraph.set(currentLabel, []);
        }
      } else if (instruction.opcode === IrOp.CALL && currentLabel && isLabelOperand(instruction.operands[0])) {
        const callee = instruction.operands[0].name;
        callGraph.set(currentLabel, [...(callGraph.get(currentLabel) ?? []), callee]);
        if (!callGraph.has(callee)) {
          callGraph.set(callee, []);
        }
      }
    }

    const cycle = this.findCycle(callGraph);
    if (cycle) {
      return [
        new IrValidationError(
          "call_depth",
          `Recursive call graphs are not supported on Intel 4004. Found cycle: ${cycle.join(" -> ")}.`,
        ),
      ];
    }

    let maxDepth = 0;
    const depthFirst = (node: string, depth: number, visited: Set<string>): number => {
      if (visited.has(node)) {
        return depth;
      }
      const nextVisited = new Set(visited);
      nextVisited.add(node);
      const children = callGraph.get(node) ?? [];
      if (children.length === 0) {
        return depth;
      }
      return Math.max(...children.map((child) => depthFirst(child, depth + 1, nextVisited)));
    };

    for (const label of callGraph.keys()) {
      maxDepth = Math.max(maxDepth, depthFirst(label, 0, new Set()));
    }

    if (maxDepth > MAX_CALL_DEPTH) {
      return [
        new IrValidationError(
          "call_depth",
          `Call graph depth ${maxDepth} exceeds the Intel 4004 hardware stack limit of ${MAX_CALL_DEPTH} nested calls.`,
        ),
      ];
    }

    return [];
  }

  private findCycle(callGraph: ReadonlyMap<string, readonly string[]>): string[] | null {
    const visiting = new Set<string>();
    const visited = new Set<string>();
    const path: string[] = [];

    const visit = (node: string): string[] | null => {
      if (visiting.has(node)) {
        const start = path.indexOf(node);
        return start >= 0 ? [...path.slice(start), node] : [node, node];
      }
      if (visited.has(node)) {
        return null;
      }

      visiting.add(node);
      visited.add(node);
      path.push(node);
      for (const child of callGraph.get(node) ?? []) {
        const cycle = visit(child);
        if (cycle) {
          return cycle;
        }
      }
      path.pop();
      visiting.delete(node);
      return null;
    };

    for (const node of callGraph.keys()) {
      const cycle = visit(node);
      if (cycle) {
        return cycle;
      }
    }
    return null;
  }

  private checkRegisterCount(program: IrProgram): IrValidationError[] {
    const registers = new Set<number>();
    for (const instruction of program.instructions) {
      for (const operand of instruction.operands) {
        if (isRegisterOperand(operand)) {
          registers.add(operand.index);
        }
      }
    }
    if (registers.size <= MAX_VIRTUAL_REGISTERS) {
      return [];
    }
    return [
      new IrValidationError(
        "register_count",
        `Program uses ${registers.size} distinct virtual registers but Intel 4004 supports at most ${MAX_VIRTUAL_REGISTERS}.`,
      ),
    ];
  }

  private checkOperandRange(program: IrProgram): IrValidationError[] {
    const errors: IrValidationError[] = [];
    for (const instruction of program.instructions) {
      if (instruction.opcode !== IrOp.LOAD_IMM || instruction.operands.length < 2) {
        continue;
      }
      const operand = instruction.operands[1];
      if (!operand || typeof operand !== "object" || (operand as { kind?: string }).kind !== "immediate") {
        continue;
      }
      const value = (operand as { value: number }).value;
      if (value < MIN_LOAD_IMM || value > MAX_LOAD_IMM) {
        errors.push(
          new IrValidationError(
            "operand_range",
            `LOAD_IMM immediate ${value} is out of range for Intel 4004. Valid range is [${MIN_LOAD_IMM}, ${MAX_LOAD_IMM}].`,
          ),
        );
      }
    }
    return errors;
  }
}
