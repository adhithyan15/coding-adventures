/**
 * Haskell Parser — parses Haskell source code into ASTs using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { parseHaskell, createHaskellParser } from "@coding-adventures/haskell-parser";
 *
 *     // Default version (Haskell 21)
 *     const ast = parseHaskell("class Hello { }");
 *     console.log(ast.ruleName); // "program"
 *
 *     // Version-specific grammar
 *     const ast = parseHaskell("int x = 1 + 2;", "8");
 *
 *     // Factory function for more control
 *     const parser = createHaskellParser("int x = 42;", "21");
 *     const ast = parser.parse();
 */

export { parseHaskell, createHaskellParser } from "./parser.js";
