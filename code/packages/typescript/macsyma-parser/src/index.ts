import { tokenizeMacsyma } from "@coding-adventures/macsyma-lexer";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";

import { PARSER_GRAMMAR } from "./generated/parser-grammar.js";

export { PARSER_GRAMMAR };
export type { ASTNode };

export function createMacsymaParser(source: string): GrammarParser {
  return new GrammarParser(tokenizeMacsyma(source), PARSER_GRAMMAR);
}

export function parseMacsyma(source: string): ASTNode {
  return createMacsymaParser(source).parse();
}
