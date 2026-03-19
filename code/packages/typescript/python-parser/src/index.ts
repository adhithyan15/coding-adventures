/**
 * Python Parser — parses Python source code into ASTs using the grammar-driven approach.
 *
 * This package demonstrates the power of the grammar-driven parser: by simply
 * providing a different `.grammar` file, the same parser engine that parses
 * one language can parse Python. No new parser code needed — just a new grammar.
 *
 * Usage:
 *
 *     import { parsePython } from "@coding-adventures/python-parser";
 *
 *     const ast = parsePython("x = 1 + 2");
 *     console.log(ast.ruleName); // "program"
 */

export { parsePython } from "./parser.js";
