import { tokenizeCss } from "@coding-adventures/css-lexer";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";

import { PARSER_GRAMMAR } from "./_grammar.js";

export function createCssParser(source: string): GrammarParser {
  const tokens = tokenizeCss(source);
  return new GrammarParser(tokens, PARSER_GRAMMAR);
}

export function parseCss(source: string): ASTNode {
  return createCssParser(source).parse();
}

export const parseCSS = parseCss;
