import type { IrProgram } from "@coding-adventures/compiler-ir";
import type { ASTNode, Token } from "@coding-adventures/parser";
import { GrammarParser } from "@coding-adventures/parser";
import { grammarTokenize } from "@coding-adventures/lexer";
import { assemble, AssemblerError } from "@coding-adventures/intel-4004-assembler";
import { decodeHex, encodeHex } from "@coding-adventures/intel-4004-packager";
import { IrOptimizer } from "@coding-adventures/ir-optimizer";
import { IrToIntel4004Compiler } from "@coding-adventures/ir-to-intel-4004-compiler";
import type { BuildConfig } from "@coding-adventures/nib-ir-compiler";
import { compileNib, releaseConfig } from "@coding-adventures/nib-ir-compiler";
import { checkNib } from "@coding-adventures/nib-type-checker";
import { PARSER_GRAMMAR } from "./generated/nib-grammar.js";
import { TOKEN_GRAMMAR } from "./generated/nib-tokens.js";

export interface WebCompileResult {
  readonly source: string;
  readonly optimizedIr: IrProgram;
  readonly assembly: string;
  readonly binary: Uint8Array;
  readonly hexText: string;
  readonly origin: number;
}

export class WebPipelineError extends Error {
  constructor(
    readonly stage: string,
    message: string,
    readonly cause?: unknown,
  ) {
    super(message);
  }
}

const backend = new IrToIntel4004Compiler();

function tokenizeNib(source: string): Token[] {
  return grammarTokenize(source, TOKEN_GRAMMAR).map((token) => {
    if (token.type === "KEYWORD") {
      return { ...token, type: token.value };
    }
    return token;
  });
}

function parseNib(source: string): ASTNode {
  return new GrammarParser(tokenizeNib(source), PARSER_GRAMMAR).parse();
}

export function compileNibToHex(
  source: string,
  options: {
    readonly origin?: number;
    readonly buildConfig?: BuildConfig;
    readonly optimizeIr?: boolean;
  } = {},
): WebCompileResult {
  const origin = options.origin ?? 0;
  const buildConfig = options.buildConfig ?? releaseConfig();
  const optimizeIr = options.optimizeIr ?? true;

  let ast;
  try {
    ast = parseNib(source);
  } catch (error) {
    throw new WebPipelineError("parse", error instanceof Error ? error.message : String(error), error);
  }

  const typeResult = checkNib(ast);
  if (!typeResult.ok) {
    const diagnostics = typeResult.errors
      .map((diagnostic) => `Line ${diagnostic.line}, Col ${diagnostic.column}: ${diagnostic.message}`)
      .join("\n");
    throw new WebPipelineError("type-check", diagnostics);
  }

  let rawIr: IrProgram;
  try {
    rawIr = compileNib(typeResult.typedAst, buildConfig).program;
  } catch (error) {
    throw new WebPipelineError("ir-compile", error instanceof Error ? error.message : String(error), error);
  }

  const optimization = (optimizeIr ? IrOptimizer.defaultPasses() : IrOptimizer.noOp()).optimize(rawIr);

  let assembly: string;
  try {
    assembly = backend.compile(optimization.program);
  } catch (error) {
    throw new WebPipelineError("validate", error instanceof Error ? error.message : String(error), error);
  }

  let binary: Uint8Array;
  try {
    binary = assemble(assembly);
  } catch (error) {
    const message = error instanceof AssemblerError || error instanceof Error ? error.message : String(error);
    throw new WebPipelineError("assemble", message, error);
  }

  let hexText: string;
  try {
    hexText = encodeHex(binary, origin);
  } catch (error) {
    throw new WebPipelineError("package", error instanceof Error ? error.message : String(error), error);
  }

  return {
    source,
    optimizedIr: optimization.program,
    assembly,
    binary,
    hexText,
    origin,
  };
}

export function loadHexForSimulation(hexText: string): Uint8Array {
  return decodeHex(hexText).binary;
}
