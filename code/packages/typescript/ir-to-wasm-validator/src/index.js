import { IrToWasmCompiler, WasmLoweringError, } from "@coding-adventures/ir-to-wasm-compiler";
export const VERSION = "0.1.0";
export class WasmIrValidator {
    validate(program, functionSignatures = []) {
        try {
            new IrToWasmCompiler().compile(program, functionSignatures);
        }
        catch (error) {
            if (error instanceof WasmLoweringError) {
                return [{ rule: "lowering", message: error.message }];
            }
            throw error;
        }
        return [];
    }
}
export function validate(program, functionSignatures = []) {
    return new WasmIrValidator().validate(program, functionSignatures);
}
//# sourceMappingURL=index.js.map