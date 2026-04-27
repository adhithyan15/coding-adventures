import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeVhdl } from "@coding-adventures/vhdl-lexer";

import { PARSER_GRAMMAR as PARSER_GRAMMAR_1987 } from "./_grammar_1987.js";
import { PARSER_GRAMMAR as PARSER_GRAMMAR_1993 } from "./_grammar_1993.js";
import { PARSER_GRAMMAR as PARSER_GRAMMAR_2002 } from "./_grammar_2002.js";
import { PARSER_GRAMMAR as PARSER_GRAMMAR_2008 } from "./_grammar_2008.js";
import { PARSER_GRAMMAR as PARSER_GRAMMAR_2019 } from "./_grammar_2019.js";

export const DEFAULT_VERSION = "2008";

const SUPPORTED_VERSIONS = new Set(["1987", "1993", "2002", "2008", "2019"]);

function resolveVersion(version?: string): string {
  if (!version) {
    return DEFAULT_VERSION;
  }
  if (!SUPPORTED_VERSIONS.has(version)) {
    throw new Error(
      `Unknown VHDL version "${version}". Valid values: 1987, 1993, 2002, 2008, 2019`
    );
  }
  return version;
}

function loadVhdlGrammar(version?: string) {
  switch (resolveVersion(version)) {
    case "1987":
      return PARSER_GRAMMAR_1987;
    case "1993":
      return PARSER_GRAMMAR_1993;
    case "2002":
      return PARSER_GRAMMAR_2002;
    case "2008":
      return PARSER_GRAMMAR_2008;
    case "2019":
      return PARSER_GRAMMAR_2019;
    default:
      throw new Error("Unsupported VHDL grammar version");
  }
}

export function parseVhdl(source: string, version?: string): ASTNode {
  const tokens = tokenizeVhdl(source, version);
  const parser = new GrammarParser(tokens, loadVhdlGrammar(version));
  return parser.parse();
}
