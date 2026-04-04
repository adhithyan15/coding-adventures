/**
 * Mosaic Parser — parses `.mosaic` source into an ASTNode tree.
 *
 * Usage:
 *
 *     import { parseMosaic } from "@coding-adventures/mosaic-parser";
 *
 *     const ast = parseMosaic('component Label { slot text: text; Text { content: @text; } }');
 *     console.log(ast.ruleName); // "file"
 */

export { parseMosaic } from "./parser.js";
export { PARSER_GRAMMAR } from "./_grammar.js";
export type { ASTNode } from "@coding-adventures/parser";
