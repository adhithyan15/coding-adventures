/**
 * ALGOL 60 Parser -- parses ALGOL 60 source text into ASTs using the grammar-driven approach.
 *
 * ALGOL 60 (ALGOrithmic Language, 1960) was the first programming language with a
 * formally specified grammar. This parser produces abstract syntax trees (ASTs) from
 * ALGOL 60 source text, suitable for analysis, interpretation, or compilation.
 *
 * Usage:
 *
 *     import { parseAlgol } from "@coding-adventures/algol-parser";
 *
 *     const ast = parseAlgol("begin integer x; x := 42 end");
 *     console.log(ast.ruleName); // "program"
 */

export { parseAlgol } from "./parser.js";
