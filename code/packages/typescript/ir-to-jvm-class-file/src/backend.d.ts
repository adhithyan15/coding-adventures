import { type IrProgram } from "@coding-adventures/compiler-ir";
export declare class JvmBackendError extends Error {
    constructor(message: string);
}
export interface JvmBackendConfig {
    readonly className: string;
    readonly classFileMajor?: number;
    readonly classFileMinor?: number;
    readonly emitMainWrapper?: boolean;
}
export interface JVMClassArtifact {
    readonly className: string;
    readonly classBytes: Uint8Array;
    readonly callableLabels: readonly string[];
    readonly dataOffsets: ReadonlyMap<string, number>;
    readonly classFilename: string;
}
export declare function lowerIrToJvmClassFile(program: IrProgram, config: JvmBackendConfig): JVMClassArtifact;
export declare function writeClassFile(artifact: JVMClassArtifact, outputDir: string): string;
//# sourceMappingURL=backend.d.ts.map