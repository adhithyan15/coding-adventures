import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize, GrammarLexer } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const LISP_TOKENS_PATH = join(GRAMMARS_DIR, "lisp.tokens");

function loadGrammar() {
  return parseTokenGrammar(readFileSync(LISP_TOKENS_PATH, "utf-8"));
}

export function createLispLexer(source: string): GrammarLexer {
  return new GrammarLexer(source, loadGrammar());
}

export function tokenizeLisp(source: string): Token[] {
  return grammarTokenize(source, loadGrammar());
}
