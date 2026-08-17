import { grammarTokenize, type Token } from "@coding-adventures/lexer";

import { TOKEN_GRAMMAR } from "./_grammar.js";

export interface TokenizeNibOptions {
  readonly preserveSourceInfo?: boolean;
}

export function tokenizeNib(
  source: string,
  options: TokenizeNibOptions = {},
): Token[] {
  return grammarTokenize(source, TOKEN_GRAMMAR, {
    preserveSourceInfo: options.preserveSourceInfo,
  }).map((token) => {
    if (token.type === "KEYWORD") {
      return { ...token, type: token.value };
    }
    return token;
  });
}
