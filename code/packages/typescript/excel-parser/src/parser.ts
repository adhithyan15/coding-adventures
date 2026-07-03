import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";
import { tokenizeExcelFormula } from "@coding-adventures/excel-lexer";

// The grammar is consumed from its pre-compiled, embedded form
// (`_grammar.ts`, auto-generated from `code/grammars/excel.grammar`)
// rather than read from disk at runtime.  This is what that generated file
// was made for — its own header says "Import it directly instead of reading
// and parsing the .grammar file at runtime."  Two payoffs:
//
//   1. No `fs`/`path`/`url` imports, so the parser bundles and runs in a
//      browser (this package advertises itself as "pure TypeScript" — it
//      now actually is).  The VisiCalc HTML demo bundles the spreadsheet
//      engine, which depends on this parser, straight to the browser.
//   2. One fewer parse step per call: the embedded value is already a
//      `ParserGrammar` object, so we skip re-parsing the `.grammar` text.
//
// The embedded grammar is verified structurally identical (order-insensitive)
// to `parseParserGrammar(readFileSync("excel.grammar"))`, so this swap is
// behaviour-preserving.  If `excel.grammar` ever changes, regenerate
// `_grammar.ts` with `grammar-tools compile-grammar excel.grammar`.
import { PARSER_GRAMMAR } from "./_grammar.js";

function previousSignificantToken(tokens: Token[], index: number): Token | null {
  for (let i = index - 1; i >= 0; i -= 1) {
    if (tokens[i].type !== "SPACE") {
      return tokens[i];
    }
  }
  return null;
}

function nextSignificantToken(tokens: Token[], index: number): Token | null {
  for (let i = index + 1; i < tokens.length; i += 1) {
    if (tokens[i].type !== "SPACE") {
      return tokens[i];
    }
  }
  return null;
}

function normalizeExcelReferenceTokens(tokens: Token[]): Token[] {
  return tokens.map((token, index) => {
    if (token.type !== "NAME" && token.type !== "NUMBER") {
      return token;
    }

    const previous = previousSignificantToken(tokens, index);
    const next = nextSignificantToken(tokens, index);
    const adjacentToColon = previous?.type === "COLON" || next?.type === "COLON";

    if (token.type === "NAME" && adjacentToColon) {
      return { ...token, type: "COLUMN_REF" };
    }

    if (token.type === "NUMBER" && adjacentToColon) {
      return { ...token, type: "ROW_REF" };
    }

    return token;
  });
}

export function parseExcelFormula(source: string): ASTNode {
  const tokens = tokenizeExcelFormula(source);
  const parser = new GrammarParser(tokens, PARSER_GRAMMAR);
  parser.addPreParse(normalizeExcelReferenceTokens);
  return parser.parse();
}
