/**
 * JavaScript Lexer — tokenizes JavaScript source code using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { tokenizeJavascript, createJavascriptLexer } from "@coding-adventures/javascript-lexer";
 *
 *     // Generic (backwards-compatible with v0.1.x)
 *     const tokens = tokenizeJavascript("let x = 1 + 2;");
 *
 *     // Version-specific grammar
 *     const tokens = tokenizeJavascript("var x = 1 + 2;", "es5");
 *     const tokens = tokenizeJavascript("let x = 1 + 2;", "es2015");
 *
 *     // Class-based lexer with on-token callbacks
 *     const lexer = createJavascriptLexer("let x = 1;", "es2015");
 *     const tokens = lexer.tokenize();
 */

export { tokenizeJavascript, createJavascriptLexer } from "./tokenizer.js";
