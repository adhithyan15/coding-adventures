import type { IrProgram } from "@coding-adventures/compiler-ir";
import { IrOptimizer, type OptimizationResult } from "@coding-adventures/ir-optimizer";
import {
  lowerIrToJvmClassFile,
  writeClassFile as backendWriteClassFile,
  type JVMClassArtifact,
  type JvmBackendConfig,
} from "@coding-adventures/ir-to-jvm-class-file";
import { parseClassFile, type JVMClassFile } from "@coding-adventures/jvm-class-file";
import {
  compileNib,
  releaseConfig,
  type BuildConfig,
} from "@coding-adventures/nib-ir-compiler";
import { parseNib } from "@coding-adventures/nib-parser";
import { checkNib } from "@coding-adventures/nib-type-checker";
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

export class NibJvmCompiler {
  private readonly className: string;
  private readonly buildConfig?: BuildConfig;
  private readonly optimize: boolean;
  private readonly emitMainWrapper: boolean;

  constructor(options: NibJvmCompilerOptions = {}) {
    this.className = options.className ?? "NibProgram";
    this.buildConfig = options.buildConfig;
    this.optimize = options.optimize ?? true;
    this.emitMainWrapper = options.emitMainWrapper ?? true;
  }

  compileSource(source: string, overrides: NibJvmCompilerOptions = {}): PackageResult {
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
    const artifact = tryStage("lower-jvm", () =>
      lowerIrToJvmClassFile(optimization.program, {
        className,
        emitMainWrapper,
      } satisfies JvmBackendConfig));
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

  writeClassFile(source: string, outputDir: string, overrides: NibJvmCompilerOptions = {}): PackageResult {
    const result = this.compileSource(source, overrides);
    const classFilePath = tryStage("write", () => backendWriteClassFile(result.artifact, outputDir));
    return { ...result, classFilePath };
  }
}

export function compileSource(source: string, overrides: NibJvmCompilerOptions = {}): PackageResult {
  return new NibJvmCompiler(overrides).compileSource(source, overrides);
}

export function packSource(source: string, overrides: NibJvmCompilerOptions = {}): PackageResult {
  return compileSource(source, overrides);
}

export function writeClassFile(
  source: string,
  outputDir: string,
  overrides: NibJvmCompilerOptions = {},
): PackageResult {
  return new NibJvmCompiler(overrides).writeClassFile(source, outputDir, overrides);
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
