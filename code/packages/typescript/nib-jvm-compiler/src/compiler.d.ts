import type { IrProgram } from "@coding-adventures/compiler-ir";
import { type OptimizationResult } from "@coding-adventures/ir-optimizer";
import { type JVMClassArtifact } from "@coding-adventures/ir-to-jvm-class-file";
import { type JVMClassFile } from "@coding-adventures/jvm-class-file";
import { type BuildConfig } from "@coding-adventures/nib-ir-compiler";
import type { ASTNode } from "@coding-adventures/parser";
export interface NibJvmCompilerOptions {
    readonly className?: string;
    readonly buildConfig?: BuildConfig;
    readonly optimize?: boolean;
    readonly emitMainWrapper?: boolean;
}
export interface PackageResult {
    readonly source: string;
    readonly className: string;
    readonly ast: ASTNode;
    readonly typedAst: ASTNode;
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
export declare class NibJvmCompiler {
    private readonly className;
    private readonly buildConfig?;
    private readonly optimize;
    private readonly emitMainWrapper;
    constructor(options?: NibJvmCompilerOptions);
    compileSource(source: string, overrides?: NibJvmCompilerOptions): PackageResult;
    writeClassFile(source: string, outputDir: string, overrides?: NibJvmCompilerOptions): PackageResult;
}
export declare function compileSource(source: string, overrides?: NibJvmCompilerOptions): PackageResult;
export declare function packSource(source: string, overrides?: NibJvmCompilerOptions): PackageResult;
export declare function writeClassFile(source: string, outputDir: string, overrides?: NibJvmCompilerOptions): PackageResult;
//# sourceMappingURL=compiler.d.ts.map