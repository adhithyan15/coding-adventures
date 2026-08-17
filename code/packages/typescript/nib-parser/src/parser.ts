import { type ASTNode, GrammarParser } from "@coding-adventures/parser";
import { tokenizeNib, type TokenizeNibOptions } from "@coding-adventures/nib-lexer";
import type { Token } from "@coding-adventures/lexer";

import { PARSER_GRAMMAR } from "./_grammar.js";

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
  const parser = new GrammarParser(tokens, PARSER_GRAMMAR, {
    preserveSourceInfo: options.preserveSourceInfo,
  });

  return {
    ast: parser.parse(),
    tokens,
  };
}
