import { parseBrainfuck } from "@coding-adventures/brainfuck";
import { type OptimizationResult } from "@coding-adventures/ir-optimizer";
import { type ValidatedModule } from "@coding-adventures/wasm-validator";
import type { IrProgram } from "@coding-adventures/compiler-ir";
import type { WasmModule } from "@coding-adventures/wasm-types";
export interface BrainfuckWasmCompilerOptions {
    readonly filename?: string;
    readonly optimize?: boolean;
}
export interface PackageResult {
    readonly source: string;
    readonly filename: string;
    readonly ast: ReturnType<typeof parseBrainfuck>;
    readonly rawIr: IrProgram;
    readonly optimization: OptimizationResult;
    readonly optimizedIr: IrProgram;
    readonly module: WasmModule;
    readonly validatedModule: ValidatedModule;
    readonly binary: Uint8Array;
    readonly wasmPath?: string;
}
export declare class PackageError extends Error {
    readonly stage: string;
    readonly cause?: unknown;
    constructor(stage: string, message: string, cause?: unknown);
}
export declare class BrainfuckWasmCompiler {
    private readonly filename;
    private readonly optimizer;
    constructor(options?: BrainfuckWasmCompilerOptions);
    compileSource(source: string): PackageResult;
    writeWasmFile(source: string, outputPath: string): PackageResult;
}
export declare function compileSource(source: string): PackageResult;
export declare function packSource(source: string): PackageResult;
export declare function writeWasmFile(source: string, outputPath: string): PackageResult;
//# sourceMappingURL=compiler.d.ts.map