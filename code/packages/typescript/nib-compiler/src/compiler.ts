import { writeFile } from "node:fs/promises";

import type { IrProgram } from "@coding-adventures/compiler-ir";
import { assemble, AssemblerError } from "@coding-adventures/intel-4004-assembler";
import { decodeHex, encodeHex } from "@coding-adventures/intel-4004-packager";
import { IrOptimizer, type OptimizationResult } from "@coding-adventures/ir-optimizer";
import { IrToIntel4004Compiler } from "@coding-adventures/ir-to-intel-4004-compiler";
import type { BuildConfig } from "@coding-adventures/nib-ir-compiler";
import { compileNib, releaseConfig } from "@coding-adventures/nib-ir-compiler";
import { parseNib } from "@coding-adventures/nib-parser";
import { checkNib } from "@coding-adventures/nib-type-checker";
import type { ASTNode } from "@coding-adventures/parser";

export interface PackageResult {
  readonly source: string;
  readonly ast: ASTNode;
  readonly typedAst: ASTNode;
  readonly rawIr: IrProgram;
  readonly optimization: OptimizationResult;
  readonly optimizedIr: IrProgram;
  readonly assembly: string;
  readonly binary: Uint8Array;
  readonly hexText: string;
  readonly origin: number;
  readonly hexPath?: string;
}

export class PackageError extends Error {
  constructor(
    readonly stage: string,
    readonly message: string,
    readonly cause?: unknown,
  ) {
    super(message);
  }

  override toString(): string {
    return `[${this.stage}] ${this.message}`;
  }
}

export class NibCompiler {
  private readonly backend = new IrToIntel4004Compiler();

  constructor(
    private readonly options: {
      readonly buildConfig?: BuildConfig;
      readonly optimizeIr?: boolean;
    } = {},
  ) {}

  compileSource(
    source: string,
    options: {
      readonly origin?: number;
      readonly buildConfig?: BuildConfig;
      readonly optimizeIr?: boolean;
    } = {},
  ): PackageResult {
    const origin = options.origin ?? 0;
    const buildConfig = options.buildConfig ?? this.options.buildConfig ?? releaseConfig();
    const optimizeIr = options.optimizeIr ?? this.options.optimizeIr ?? true;

    let ast: ASTNode;
    try {
      ast = parseNib(source);
    } catch (error) {
      throw new PackageError("parse", error instanceof Error ? error.message : String(error), error);
    }

    const typeResult = checkNib(ast);
    if (!typeResult.ok) {
      const diagnostics = typeResult.errors
        .map((error) => `Line ${error.line}, Col ${error.column}: ${error.message}`)
        .join("\n");
      throw new PackageError("type-check", diagnostics);
    }

    let rawIr: IrProgram;
    try {
      rawIr = compileNib(typeResult.typedAst, buildConfig).program;
    } catch (error) {
      throw new PackageError("ir-compile", error instanceof Error ? error.message : String(error), error);
    }

    const optimizer = optimizeIr ? IrOptimizer.defaultPasses() : IrOptimizer.noOp();
    const optimization = optimizer.optimize(rawIr);

    let assembly: string;
    try {
      assembly = this.backend.compile(optimization.program);
    } catch (error) {
      throw new PackageError("validate", error instanceof Error ? error.message : String(error), error);
    }

    let binary: Uint8Array;
    try {
      binary = assemble(assembly);
    } catch (error) {
      const message = error instanceof AssemblerError || error instanceof Error ? error.message : String(error);
      throw new PackageError("assemble", message, error);
    }

    let hexText: string;
    try {
      hexText = encodeHex(binary, origin);
    } catch (error) {
      throw new PackageError("package", error instanceof Error ? error.message : String(error), error);
    }

    return {
      source,
      ast,
      typedAst: typeResult.typedAst,
      rawIr,
      optimization,
      optimizedIr: optimization.program,
      assembly,
      binary,
      hexText,
      origin,
    };
  }

  async writeHexFile(
    source: string,
    outputPath: string,
    options: {
      readonly origin?: number;
      readonly buildConfig?: BuildConfig;
      readonly optimizeIr?: boolean;
    } = {},
  ): Promise<PackageResult> {
    const result = this.compileSource(source, options);
    try {
      await writeFile(outputPath, result.hexText, "utf8");
    } catch (error) {
      throw new PackageError("write", error instanceof Error ? error.message : String(error), error);
    }
    return { ...result, hexPath: outputPath };
  }
}

export function compileSource(
  source: string,
  options: {
    readonly origin?: number;
    readonly buildConfig?: BuildConfig;
    readonly optimizeIr?: boolean;
  } = {},
): PackageResult {
  return new NibCompiler().compileSource(source, options);
}

export const packSource = compileSource;

export async function writeHexFile(
  source: string,
  outputPath: string,
  options: {
    readonly origin?: number;
    readonly buildConfig?: BuildConfig;
    readonly optimizeIr?: boolean;
  } = {},
): Promise<PackageResult> {
  return new NibCompiler().writeHexFile(source, outputPath, options);
}

export { decodeHex };
