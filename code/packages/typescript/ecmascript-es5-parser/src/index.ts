/**
 * ECMAScript 5 (2009) Parser — parses ES5 source code into ASTs using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { parseEs5 } from "@coding-adventures/ecmascript-es5-parser";
 *
 *     const ast = parseEs5("debugger;");
 *     console.log(ast.ruleName); // "program"
 */

export { parseEs5 } from "./parser.js";
