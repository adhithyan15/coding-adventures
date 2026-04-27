import { readFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { type ASTNode, GrammarParser } from "@coding-adventures/parser";
import { tokenizeNib, type TokenizeNibOptions } from "@coding-adventures/nib-lexer";
import type { Token } from "@coding-adventures/lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const NIB_GRAMMAR_PATH = join(GRAMMARS_DIR, "nib.grammar");

export interface ParseNibOptions extends TokenizeNibOptions {}

export interface ParsedNibDocument {
  readonly ast: ASTNode;
  readonly tokens: readonly Token[];
}

export function parseNib(source: string, options: ParseNibOptions = {}): ASTNode {
  return parseNibDocument(source, options).ast;
}

export function parseNibDocument(
  source: string,
  options: ParseNibOptions = {},
): ParsedNibDocument {
  const tokens = tokenizeNib(source, {
    preserveSourceInfo: options.preserveSourceInfo,
  });
  const grammarText = readFileSync(NIB_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar, {
    preserveSourceInfo: options.preserveSourceInfo,
  });

  return {
    ast: parser.parse(),
    tokens,
  };
}
