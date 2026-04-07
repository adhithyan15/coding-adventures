/**
 * ALGOL 60 Lexer -- tokenizes ALGOL 60 source text using the grammar-driven approach.
 *
 * ALGOL 60 (ALGOrithmic Language, 1960) is the common ancestor of Pascal, C,
 * Simula (first OOP language), Ada, and virtually every modern programming language.
 * It was the first language whose grammar was formally specified using BNF notation.
 *
 * This lexer produces a flat stream of tokens from ALGOL 60 source text, suitable
 * for feeding into the algol-parser.
 *
 * Usage:
 *
 *     import { tokenizeAlgol } from "@coding-adventures/algol-lexer";
 *
 *     const tokens = tokenizeAlgol("begin integer x; x := 42 end");
 */

export { tokenizeAlgol } from "./tokenizer.js";
