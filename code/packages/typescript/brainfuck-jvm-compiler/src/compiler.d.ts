import type { IrProgram } from "@coding-adventures/compiler-ir";
import { parseBrainfuck } from "@coding-adventures/brainfuck/src/parser.js";
import { type BuildConfig } from "@coding-adventures/brainfuck-ir-compiler";
import { type OptimizationResult } from "@coding-adventures/ir-optimizer";
import { type JVMClassArtifact } from "@coding-adventures/ir-to-jvm-class-file";
import { type JVMClassFile } from "@coding-adventures/jvm-class-file";
export interface BrainfuckJvmCompilerOptions {
    readonly filename?: string;
    readonly className?: string;
    readonly buildConfig?: BuildConfig;
    readonly optimize?: boolean;
    readonly emitMainWrapper?: boolean;
}
export interface PackageResult {
    readonly source: string;
    readonly filename: string;
    readonly className: string;
    readonly ast: ReturnType<typeof parseBrainfuck>;
    readonly rawIr: IrProgram;
    readonly optimization: OptimizationResult;
    readonly optimizedIr: IrProgram;
    readonly artifact: JVMClassArtifact;
    readonly parsedClass: JVMClassFile;
    readonly classBytes: Uint8Array;
    readonly classFilePath?: string;
}
export declare class PackageError extends Error {
    readonly stage: string;
    readonly cause?: unknown;
    constructor(stage: string, message: string, cause?: unknown);
}
export declare class BrainfuckJvmCompiler {
    private readonly filename;
    private readonly className;
    private readonly buildConfig?;
    private readonly optimize;
    private readonly emitMainWrapper;
    constructor(options?: BrainfuckJvmCompilerOptions);
    compileSource(source: string, overrides?: BrainfuckJvmCompilerOptions): PackageResult;
    writeClassFile(source: string, outputDir: string, overrides?: BrainfuckJvmCompilerOptions): PackageResult;
}
export declare function compileSource(source: string, overrides?: BrainfuckJvmCompilerOptions): PackageResult;
export declare function packSource(source: string, overrides?: BrainfuckJvmCompilerOptions): PackageResult;
export declare function writeClassFile(source: string, outputDir: string, overrides?: BrainfuckJvmCompilerOptions): PackageResult;
//# sourceMappingURL=compiler.d.ts.map