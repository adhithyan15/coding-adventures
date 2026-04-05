/**
 * Python Lexer — tokenizes Python source code using the grammar-driven approach.
 *
 * This package demonstrates the power of the grammar-driven lexer: by simply
 * providing a different `.tokens` file, the same lexer engine that tokenizes
 * one language can tokenize Python. No new lexer code needed — just a new grammar.
 *
 * Supports multiple Python versions (2.7, 3.0, 3.6, 3.8, 3.10, 3.12) with
 * version-specific grammar files. Defaults to Python 3.12.
 *
 * Usage:
 *
 *     import { tokenizePython } from "@coding-adventures/python-lexer";
 *
 *     const tokens = tokenizePython("x = 1 + 2");          // defaults to 3.12
 *     const tokens = tokenizePython("print x", "2.7");     // Python 2.7
 *     for (const token of tokens) {
 *       console.log(token);
 *     }
 */

export { tokenizePython, SUPPORTED_VERSIONS } from "./tokenizer.js";
