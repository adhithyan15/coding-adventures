import { writeFileSync } from "node:fs";
import { parseBrainfuck } from "@coding-adventures/brainfuck";
import { compile, releaseConfig } from "@coding-adventures/brainfuck-ir-compiler";
import { IrOptimizer } from "@coding-adventures/ir-optimizer";
import { IrToWasmCompiler } from "@coding-adventures/ir-to-wasm-compiler";
import { validate as validateLowering } from "@coding-adventures/ir-to-wasm-validator";
import { encodeModule } from "@coding-adventures/wasm-module-encoder";
import { validate } from "@coding-adventures/wasm-validator";
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
export class BrainfuckWasmCompiler {
    filename;
    optimizer;
    constructor(options = {}) {
        this.filename = options.filename ?? "program.bf";
        this.optimizer = options.optimize === false
            ? IrOptimizer.noOp()
            : IrOptimizer.defaultPasses();
    }
    compileSource(source) {
        const ast = tryStage("parse", () => parseBrainfuck(source));
        const rawIr = tryStage("ir", () => {
            const result = compile(ast, this.filename, releaseConfig());
            return result.program;
        });
        const optimization = tryStage("optimize", () => this.optimizer.optimize(rawIr));
        const optimizedIr = optimization.program;
        tryStage("lowering-validate", () => {
            const errors = validateLowering(optimizedIr);
            if (errors.length > 0) {
                throw new Error(errors.map((error) => error.message).join("; "));
            }
        });
        const module = tryStage("lower", () => new IrToWasmCompiler().compile(optimizedIr));
        const validatedModule = tryStage("validate", () => validate(module));
        const binary = tryStage("encode", () => encodeModule(module));
        return {
            source,
            filename: this.filename,
            ast,
            rawIr,
            optimization,
            optimizedIr,
            module,
            validatedModule,
            binary,
        };
    }
    writeWasmFile(source, outputPath) {
        const result = this.compileSource(source);
        tryStage("write", () => {
            writeFileSync(outputPath, result.binary);
        });
        return {
            ...result,
            wasmPath: outputPath,
        };
    }
}
export function compileSource(source) {
    return new BrainfuckWasmCompiler().compileSource(source);
}
export function packSource(source) {
    return compileSource(source);
}
export function writeWasmFile(source, outputPath) {
    return new BrainfuckWasmCompiler().writeWasmFile(source, outputPath);
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