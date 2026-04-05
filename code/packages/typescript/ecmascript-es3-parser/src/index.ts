/**
 * ECMAScript 3 (1999) Parser — parses ES3 source code into ASTs using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { parseEs3 } from "@coding-adventures/ecmascript-es3-parser";
 *
 *     const ast = parseEs3("try { x; } catch (e) { y; }");
 *     console.log(ast.ruleName); // "program"
 */

export { parseEs3 } from "./parser.js";
