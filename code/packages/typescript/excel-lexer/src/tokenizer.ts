import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { GrammarLexer, LexerContext } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const EXCEL_TOKENS_PATH = join(GRAMMARS_DIR, "excel.tokens");

function loadExcelTokenGrammar() {
  const grammarText = readFileSync(EXCEL_TOKENS_PATH, "utf-8");
  return parseTokenGrammar(grammarText);
}

function nextNonSpaceChar(ctx: LexerContext): string {
  let offset = 1;
  for (;;) {
    const ch = ctx.peek(offset);
    if (ch === "" || ch !== " ") {
      return ch;
    }
    offset += 1;
  }
}

export function excelOnToken(token: Token, ctx: LexerContext): void {
  if (token.type !== "NAME") {
    return;
  }

  const nextChar = nextNonSpaceChar(ctx);
  if (nextChar === "(") {
    ctx.suppress();
    ctx.emit({ ...token, type: "FUNCTION_NAME" });
    return;
  }

  if (nextChar === "[") {
    ctx.suppress();
    ctx.emit({ ...token, type: "TABLE_NAME" });
  }
}

export function createExcelLexer(source: string): GrammarLexer {
  const grammar = loadExcelTokenGrammar();
  const lexer = new GrammarLexer(source, grammar);
  lexer.setOnToken(excelOnToken);
  return lexer;
}

export function tokenizeExcelFormula(source: string): Token[] {
  return createExcelLexer(source).tokenize();
}
