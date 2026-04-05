/**
 * JavaScript Parser — parses JavaScript source code into ASTs using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { parseJavascript } from "@coding-adventures/javascript-parser";
 *
 *     // Generic (backwards-compatible with v0.1.x)
 *     const ast = parseJavascript("let x = 1 + 2;");
 *     console.log(ast.ruleName); // "program"
 *
 *     // Version-specific grammar
 *     const ast = parseJavascript("var x = 1 + 2;", "es5");
 */

export { parseJavascript } from "./parser.js";
