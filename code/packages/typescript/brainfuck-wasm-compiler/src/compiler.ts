import { writeFileSync } from "node:fs";

import { parseBrainfuck } from "@coding-adventures/brainfuck/src/parser.js";
import { compile, releaseConfig } from "@coding-adventures/brainfuck-ir-compiler";
import { IrOptimizer, type OptimizationResult } from "@coding-adventures/ir-optimizer";
import { IrToWasmCompiler } from "@coding-adventures/ir-to-wasm-compiler";
import { validate as validateLowering } from "@coding-adventures/ir-to-wasm-validator";
import { encodeModule } from "@coding-adventures/wasm-module-encoder";
import { validate, type ValidatedModule } from "@coding-adventures/wasm-validator";
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

export class PackageError extends Error {
  readonly stage: string;
  readonly cause?: unknown;

  constructor(stage: string, message: string, cause?: unknown) {
    super(message);
    this.name = "PackageError";
    this.stage = stage;
    this.cause = cause;
  }
}

export class BrainfuckWasmCompiler {
  private readonly filename: string;
  private readonly optimizer: IrOptimizer;

  constructor(options: BrainfuckWasmCompilerOptions = {}) {
    this.filename = options.filename ?? "program.bf";
    this.optimizer = options.optimize === false
      ? IrOptimizer.noOp()
      : IrOptimizer.defaultPasses();
  }

  compileSource(source: string): PackageResult {
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

  writeWasmFile(source: string, outputPath: string): PackageResult {
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

export function compileSource(source: string): PackageResult {
  return new BrainfuckWasmCompiler().compileSource(source);
}

export function packSource(source: string): PackageResult {
  return compileSource(source);
}

export function writeWasmFile(source: string, outputPath: string): PackageResult {
  return new BrainfuckWasmCompiler().writeWasmFile(source, outputPath);
}

function tryStage<T>(stage: string, action: () => T): T {
  try {
    return action();
  } catch (error) {
    if (error instanceof PackageError) {
      throw error;
    }
    const message = error instanceof Error ? error.message : String(error);
    throw new PackageError(stage, message, error);
  }
}
