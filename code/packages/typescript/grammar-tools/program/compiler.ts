import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, extname } from "node:path";

import {
  compileParserGrammar,
  compileTokenGrammar,
  parseParserGrammar,
  parseTokenGrammar,
} from "../src/index.js";

export type GrammarSourceKind = "tokens" | "grammar";

export interface CompileGrammarSourceOptions {
  readonly kind?: GrammarSourceKind;
  readonly sourcePath?: string;
}

export interface CompileGrammarFileOptions extends CompileGrammarSourceOptions {
  readonly outputPath?: string;
}

export function inferGrammarSourceKind(sourcePath: string): GrammarSourceKind {
  const extension = extname(sourcePath);
  if (extension === ".tokens") return "tokens";
  if (extension === ".grammar") return "grammar";
  throw new Error(`Cannot infer grammar kind from ${sourcePath}; pass --kind tokens or --kind grammar`);
}

export function compileGrammarSource(
  source: string,
  options: CompileGrammarSourceOptions = {}
): string {
  const kind = options.kind ?? inferGrammarSourceKind(requiredSourcePath(options.sourcePath));
  const sourcePath = options.sourcePath ?? "";

  if (kind === "tokens") {
    return compileTokenGrammar(parseTokenGrammar(source), sourcePath);
  }
  return compileParserGrammar(parseParserGrammar(source), sourcePath);
}

export async function compileGrammarFile(
  inputPath: string,
  options: CompileGrammarFileOptions = {}
): Promise<string> {
  const source = await readFile(inputPath, "utf8");
  const code = compileGrammarSource(source, {
    kind: options.kind,
    sourcePath: options.sourcePath ?? inputPath,
  });

  if (options.outputPath !== undefined) {
    await mkdir(dirname(options.outputPath), { recursive: true });
    await writeFile(options.outputPath, code, "utf8");
  }

  return code;
}

function requiredSourcePath(sourcePath: string | undefined): string {
  if (sourcePath === undefined || sourcePath.length === 0) {
    throw new Error("Grammar kind is required when no sourcePath is provided");
  }
  return sourcePath;
}
