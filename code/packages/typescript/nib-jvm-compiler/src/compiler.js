import { IrOptimizer } from "@coding-adventures/ir-optimizer";
import { lowerIrToJvmClassFile, writeClassFile as backendWriteClassFile, } from "@coding-adventures/ir-to-jvm-class-file";
import { parseClassFile } from "@coding-adventures/jvm-class-file";
import { compileNib, releaseConfig, } from "@coding-adventures/nib-ir-compiler";
import { parseNib } from "@coding-adventures/nib-parser";
import { checkNib } from "@coding-adventures/nib-type-checker";
export class PackageError extends Error {
    stage;
    cause;
    constructor(stage, message, cause) {
        super(message);
        this.name = "PackageError";
        this.stage = stage;
        this.cause = cause;
    }
}
export class NibJvmCompiler {
    className;
    buildConfig;
    optimize;
    emitMainWrapper;
    constructor(options = {}) {
        this.className = options.className ?? "NibProgram";
        this.buildConfig = options.buildConfig;
        this.optimize = options.optimize ?? true;
        this.emitMainWrapper = options.emitMainWrapper ?? true;
    }
    compileSource(source, overrides = {}) {
        const className = overrides.className ?? this.className;
        const buildConfig = overrides.buildConfig ?? this.buildConfig ?? releaseConfig();
        const optimize = overrides.optimize ?? this.optimize;
        const emitMainWrapper = overrides.emitMainWrapper ?? this.emitMainWrapper;
        const ast = tryStage("parse", () => parseNib(source));
        const typeResult = tryStage("type-check", () => checkNib(ast));
        if (!typeResult.ok) {
            const diagnostics = typeResult.errors
                .map((error) => `Line ${error.line}, Col ${error.column}: ${error.message}`)
                .join("\n");
            throw new PackageError("type-check", diagnostics);
        }
        const rawIr = tryStage("ir-compile", () => compileNib(typeResult.typedAst, buildConfig).program);
        const optimizer = optimize ? IrOptimizer.defaultPasses() : IrOptimizer.noOp();
        const optimization = tryStage("optimize", () => optimizer.optimize(rawIr));
        const artifact = tryStage("lower-jvm", () => lowerIrToJvmClassFile(optimization.program, {
            className,
            emitMainWrapper,
        }));
        const parsedClass = tryStage("validate-class", () => parseClassFile(artifact.classBytes));
        return {
            source,
            className,
            ast,
            typedAst: typeResult.typedAst,
            rawIr,
            optimization,
            optimizedIr: optimization.program,
            artifact,
            parsedClass,
            classBytes: artifact.classBytes,
        };
    }
    writeClassFile(source, outputDir, overrides = {}) {
        const result = this.compileSource(source, overrides);
        const classFilePath = tryStage("write", () => backendWriteClassFile(result.artifact, outputDir));
        return { ...result, classFilePath };
    }
}
export function compileSource(source, overrides = {}) {
    return new NibJvmCompiler(overrides).compileSource(source, overrides);
}
export function packSource(source, overrides = {}) {
    return compileSource(source, overrides);
}
export function writeClassFile(source, outputDir, overrides = {}) {
    return new NibJvmCompiler(overrides).writeClassFile(source, outputDir, overrides);
}
function tryStage(stage, action) {
    try {
        return action();
    }
    catch (error) {
        if (error instanceof PackageError) {
            throw error;
        }
        const message = error instanceof Error ? error.message : String(error);
        throw new PackageError(stage, message, error);
    }
}
//# sourceMappingURL=compiler.js.map