/**
 * Parser — Layer 3 of the computing stack.
 *
 * Builds abstract syntax trees from token streams. This package provides two
 * parsers:
 *
 * 1. **Hand-written Parser** (``Parser``): Uses recursive descent with specific
 *    AST node types (``NumberLiteral``, ``BinaryOp``, etc.). Great for learning
 *    and for cases where you want typed AST nodes.
 *
 * 2. **Grammar-driven Parser** (``GrammarParser``): Reads grammar rules from a
 *    ``.grammar`` file (via ``grammar-tools``) and produces generic ``ASTNode``
 *    trees. Language-agnostic — swap the grammar file to parse a different
 *    language.
 *
 * Usage (hand-written parser):
 *
 *     import { Parser } from "@coding-adventures/parser";
 *     import type { Token } from "@coding-adventures/lexer";
 *
 *     const tokens: Token[] = [
 *       { type: "NUMBER", value: "42", line: 1, column: 1 },
 *       { type: "EOF", value: "", line: 1, column: 3 },
 *     ];
 *     const parser = new Parser(tokens);
 *     const ast = parser.parse();  // Returns a Program node
 *
 * Usage (grammar-driven parser):
 *
 *     import { parseParserGrammar } from "@coding-adventures/grammar-tools";
 *     import { tokenize } from "@coding-adventures/lexer";
 *     import { GrammarParser } from "@coding-adventures/parser";
 *
 *     const grammar = parseParserGrammar(fs.readFileSync("python.grammar", "utf-8"));
 *     const tokens = tokenize("x = 1 + 2");
 *     const parser = new GrammarParser(tokens, grammar);
 *     const ast = parser.parse();  // Returns a generic ASTNode tree
 *
 * AST Node Types (hand-written parser):
 *     NumberLiteral  — A numeric literal (e.g., 42)
 *     StringLiteral  — A string literal (e.g., "hello")
 *     Name           — A variable reference (e.g., x)
 *     BinaryOp       — A binary operation (e.g., 1 + 2)
 *     Assignment     — A variable assignment (e.g., x = 42)
 *     Program        — The root node containing all statements
 *
 * AST Node Types (grammar-driven parser):
 *     ASTNode        — A generic node with ruleName and children
 */

// Hand-written parser (specific AST nodes)
export type {
  NumberLiteral,
  StringLiteral,
  Name,
  BinaryOp,
  Assignment,
  Program,
  Expression,
  Statement,
} from "./parser.js";
export { Parser, ParseError } from "./parser.js";

// Grammar-driven parser (generic AST nodes)
export type { ASTNode, GrammarParserOptions } from "./grammar-parser.js";
export {
  GrammarParser,
  GrammarParseError,
  isASTNode,
  isLeafNode,
  getLeafToken,
} from "./grammar-parser.js";
