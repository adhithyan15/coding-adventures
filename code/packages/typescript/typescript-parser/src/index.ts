/**
 * TypeScript Parser — parses TypeScript source code into ASTs using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { parseTypescript } from "@coding-adventures/typescript-parser";
 *
 *     const ast = parseTypescript("let x = 1 + 2;");
 *     console.log(ast.ruleName); // "program"
 */

export { parseTypescript } from "./parser.js";
