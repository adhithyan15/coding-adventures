/**
 * Lattice Parser — Parses Lattice source text into an AST.
 *
 * Lattice is a CSS superset language with variables, mixins, control flow,
 * functions, and modules. This parser produces a generic ASTNode tree from
 * Lattice source text using the grammar-driven approach.
 *
 * The Pipeline
 * ------------
 *
 * Lattice source → tokenize (lattice-lexer) → parse (GrammarParser) → ASTNode
 *
 * This module is a thin wrapper, exactly like json-parser but for Lattice:
 *
 * 1. Tokenize with the Lattice lexer (lattice.tokens grammar)
 * 2. Load the Lattice grammar rules (lattice.grammar)
 * 3. Parse with GrammarParser using recursive descent
 *
 * The AST Structure
 * -----------------
 *
 * The root node has ruleName "stylesheet". Its children are "rule" nodes,
 * which contain either CSS constructs or Lattice constructs:
 *
 *   ASTNode("stylesheet", [
 *     ASTNode("rule", [
 *       ASTNode("lattice_rule", [
 *         ASTNode("variable_declaration", [
 *           Token(VARIABLE, "$color"),
 *           Token(COLON, ":"),
 *           ASTNode("value_list", [
 *             ASTNode("value", [Token(IDENT, "red")])
 *           ]),
 *           Token(SEMICOLON, ";"),
 *         ])
 *       ])
 *     ]),
 *     ASTNode("rule", [
 *       ASTNode("qualified_rule", [
 *         ASTNode("selector_list", [...]),
 *         ASTNode("block", [...])
 *       ])
 *     ])
 *   ])
 *
 * Lattice-Specific Grammar Rules
 * --------------------------------
 *
 *   stylesheet           — { rule }
 *   rule                 — lattice_rule | at_rule | qualified_rule
 *   lattice_rule         — variable_declaration | mixin_definition | ...
 *   variable_declaration — VARIABLE COLON value_list SEMICOLON
 *   mixin_definition     — "@mixin" FUNCTION [ mixin_params ] RPAREN block
 *   include_directive    — "@include" FUNCTION include_args RPAREN ...
 *   if_directive         — "@if" lattice_expression block { @else if ... } [ @else ... ]
 *   for_directive        — "@for" VARIABLE "from" expr ("through"|"to") expr block
 *   each_directive       — "@each" VARIABLE "in" each_list block
 *   function_definition  — "@function" FUNCTION [ mixin_params ] RPAREN function_body
 *   return_directive     — "@return" lattice_expression SEMICOLON
 *   use_directive        — "@use" STRING [ "as" IDENT ] SEMICOLON
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The lattice.grammar file lives in code/grammars/ at the repository root.
 *
 *     src/index.ts
 *       → lattice-parser/
 *         → typescript/
 *           → packages/
 *             → code/
 *               → grammars/
 *                 → lattice.grammar
 *
 * Usage:
 *
 *     import { parseLattice } from "@coding-adventures/lattice-parser";
 *
 *     const ast = parseLattice("$color: red; h1 { color: $color; }");
 *     console.log(ast.ruleName); // "stylesheet"
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeLatticeLexer } from "@coding-adventures/lattice-lexer";

/**
 * Resolve __dirname for ESM modules.
 * In ESM, __dirname is not available — we derive it from import.meta.url.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Navigate from src/ up to the grammars/ directory.
 *
 * Traversal:
 *   __dirname  = .../lattice-parser/src/
 *   ..         = .../lattice-parser/
 *   ../..      = .../typescript/
 *   ../../..   = .../packages/
 *   ../../../.. = .../code/
 *   + grammars = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const LATTICE_GRAMMAR_PATH = join(GRAMMARS_DIR, "lattice.grammar");

/**
 * Create a GrammarParser configured for Lattice source text.
 *
 * This function:
 * 1. Tokenizes the source text using the Lattice lexer.
 * 2. Reads and parses the lattice.grammar file.
 * 3. Creates a GrammarParser with those tokens and grammar rules.
 *
 * Call .parse() on the returned parser to get the AST.
 *
 * @param source - The Lattice source text to parse.
 * @returns A GrammarParser instance. Call .parse() to get the AST.
 *
 * @example
 *     const parser = createLatticeParser("$color: red;");
 *     const ast = parser.parse();
 *     console.log(ast.ruleName); // "stylesheet"
 */
export function createLatticeParser(source: string): GrammarParser {
  // Step 1: Tokenize. The Lattice lexer handles all CSS tokens plus the
  // 5 Lattice extensions (VARIABLE, EQUALS_EQUALS, NOT_EQUALS, etc.).
  const tokens = tokenizeLatticeLexer(source);

  // Step 2: Load the grammar rules. The grammar file defines ~45 rules
  // in EBNF-like notation: stylesheet, rule, qualified_rule, at_rule,
  // declaration, and all Lattice-specific rules (variable_declaration,
  // mixin_definition, if_directive, etc.).
  const grammarText = readFileSync(LATTICE_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);

  // Step 3: Create the grammar-driven parser. It will perform recursive
  // descent with backtracking (packrat memoization) to match the token
  // stream against the grammar rules.
  return new GrammarParser(tokens, grammar);
}

/**
 * Parse Lattice source text and return an AST.
 *
 * This is the main entry point for the Lattice parser. Pass in a string of
 * Lattice source, get back an ASTNode representing the complete parse tree.
 *
 * The returned AST has ruleName "stylesheet" at the root. Its children are
 * "rule" nodes, each containing either a Lattice construct (variable,
 * mixin, function, @use) or a CSS construct (qualified_rule, at_rule).
 *
 * The AST-to-CSS transformer (lattice-ast-to-css package) takes this AST
 * and expands all Lattice nodes into pure CSS nodes.
 *
 * @param source - The Lattice source text to parse.
 * @returns An ASTNode with ruleName "stylesheet".
 * @throws GrammarParseError if the source has syntax errors.
 * @throws LexerError if the source has lexical errors.
 *
 * @example
 *     const ast = parseLattice("$color: red;");
 *     console.log(ast.ruleName); // "stylesheet"
 *
 * @example
 *     const ast = parseLattice(`
 *       $primary: #4a90d9;
 *       h1 { color: $primary; }
 *     `);
 *
 * @example
 *     const ast = parseLattice(`
 *       @mixin button($bg) {
 *         background: $bg;
 *       }
 *       .btn { @include button(red); }
 *     `);
 */
export function parseLattice(source: string): ASTNode {
  const parser = createLatticeParser(source);
  return parser.parse();
}

// Re-export ASTNode type for consumers
export type { ASTNode };
