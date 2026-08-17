import { grammarTokenize, GrammarLexer } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import { TOKEN_GRAMMAR } from "./_grammar.js";

export function createLispLexer(source: string): GrammarLexer {
  return new GrammarLexer(source, TOKEN_GRAMMAR);
}

export function tokenizeLisp(source: string): Token[] {
  return grammarTokenize(source, TOKEN_GRAMMAR);
}
