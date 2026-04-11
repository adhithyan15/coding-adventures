/**
 * Dartmouth BASIC Parser -- parses 1964 BASIC source text into ASTs.
 *
 * Dartmouth BASIC was the first programming language designed for
 * non-science students. Created by Kemeny and Kurtz at Dartmouth College
 * in 1964, it ran on a GE-225 mainframe accessed via uppercase teletypes.
 * This parser produces abstract syntax trees from BASIC source text,
 * suitable for compilation, interpretation, or analysis.
 *
 * Usage:
 *
 *     import { parseDartmouthBasic } from "@coding-adventures/dartmouth-basic-parser";
 *
 *     const ast = parseDartmouthBasic("10 LET X = 5\n20 PRINT X\n30 END\n");
 *     console.log(ast.ruleName); // "program"
 */

export { parseDartmouthBasic } from "./parser.js";
