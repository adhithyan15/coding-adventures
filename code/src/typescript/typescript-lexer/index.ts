/**
 * TypeScript Lexer — tokenizes TypeScript source code using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { tokenizeTypescript, createTypescriptLexer } from "@coding-adventures/typescript-lexer";
 *
 *     // Generic (backwards-compatible with v0.1.x)
 *     const tokens = tokenizeTypescript("let x: number = 1 + 2;");
 *
 *     // Version-specific grammar
 *     const tokens = tokenizeTypescript("let x: number = 1 + 2;", "ts5.8");
 *
 *     // Class-based lexer with on-token callbacks
 *     const lexer = createTypescriptLexer("let x = 1;", "ts5.0");
 *     const tokens = lexer.tokenize();
 */

export { tokenizeTypescript, createTypescriptLexer } from "./tokenizer.js";
