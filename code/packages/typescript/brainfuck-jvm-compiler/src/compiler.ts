import type { IrProgram } from "@coding-adventures/compiler-ir";
import { parseBrainfuck } from "@coding-adventures/brainfuck/src/parser.js";
import { compile, releaseConfig, type BuildConfig } from "@coding-adventures/brainfuck-ir-compiler";
import { IrOptimizer, type OptimizationResult } from "@coding-adventures/ir-optimizer";
import {
  lowerIrToJvmClassFile,
  writeClassFile as backendWriteClassFile,
  type JVMClassArtifact,
  type JvmBackendConfig,
} from "@coding-adventures/ir-to-jvm-class-file";
import { parseClassFile, type JVMClassFile } from "@coding-adventures/jvm-class-file";

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

export class BrainfuckJvmCompiler {
  private readonly filename: string;
  private readonly className: string;
  private readonly buildConfig?: BuildConfig;
  private readonly optimize: boolean;
  private readonly emitMainWrapper: boolean;

  constructor(options: BrainfuckJvmCompilerOptions = {}) {
    this.filename = options.filename ?? "program.bf";
    this.className = options.className ?? "BrainfuckProgram";
    this.buildConfig = options.buildConfig;
    this.optimize = options.optimize ?? true;
    this.emitMainWrapper = options.emitMainWrapper ?? true;
  }

  compileSource(source: string, overrides: BrainfuckJvmCompilerOptions = {}): PackageResult {
    const filename = overrides.filename ?? this.filename;
    const className = overrides.className ?? this.className;
    const buildConfig = overrides.buildConfig ?? this.buildConfig ?? releaseConfig();
    const optimize = overrides.optimize ?? this.optimize;
    const emitMainWrapper = overrides.emitMainWrapper ?? this.emitMainWrapper;

    const ast = tryStage("parse", () => parseBrainfuck(source));
    const rawIr = tryStage("ir-compile", () => compile(ast, filename, buildConfig).program);
    const optimizer = optimize ? IrOptimizer.defaultPasses() : IrOptimizer.noOp();
    const optimization = tryStage("optimize", () => optimizer.optimize(rawIr));
    const artifact = tryStage("lower-jvm", () =>
      lowerIrToJvmClassFile(optimization.program, {
        className,
        emitMainWrapper,
      } satisfies JvmBackendConfig));
    const parsedClass = tryStage("validate-class", () => parseClassFile(artifact.classBytes));

    return {
      source,
      filename,
      className,
      ast,
      rawIr,
      optimization,
      optimizedIr: optimization.program,
      artifact,
      parsedClass,
      classBytes: artifact.classBytes,
    };
  }

  writeClassFile(
    source: string,
    outputDir: string,
    overrides: BrainfuckJvmCompilerOptions = {},
  ): PackageResult {
    const result = this.compileSource(source, overrides);
    const classFilePath = tryStage("write", () => backendWriteClassFile(result.artifact, outputDir));
    return { ...result, classFilePath };
  }
}

export function compileSource(
  source: string,
  overrides: BrainfuckJvmCompilerOptions = {},
): PackageResult {
  return new BrainfuckJvmCompiler(overrides).compileSource(source, overrides);
}

export function packSource(
  source: string,
  overrides: BrainfuckJvmCompilerOptions = {},
): PackageResult {
  return compileSource(source, overrides);
}

export function writeClassFile(
  source: string,
  outputDir: string,
  overrides: BrainfuckJvmCompilerOptions = {},
): PackageResult {
  return new BrainfuckJvmCompiler(overrides).writeClassFile(source, outputDir, overrides);
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
