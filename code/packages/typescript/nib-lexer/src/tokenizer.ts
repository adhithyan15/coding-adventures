import { readFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize, type Token } from "@coding-adventures/lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const NIB_TOKENS_PATH = join(GRAMMARS_DIR, "nib.tokens");

export interface TokenizeNibOptions {
  readonly preserveSourceInfo?: boolean;
}

export function tokenizeNib(
  source: string,
  options: TokenizeNibOptions = {},
): Token[] {
  const grammarText = readFileSync(NIB_TOKENS_PATH, "utf-8");
  const grammar = parseTokenGrammar(grammarText);

  return grammarTokenize(source, grammar, {
    preserveSourceInfo: options.preserveSourceInfo,
  }).map((token) => {
    if (token.type === "KEYWORD") {
      return { ...token, type: token.value };
    }
    return token;
  });
}
