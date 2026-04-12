/**
 * C# Parser — parses C# source code into ASTs using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { parseCSharp, createCSharpParser } from "@coding-adventures/csharp-parser";
 *
 *     // Default version (C# 12.0)
 *     const ast = parseCSharp("class Hello { }");
 *     console.log(ast.ruleName); // "program"
 *
 *     // Version-specific grammar
 *     const ast = parseCSharp("int x = 1 + 2;", "8.0");
 *
 *     // Factory function for more control
 *     const parser = createCSharpParser("int x = 42;", "12.0");
 *     const ast = parser.parse();
 */

export { parseCSharp, createCSharpParser } from "./parser.js";
