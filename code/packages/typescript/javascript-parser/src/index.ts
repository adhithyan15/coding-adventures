/**
 * JavaScript Parser — parses JavaScript source code into ASTs using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { parseJavascript } from "@coding-adventures/javascript-parser";
 *
 *     const ast = parseJavascript("let x = 1 + 2;");
 *     console.log(ast.ruleName); // "program"
 */

export { parseJavascript } from "./parser.js";
