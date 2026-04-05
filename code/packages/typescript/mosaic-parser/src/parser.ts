/**
 * Mosaic Parser — parses Mosaic token streams into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from
 * `@coding-adventures/parser`. All parsing logic lives in the generic engine;
 * this module provides the Mosaic parser grammar and wires the lexer and parser
 * together into a single `parseMosaic` function.
 *
 * The Two-Stage Pipeline
 * ----------------------
 *
 * Parsing a Mosaic file happens in two sequential stages:
 *
 *   1. **Lexing** — `tokenizeMosaic` converts raw source text into a flat
 *      array of typed tokens. It handles string recognition, keyword
 *      reclassification (`NAME` → `KEYWORD`), skip patterns (comments and
 *      whitespace), and position tracking (line/column for error messages).
 *
 *   2. **Parsing** — `GrammarParser` takes the token stream and the
 *      `PARSER_GRAMMAR` (imported from `_grammar.ts`) and applies recursive
 *      descent with full backtracking to produce an AST. The grammar rules
 *      mirror the EBNF grammar from `mosaic.grammar`.
 *
 * The AST Structure
 * -----------------
 *
 * Each node is an `ASTNode` with:
 *   - `ruleName` — the grammar rule that produced this node (e.g., `"file"`,
 *     `"component_decl"`, `"slot_decl"`, `"node_element"`, `"when_block"`)
 *   - `children` — an array of child `ASTNode`s or leaf `Token`s
 *
 * For a simple component:
 *
 *     component Label {
 *       slot text: text;
 *       Text { content: @text; }
 *     }
 *
 * The AST looks like:
 *
 *     ASTNode("file", [
 *       ASTNode("component_decl", [
 *         Token(KEYWORD, "component"),
 *         Token(NAME, "Label"),
 *         Token(LBRACE, "{"),
 *         ASTNode("slot_decl", [
 *           Token(KEYWORD, "slot"),
 *           Token(NAME, "text"),
 *           Token(COLON, ":"),
 *           ASTNode("slot_type", [Token(KEYWORD, "text")]),
 *           Token(SEMICOLON, ";"),
 *         ]),
 *         ASTNode("node_tree", [
 *           ASTNode("node_element", [
 *             Token(NAME, "Text"),
 *             Token(LBRACE, "{"),
 *             ASTNode("node_content", [
 *               ASTNode("property_assignment", [
 *                 Token(NAME, "content"),
 *                 Token(COLON, ":"),
 *                 ASTNode("property_value", [
 *                   ASTNode("slot_ref", [Token(AT, "@"), Token(NAME, "text")])
 *                 ]),
 *                 Token(SEMICOLON, ";"),
 *               ])
 *             ]),
 *             Token(RBRACE, "}"),
 *           ])
 *         ]),
 *         Token(RBRACE, "}"),
 *       ])
 *     ])
 *
 * Grammar Rules
 * -------------
 *
 * The full grammar (from `mosaic.grammar`) has 18 rules:
 *
 *   | Rule               | Description                                         |
 *   |--------------------|-----------------------------------------------------|
 *   | file               | Entry point: { import_decl } component_decl        |
 *   | import_decl        | import NAME [as NAME] from STRING ;                 |
 *   | component_decl     | component NAME { slot_decl* node_tree }             |
 *   | slot_decl          | slot NAME : slot_type [= default_value] ;           |
 *   | slot_type          | KEYWORD \| NAME \| list_type                        |
 *   | list_type          | list < slot_type >                                  |
 *   | default_value      | STRING \| NUMBER \| DIMENSION \| COLOR_HEX \| KEYWORD |
 *   | node_tree          | node_element (root element)                         |
 *   | node_element       | NAME { node_content* }                              |
 *   | node_content       | property_assignment \| child_node \| slot_reference \| when_block \| each_block |
 *   | property_assignment| NAME : property_value ;                             |
 *   | property_value     | slot_ref \| STRING \| DIMENSION \| NUMBER \| COLOR_HEX \| KEYWORD \| enum_value \| NAME |
 *   | slot_ref           | @ NAME                                              |
 *   | enum_value         | NAME . NAME                                         |
 *   | child_node         | node_element (syntactic alias)                      |
 *   | slot_reference     | @ NAME ;                                            |
 *   | when_block         | when slot_ref { node_content* }                     |
 *   | each_block         | each slot_ref as NAME { node_content* }             |
 *
 * Ambiguity Resolution
 * --------------------
 *
 * Two ambiguities exist in the grammar that the parser resolves via
 * backtracking:
 *
 *   1. **property_assignment vs child_node** — Both start with `NAME`.
 *      `property_assignment` has `NAME COLON`, `child_node` has `NAME LBRACE`.
 *      The parser tries `property_assignment` first; on failure, tries `child_node`.
 *
 *   2. **enum_value vs bare NAME** — `enum_value` is `NAME DOT NAME`, bare
 *      `NAME` is just `NAME`. In `property_value`, `enum_value` appears before
 *      `NAME` in the alternation so the longer form is attempted first.
 */

import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeMosaic } from "@coding-adventures/mosaic-lexer";
import { PARSER_GRAMMAR } from "./_grammar.js";

/**
 * Parse Mosaic source text and return an AST rooted at the `file` rule.
 *
 * This function orchestrates the full parsing pipeline:
 *   1. Tokenize the source using `tokenizeMosaic`
 *   2. Run the grammar-driven parser with `PARSER_GRAMMAR`
 *   3. Return the root `ASTNode` (rule name: `"file"`)
 *
 * @param source - The `.mosaic` source text to parse.
 * @returns An `ASTNode` with `ruleName` of `"file"`.
 * @throws ParseError if the source does not match the Mosaic grammar.
 *
 * @example
 *     const ast = parseMosaic(`
 *       component Label {
 *         slot text: text;
 *         Text { content: @text; }
 *       }
 *     `);
 *     console.log(ast.ruleName); // "file"
 *
 * @example
 *     // Component with a list slot:
 *     const ast = parseMosaic(`
 *       component List {
 *         slot items: list<text>;
 *         Column {
 *           each @items as item {
 *             Text { content: @item; }
 *           }
 *         }
 *       }
 *     `);
 *
 * @example
 *     // Component with an import:
 *     const ast = parseMosaic(`
 *       import Button from "./button.mosaic";
 *       component Card {
 *         slot action: Button;
 *         Row { @action; }
 *       }
 *     `);
 */
export function parseMosaic(source: string): ASTNode {
  /**
   * Step 1: Tokenize.
   * The mosaic-lexer handles keyword reclassification, skip patterns (comments
   * and whitespace), position tracking, and all lexical details.
   */
  const tokens = tokenizeMosaic(source);

  /**
   * Step 2: Parse.
   * The GrammarParser takes the token array and grammar rules, then performs
   * recursive descent with backtracking to produce an AST. The starting rule
   * is the first rule in the grammar — `"file"`.
   */
  const parser = new GrammarParser(tokens, PARSER_GRAMMAR);
  return parser.parse();
}
