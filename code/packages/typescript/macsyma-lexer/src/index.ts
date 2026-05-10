import { GrammarLexer } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import { TOKEN_GRAMMAR } from "./generated/token-grammar.js";

export { TOKEN_GRAMMAR };
export type { Token };

export function createMacsymaLexer(source: string): GrammarLexer {
  return new GrammarLexer(source, TOKEN_GRAMMAR);
}

export function tokenizeMacsyma(source: string): Token[] {
  return createMacsymaLexer(source).tokenize();
}
