/**
 * Java Parser — parses Java source code into ASTs using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { parseJava, createJavaParser } from "@coding-adventures/java-parser";
 *
 *     // Default version (Java 21)
 *     const ast = parseJava("class Hello { }");
 *     console.log(ast.ruleName); // "program"
 *
 *     // Version-specific grammar
 *     const ast = parseJava("int x = 1 + 2;", "8");
 *
 *     // Factory function for more control
 *     const parser = createJavaParser("int x = 42;", "21");
 *     const ast = parser.parse();
 */

export { parseJava, createJavaParser } from "./parser.js";
