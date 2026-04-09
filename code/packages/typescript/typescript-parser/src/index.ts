/**
 * TypeScript Parser — parses TypeScript source code into ASTs using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { parseTypescript } from "@coding-adventures/typescript-parser";
 *
 *     // Generic (backwards-compatible with v0.1.x)
 *     const ast = parseTypescript("let x = 1 + 2;");
 *     console.log(ast.ruleName); // "program"
 *
 *     // Version-specific grammar
 *     const ast = parseTypescript("let x: number = 1;", "ts5.8");
 */

export { parseTypescript } from "./parser.js";
