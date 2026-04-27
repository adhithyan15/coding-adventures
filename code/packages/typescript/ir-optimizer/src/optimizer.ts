import type { IrProgram } from "@coding-adventures/compiler-ir";

import { ConstantFolder, DeadCodeEliminator, PeepholeOptimizer } from "./passes/index.js";
import type { IrPass } from "./protocol.js";

export interface OptimizationResult {
  readonly program: IrProgram;
  readonly passesRun: readonly string[];
  readonly instructionsBefore: number;
  readonly instructionsAfter: number;
  readonly instructionsEliminated: number;
}

export class IrOptimizer {
  constructor(private readonly passes: readonly IrPass[]) {}

  optimize(program: IrProgram): OptimizationResult {
    const instructionsBefore = program.instructions.length;
    const passesRun: string[] = [];

    let current = program;
    for (const pass of this.passes) {
      current = pass.run(current);
      passesRun.push(pass.name);
    }

    const instructionsAfter = current.instructions.length;
    return {
      program: current,
      passesRun,
      instructionsBefore,
      instructionsAfter,
      instructionsEliminated: instructionsBefore - instructionsAfter,
    };
  }

  static defaultPasses(): IrOptimizer {
    return new IrOptimizer([
      new DeadCodeEliminator(),
      new ConstantFolder(),
      new PeepholeOptimizer(),
    ]);
  }

  static noOp(): IrOptimizer {
    return new IrOptimizer([]);
  }
}
