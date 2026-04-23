import type { TokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize, GrammarLexer } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import { TOKEN_GRAMMAR as TOKEN_GRAMMAR_1987 } from "./_grammar_1987.js";
import { TOKEN_GRAMMAR as TOKEN_GRAMMAR_1993 } from "./_grammar_1993.js";
import { TOKEN_GRAMMAR as TOKEN_GRAMMAR_2002 } from "./_grammar_2002.js";
import { TOKEN_GRAMMAR as TOKEN_GRAMMAR_2008 } from "./_grammar_2008.js";
import { TOKEN_GRAMMAR as TOKEN_GRAMMAR_2019 } from "./_grammar_2019.js";

export const DEFAULT_VERSION = "2008";

const SUPPORTED_VERSIONS = new Set(["1987", "1993", "2002", "2008", "2019"]);

function resolveVersion(version?: string): string {
  if (!version) {
    return DEFAULT_VERSION;
  }
  if (!SUPPORTED_VERSIONS.has(version)) {
    throw new Error(
      `Unknown VHDL version "${version}". Valid values: 1987, 1993, 2002, 2008, 2019`
    );
  }
  return version;
}

function loadVhdlGrammar(version?: string): TokenGrammar {
  switch (resolveVersion(version)) {
    case "1987":
      return TOKEN_GRAMMAR_1987;
    case "1993":
      return TOKEN_GRAMMAR_1993;
    case "2002":
      return TOKEN_GRAMMAR_2002;
    case "2008":
      return TOKEN_GRAMMAR_2008;
    case "2019":
      return TOKEN_GRAMMAR_2019;
    default:
      throw new Error("Unsupported VHDL grammar version");
  }
}

function normalizeCasing(tokens: Token[], keywordSet: ReadonlySet<string>): Token[] {
  return tokens.map((token) => {
    if (token.type === "NAME" || token.type === "KEYWORD") {
      const lowered = token.value.toLowerCase();
      const type = keywordSet.has(lowered) ? "KEYWORD" : "NAME";
      return {
        type,
        value: lowered,
        line: token.line,
        column: token.column,
      };
    }
    return token;
  });
}

export function tokenizeVhdl(source: string, version?: string): Token[] {
  const grammar = loadVhdlGrammar(version);
  return normalizeCasing(grammarTokenize(source, grammar), new Set(grammar.keywords));
}

export function createVhdlLexer(source: string, version?: string): GrammarLexer {
  return new GrammarLexer(source, loadVhdlGrammar(version));
}
