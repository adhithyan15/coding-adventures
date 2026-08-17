import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeLisp } from "@coding-adventures/lisp-lexer";

import { PARSER_GRAMMAR } from "./_grammar.js";

export function createLispParser(source: string): GrammarParser {
  return new GrammarParser(tokenizeLisp(source), PARSER_GRAMMAR);
}

export function parseLisp(source: string): ASTNode {
  return createLispParser(source).parse();
}
