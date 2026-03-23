/**
 * Ruby Parser — parses Ruby source code into ASTs using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { parseRuby } from "@coding-adventures/ruby-parser";
 *
 *     const ast = parseRuby("x = 1 + 2");
 *     console.log(ast.ruleName); // "program"
 */

export { parseRuby } from "./parser.js";
