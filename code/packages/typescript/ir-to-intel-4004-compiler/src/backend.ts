import type { IrProgram } from "@coding-adventures/compiler-ir";
import { IrValidationError, IrValidator } from "@coding-adventures/intel-4004-ir-validator";

import { CodeGenerator } from "./codegen.js";

export class IrToIntel4004Compiler {
  readonly validator = new IrValidator();
  readonly codegen = new CodeGenerator();

  compile(program: IrProgram): string {
    const errors = this.validator.validate(program);
    if (errors.length > 0) {
      throw new IrValidationError(
        errors.length > 1 ? "multiple" : errors[0].rule,
        errors.map((error) => error.toString()).join("\n"),
      );
    }
    return this.codegen.generate(program);
  }
}

export const Intel4004Backend = IrToIntel4004Compiler;
