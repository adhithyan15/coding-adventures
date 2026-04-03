import { GrammarLexer, LexerContext } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";
import { TOKEN_GRAMMAR } from "./_grammar.js";

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
  const lexer = new GrammarLexer(source, TOKEN_GRAMMAR);
  lexer.setOnToken(excelOnToken);
  return lexer;
}

export function tokenizeExcelFormula(source: string): Token[] {
  return createExcelLexer(source).tokenize();
}
