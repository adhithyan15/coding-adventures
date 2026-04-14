import { readFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { type ASTNode, GrammarParser } from "@coding-adventures/parser";
import { tokenizeNib } from "@coding-adventures/nib-lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const NIB_GRAMMAR_PATH = join(GRAMMARS_DIR, "nib.grammar");

export function parseNib(source: string): ASTNode {
  const tokens = tokenizeNib(source);
  const grammarText = readFileSync(NIB_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
