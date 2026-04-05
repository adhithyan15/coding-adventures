/**
 * ECMAScript 1 (1997) Parser — parses ES1 source code into ASTs using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { parseEs1 } from "@coding-adventures/ecmascript-es1-parser";
 *
 *     const ast = parseEs1("var x = 1 + 2;");
 *     console.log(ast.ruleName); // "program"
 */

export { parseEs1 } from "./parser.js";
