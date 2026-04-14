import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeVerilog } from "@coding-adventures/verilog-lexer";

import { PARSER_GRAMMAR as PARSER_GRAMMAR_1995 } from "./_grammar_1995.js";
import { PARSER_GRAMMAR as PARSER_GRAMMAR_2001 } from "./_grammar_2001.js";
import { PARSER_GRAMMAR as PARSER_GRAMMAR_2005 } from "./_grammar_2005.js";

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
      return PARSER_GRAMMAR_1995;
    case "2001":
      return PARSER_GRAMMAR_2001;
    case "2005":
      return PARSER_GRAMMAR_2005;
    default:
      throw new Error("Unsupported Verilog grammar version");
  }
}

export function parseVerilog(source: string, version?: string): ASTNode {
  const tokens = tokenizeVerilog(source, { version });
  const parser = new GrammarParser(tokens, loadVerilogGrammar(version));
  return parser.parse();
}
