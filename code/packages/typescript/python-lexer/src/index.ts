/**
 * Python Lexer — tokenizes Python source code using the grammar-driven approach.
 *
 * This package demonstrates the power of the grammar-driven lexer: by simply
 * providing a different `.tokens` file, the same lexer engine that tokenizes
 * one language can tokenize Python. No new lexer code needed — just a new grammar.
 *
 * Usage:
 *
 *     import { tokenizePython } from "@coding-adventures/python-lexer";
 *
 *     const tokens = tokenizePython("x = 1 + 2");
 *     for (const token of tokens) {
 *       console.log(token);
 *     }
 */

export { tokenizePython } from "./tokenizer.js";
