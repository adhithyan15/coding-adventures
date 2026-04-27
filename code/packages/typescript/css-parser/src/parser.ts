import { readFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

import { tokenizeCss } from "@coding-adventures/css-lexer";
import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const CSS_GRAMMAR_PATH = join(GRAMMARS_DIR, "css.grammar");

export function createCssParser(source: string): GrammarParser {
  const tokens = tokenizeCss(source);
  const grammarText = readFileSync(CSS_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  return new GrammarParser(tokens, grammar);
}

export function parseCss(source: string): ASTNode {
  return createCssParser(source).parse();
}

export const parseCSS = parseCss;
