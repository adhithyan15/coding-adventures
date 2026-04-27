import type { IrProgram } from "@coding-adventures/compiler-ir";
import {
  IrToWasmCompiler,
  WasmLoweringError,
  type FunctionSignature,
} from "@coding-adventures/ir-to-wasm-compiler";

export const VERSION = "0.1.0";

export interface ValidationError {
  readonly rule: string;
  readonly message: string;
}

export class WasmIrValidator {
  validate(
    program: IrProgram,
    functionSignatures: readonly FunctionSignature[] = [],
  ): ValidationError[] {
    try {
      new IrToWasmCompiler().compile(program, functionSignatures);
    } catch (error) {
      if (error instanceof WasmLoweringError) {
        return [{ rule: "lowering", message: error.message }];
      }
      throw error;
    }
    return [];
  }
}

export function validate(
  program: IrProgram,
  functionSignatures: readonly FunctionSignature[] = [],
): ValidationError[] {
  return new WasmIrValidator().validate(program, functionSignatures);
}
