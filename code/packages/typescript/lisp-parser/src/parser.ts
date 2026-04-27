import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeLisp } from "@coding-adventures/lisp-lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const LISP_GRAMMAR_PATH = join(GRAMMARS_DIR, "lisp.grammar");

function loadGrammar() {
  return parseParserGrammar(readFileSync(LISP_GRAMMAR_PATH, "utf-8"));
}

export function createLispParser(source: string): GrammarParser {
  return new GrammarParser(tokenizeLisp(source), loadGrammar());
}

export function parseLisp(source: string): ASTNode {
  return createLispParser(source).parse();
}
