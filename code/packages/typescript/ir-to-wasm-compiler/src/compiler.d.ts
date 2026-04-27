import { type IrProgram } from "@coding-adventures/compiler-ir";
import { WasmModule } from "@coding-adventures/wasm-types";
export declare class WasmLoweringError extends Error {
    constructor(message: string);
}
export interface FunctionSignature {
    readonly label: string;
    readonly paramCount: number;
    readonly exportName?: string;
}
export declare class IrToWasmCompiler {
    compile(program: IrProgram, functionSignatures?: readonly FunctionSignature[]): WasmModule;
    private buildTypeTable;
    private layoutData;
    private needsMemory;
    private needsWasiScratch;
    private collectWasiImports;
    private splitFunctions;
}
export declare function inferFunctionSignaturesFromComments(program: IrProgram): Map<string, FunctionSignature>;
//# sourceMappingURL=compiler.d.ts.map