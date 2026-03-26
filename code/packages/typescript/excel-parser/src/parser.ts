import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";
import { tokenizeExcelFormula } from "@coding-adventures/excel-lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const EXCEL_GRAMMAR_PATH = join(GRAMMARS_DIR, "excel.grammar");

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
  const grammarText = readFileSync(EXCEL_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  parser.addPreParse(normalizeExcelReferenceTokens);
  return parser.parse();
}
