import type { IrProgram } from "@coding-adventures/compiler-ir";
import { type FunctionSignature } from "@coding-adventures/ir-to-wasm-compiler";
export declare const VERSION = "0.1.0";
export interface ValidationError {
    readonly rule: string;
    readonly message: string;
}
export declare class WasmIrValidator {
    validate(program: IrProgram, functionSignatures?: readonly FunctionSignature[]): ValidationError[];
}
export declare function validate(program: IrProgram, functionSignatures?: readonly FunctionSignature[]): ValidationError[];
//# sourceMappingURL=index.d.ts.map