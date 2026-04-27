import { dirname } from "node:path";
import { mkdirSync, writeFileSync } from "node:fs";

import type { Token } from "@coding-adventures/lexer";
import type { ASTNode } from "@coding-adventures/parser";
import type { IrProgram } from "@coding-adventures/compiler-ir";
import { IrOptimizer, type OptimizationResult } from "@coding-adventures/ir-optimizer";
import {
  IrToWasmCompiler,
  type FunctionSignature,
} from "@coding-adventures/ir-to-wasm-compiler";
import { validate as validateLowering } from "@coding-adventures/ir-to-wasm-validator";
import { compileNib, releaseConfig, type BuildConfig } from "@coding-adventures/nib-ir-compiler";
import { parseNib } from "@coding-adventures/nib-parser";
import { checkNib } from "@coding-adventures/nib-type-checker";
import { encodeModule } from "@coding-adventures/wasm-module-encoder";
import type { WasmModule } from "@coding-adventures/wasm-types";
import { validate, type ValidatedModule } from "@coding-adventures/wasm-validator";

export interface NibWasmCompilerOptions {
  readonly buildConfig?: BuildConfig;
  readonly optimize?: boolean;
}

export interface PackageResult {
  readonly source: string;
  readonly ast: ASTNode;
  readonly typedAst: ASTNode;
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

export class NibWasmCompiler {
  private readonly buildConfig?: BuildConfig;
  private readonly optimizer: IrOptimizer;

  constructor(options: NibWasmCompilerOptions = {}) {
    this.buildConfig = options.buildConfig;
    this.optimizer = options.optimize === false
      ? IrOptimizer.noOp()
      : IrOptimizer.defaultPasses();
  }

  compileSource(source: string): PackageResult {
    const ast = tryStage("parse", () => parseNib(source));

    const typeResult = checkNib(ast);
    if (!typeResult.ok) {
      const diagnostics = typeResult.errors
        .map((error) => `Line ${error.line}, Col ${error.column}: ${error.message}`)
        .join("\n");
      throw new PackageError("type-check", diagnostics);
    }

    const rawIr = tryStage("ir", () => {
      const result = compileNib(typeResult.typedAst, this.buildConfig ?? releaseConfig());
      return result.program;
    });
    const optimization = tryStage("optimize", () => this.optimizer.optimize(rawIr));
    const optimizedIr = optimization.program;
    const signatures = extractSignatures(typeResult.typedAst);

    tryStage("lowering-validate", () => {
      const errors = validateLowering(optimizedIr, signatures);
      if (errors.length > 0) {
        throw new Error(errors.map((error) => error.message).join("; "));
      }
    });

    const module = tryStage("lower", () => new IrToWasmCompiler().compile(optimizedIr, signatures));
    const validatedModule = tryStage("validate", () => validate(module));
    const binary = tryStage("encode", () => encodeModule(module));

    return {
      source,
      ast,
      typedAst: typeResult.typedAst,
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
      mkdirSync(dirname(outputPath), { recursive: true });
      writeFileSync(outputPath, result.binary);
    });
    return {
      ...result,
      wasmPath: outputPath,
    };
  }
}

export function compileSource(source: string): PackageResult {
  return new NibWasmCompiler().compileSource(source);
}

export function packSource(source: string): PackageResult {
  return compileSource(source);
}

export function writeWasmFile(source: string, outputPath: string): PackageResult {
  return new NibWasmCompiler().writeWasmFile(source, outputPath);
}

export function extractSignatures(ast: ASTNode): FunctionSignature[] {
  const signatures: FunctionSignature[] = [{ label: "_start", paramCount: 0, exportName: "_start" }];

  for (const topDecl of childNodes(ast)) {
    const decl = unwrapTopDecl(topDecl);
    if (!decl || decl.ruleName !== "fn_decl") {
      continue;
    }
    const name = firstName(decl);
    if (!name) {
      continue;
    }
    signatures.push({
      label: `_fn_${name}`,
      paramCount: countParams(decl),
      exportName: name,
    });
  }

  return signatures;
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

function isAstNode(child: ASTNode | Token): child is ASTNode {
  return "ruleName" in child;
}

function childNodes(node: ASTNode): ASTNode[] {
  return node.children.filter(isAstNode);
}

function unwrapTopDecl(node: ASTNode): ASTNode | null {
  return childNodes(node)[0] ?? null;
}

function firstName(node: ASTNode): string | null {
  for (const child of node.children) {
    if (isAstNode(child)) {
      const nested = firstName(child);
      if (nested) {
        return nested;
      }
      continue;
    }
    if (child.type === "NAME") {
      return child.value;
    }
  }
  return null;
}

function countParams(fnDecl: ASTNode): number {
  const paramList = childNodes(fnDecl).find((node) => node.ruleName === "param_list");
  if (!paramList) {
    return 0;
  }
  return childNodes(paramList).filter((node) => node.ruleName === "param").length;
}
