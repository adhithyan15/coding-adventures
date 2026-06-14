#!/usr/bin/env node
import { fileURLToPath } from "node:url";

import { ParseErrors, Parser, SpecError } from "@coding-adventures/cli-builder";
import type { ParseResult } from "@coding-adventures/cli-builder";

import { compileGrammarFile } from "./compiler.js";
import type { GrammarSourceKind } from "./compiler.js";

const PROGRAM_NAME = "grammar-tools";
const SPEC_PATH = fileURLToPath(new URL("./grammar-tools.cli.json", import.meta.url));

export async function main(argv: readonly string[] = process.argv.slice(2)): Promise<number> {
  let parsed;
  try {
    parsed = new Parser(SPEC_PATH, [PROGRAM_NAME, ...argv]).parse();
  } catch (error) {
    process.stderr.write(`${PROGRAM_NAME}: ${formatError(error)}\n`);
    return 2;
  }

  if ("text" in parsed) {
    process.stdout.write(`${parsed.text}\n`);
    return 0;
  }

  if ("version" in parsed && !("flags" in parsed)) {
    process.stdout.write(`${parsed.version}\n`);
    return 0;
  }

  return compileFromParseResult(parsed as ParseResult);
}

async function compileFromParseResult(parsed: ParseResult): Promise<number> {
  const command = parsed.commandPath[parsed.commandPath.length - 1];
  const source = String(parsed.arguments["source"]);
  const output = parsed.flags["output"];
  const outputPath = output === null || output === undefined ? undefined : String(output);

  try {
    const kind = compileKind(command, parsed.flags["kind"]);
    const code = await compileGrammarFile(source, { kind, outputPath });
    if (outputPath === undefined) {
      process.stdout.write(code);
    }
    return 0;
  } catch (error) {
    process.stderr.write(`${PROGRAM_NAME}: ${formatError(error)}\n`);
    return 1;
  }
}

function compileKind(command: string, kindFlag: unknown): GrammarSourceKind | undefined {
  if (command === "compile-tokens") return "tokens";
  if (command === "compile-grammar") return "grammar";
  if (kindFlag === null || kindFlag === undefined) return undefined;
  return String(kindFlag) as GrammarSourceKind;
}

function formatError(error: unknown): string {
  if (error instanceof ParseErrors || error instanceof SpecError || error instanceof Error) {
    return error.message;
  }
  return String(error);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  process.exitCode = await main();
}
