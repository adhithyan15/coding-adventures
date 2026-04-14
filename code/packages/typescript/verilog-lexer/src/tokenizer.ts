import { grammarTokenize, GrammarLexer } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import { TOKEN_GRAMMAR as TOKEN_GRAMMAR_1995 } from "./_grammar_1995.js";
import { TOKEN_GRAMMAR as TOKEN_GRAMMAR_2001 } from "./_grammar_2001.js";
import { TOKEN_GRAMMAR as TOKEN_GRAMMAR_2005 } from "./_grammar_2005.js";
import { verilogPreprocess } from "./preprocessor.js";

export const DEFAULT_VERSION = "2005";

const SUPPORTED_VERSIONS = new Set(["1995", "2001", "2005"]);

function resolveVersion(version?: string): string {
  if (!version) {
    return DEFAULT_VERSION;
  }
  if (!SUPPORTED_VERSIONS.has(version)) {
    throw new Error(
      `Unknown Verilog version "${version}". Valid values: 1995, 2001, 2005`
    );
  }
  return version;
}

function loadVerilogGrammar(version?: string) {
  switch (resolveVersion(version)) {
    case "1995":
      return TOKEN_GRAMMAR_1995;
    case "2001":
      return TOKEN_GRAMMAR_2001;
    case "2005":
      return TOKEN_GRAMMAR_2005;
    default:
      throw new Error("Unsupported Verilog grammar version");
  }
}

export interface VerilogTokenizeOptions {
  preprocess?: boolean;
  version?: string;
}

export function tokenizeVerilog(
  source: string,
  options?: VerilogTokenizeOptions,
): Token[] {
  const processedSource =
    (options?.preprocess ?? true) ? verilogPreprocess(source) : source;
  return grammarTokenize(processedSource, loadVerilogGrammar(options?.version));
}

export function createVerilogLexer(
  source: string,
  options?: VerilogTokenizeOptions,
): GrammarLexer {
  const processedSource =
    (options?.preprocess ?? true) ? verilogPreprocess(source) : source;
  return new GrammarLexer(
    processedSource,
    loadVerilogGrammar(options?.version),
  );
}
